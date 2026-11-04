//! One launcher at a time. The first launcher to start creates a local endpoint: a Unix socket in its config folder on
//! macOS and Linux, or a named pipe on Windows. A launcher started while it runs writes to that endpoint, which asks the
//! first launcher to show its window, and then exits. Creating the endpoint is the claim, and the operating system lets
//! only one launcher create it.

use std::sync::Mutex;

use iced::Subscription;
use iced::futures::{SinkExt, Stream};
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};
use tracing::{info, warn};

/// The requests to show the window, sent by the listener. The subscription takes the receiving end once.
static REQUESTS: Mutex<Option<UnboundedReceiver<()>>> = Mutex::new(None);

/// What creating the endpoint found.
enum Claim {
    /// Another launcher holds the endpoint, and has been asked to show its window.
    Taken,
    /// This launcher holds the endpoint, and its listener is running.
    Held,
}

/// Makes this launcher the one that runs, or finds that another one does. When another launcher holds the endpoint, it is
/// asked to show its window, and `true` is returned: this launcher should exit. Otherwise this launcher listens for the
/// launchers that come after it, and `false` is returned. A launcher that cannot tell runs without the check.
pub fn hand_over() -> bool {
    let Some(endpoint) = platform::endpoint() else {
        return false;
    };
    let (sender, receiver) = unbounded_channel();
    match platform::claim(&endpoint, sender) {
        Ok(Claim::Taken) => {
            info!("Another launcher is running, and has been asked to show its window");
            true
        }
        Ok(Claim::Held) => {
            if let Ok(mut slot) = REQUESTS.lock() {
                *slot = Some(receiver);
            }
            false
        }
        Err(error) => {
            warn!("Could not check for another launcher, so this one runs alone: {error}");
            false
        }
    }
}

/// The requests to show the window, one for each launcher that started while this one ran.
pub fn focus_requests() -> Subscription<()> {
    Subscription::run(requests)
}

fn requests() -> impl Stream<Item = ()> {
    iced::stream::channel(4, async move |mut output| {
        let taken = REQUESTS.lock().ok().and_then(|mut slot| slot.take());
        // No listener belongs to this launcher when another one holds the endpoint, so there is nothing to wait for.
        let Some(mut receiver) = taken else {
            return;
        };
        while receiver.recv().await.is_some() {
            if output.send(()).await.is_err() {
                break;
            }
        }
    })
}

/// Unix: a socket in the config folder. Binding it is the claim, and a socket that nothing answers is one a launcher left
/// behind when it ended.
#[cfg(unix)]
mod platform {
    use std::io::{self, Write};
    use std::os::unix::net::{UnixListener, UnixStream};
    use std::path::{Path, PathBuf};
    use std::thread;

    use tokio::sync::mpsc::UnboundedSender;
    use tracing::warn;

    use super::Claim;

    /// The socket's path, in the config folder.
    pub fn endpoint() -> Option<PathBuf> {
        crate::app::paths::config_dir().map(|dir| dir.join("launcher.sock"))
    }

    pub fn claim(path: &Path, requests: UnboundedSender<()>) -> io::Result<Claim> {
        if let Ok(mut stream) = UnixStream::connect(path) {
            // The message is only a line to read, so a failed write still leaves the other launcher to be shown.
            let _ = stream.write_all(b"show\n");
            return Ok(Claim::Taken);
        }
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let _ = std::fs::remove_file(path);
        let listener = UnixListener::bind(path)?;
        thread::spawn(move || {
            for connection in listener.incoming() {
                match connection {
                    Ok(_) => {
                        if requests.send(()).is_err() {
                            break;
                        }
                    }
                    Err(error) => warn!("A launcher's request to show the window could not be read: {error}"),
                }
            }
        });
        Ok(Claim::Held)
    }
}

/// Windows: a named pipe whose name the config folder's path makes unique. Creating the first instance of the pipe is the
/// claim.
#[cfg(windows)]
mod platform {
    use std::fs::OpenOptions;
    use std::hash::{DefaultHasher, Hash, Hasher};
    use std::io::{self, Write};
    use std::thread;

    use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
    use tokio::runtime::Builder;
    use tokio::sync::mpsc::UnboundedSender;
    use tracing::warn;

    use super::Claim;

    /// Windows' error for a pipe whose instances are all in use, which only a running launcher's pipe is.
    const ERROR_PIPE_BUSY: i32 = 231;
    /// Windows' error for a pipe that another process created first.
    const ERROR_ACCESS_DENIED: i32 = 5;

    /// The pipe's name, which the config folder's path makes unique to this launcher's data.
    pub fn endpoint() -> Option<String> {
        let dir = crate::app::paths::config_dir()?;
        let mut hasher = DefaultHasher::new();
        dir.hash(&mut hasher);
        Some(format!(r"\.\pipe\lodestone-launcher-{:016x}", hasher.finish()))
    }

    pub fn claim(name: &str, requests: UnboundedSender<()>) -> io::Result<Claim> {
        match OpenOptions::new().write(true).open(name) {
            Ok(mut pipe) => {
                // The message is only a line to read, so a failed write still leaves the other launcher to be shown.
                let _ = pipe.write_all(b"show\n");
                return Ok(Claim::Taken);
            }
            Err(error) if error.raw_os_error() == Some(ERROR_PIPE_BUSY) => return Ok(Claim::Taken),
            Err(_) => {}
        }
        let runtime = Builder::new_current_thread().enable_all().build()?;
        let first = {
            let _entered = runtime.enter();
            ServerOptions::new().first_pipe_instance(true).create(name)
        };
        let first = match first {
            Ok(first) => first,
            // Another launcher created the pipe between the two steps, so it holds the claim.
            Err(error) if error.raw_os_error() == Some(ERROR_ACCESS_DENIED) => return Ok(Claim::Taken),
            Err(error) => return Err(error),
        };
        let name = name.to_string();
        thread::spawn(move || runtime.block_on(serve(first, name, requests)));
        Ok(Claim::Held)
    }

    async fn serve(mut server: NamedPipeServer, name: String, requests: UnboundedSender<()>) {
        loop {
            if server.connect().await.is_err() {
                return;
            }
            // The next instance is made before this client is answered, so the pipe is never missing for a launcher.
            let next = match ServerOptions::new().create(&name) {
                Ok(next) => next,
                Err(error) => {
                    warn!("The launcher's pipe for showing its window closed: {error}");
                    return;
                }
            };
            if requests.send(()).is_err() {
                return;
            }
            server = next;
        }
    }
}
