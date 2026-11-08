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

/// Unix: a socket in the config folder, and a lock beside it. Holding the lock is the claim. The OS releases the lock when the
/// launcher ends, so a socket a crashed launcher left behind is replaced, and never mistaken for a launcher that runs.
#[cfg(unix)]
mod platform {
    use std::fs::{File, OpenOptions, TryLockError};
    use std::io::{self, Write};
    use std::os::unix::net::{UnixListener, UnixStream};
    use std::path::{Path, PathBuf};
    use std::sync::OnceLock;
    use std::thread;
    use std::time::{Duration, Instant};

    use tokio::sync::mpsc::UnboundedSender;
    use tracing::warn;

    use super::Claim;

    /// A Unix socket path holds fewer than 104 bytes, so a deeper folder, which a portable launcher's can be, gets a short
    /// name in the runtime or temp folder instead. Both paths name the same socket for the same folder.
    const SOCKET_PATH_LIMIT: usize = 100;

    /// How long a launcher that finds the lock taken keeps trying to reach the launcher holding it. That one may still be
    /// starting its listener.
    const REACH_FOR: Duration = Duration::from_secs(5);

    /// How often that reaching is tried again.
    const REACH_EVERY: Duration = Duration::from_millis(100);

    /// The lock file, kept open for as long as the launcher runs.
    static HELD: OnceLock<File> = OnceLock::new();

    /// The socket's path: `launcher.sock` in the config folder, or a name in the runtime or temp folder when that path is too long.
    pub fn endpoint() -> Option<PathBuf> {
        let dir = crate::app::paths::config_dir()?;
        let path = dir.join("launcher.sock");
        if path.as_os_str().len() < SOCKET_PATH_LIMIT {
            return Some(path);
        }
        Some(short_socket_path(&dir))
    }

    /// The socket's name for the config folder `dir`, unique to that folder. The runtime folder is the user's own, where the
    /// system provides one. The shared temp folder is the fallback.
    fn short_socket_path(dir: &Path) -> PathBuf {
        let folder = std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .filter(|runtime| runtime.is_dir())
            .unwrap_or_else(std::env::temp_dir);
        folder.join(format!("lodestone-{:016x}.sock", stable_hash(dir.as_os_str().as_encoded_bytes())))
    }

    /// FNV-1a. Unlike the standard library's hasher, its result does not change between Rust releases, so two builds of the
    /// launcher name the same socket for the same folder.
    fn stable_hash(bytes: &[u8]) -> u64 {
        bytes.iter().fold(0xcbf2_9ce4_8422_2325u64, |hash, &byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
        })
    }

    /// The lock file. It is in the config folder, which belongs to this user, and never in the shared temp folder.
    fn lock_path(endpoint: &Path) -> PathBuf {
        crate::app::paths::config_dir().map_or_else(|| endpoint.with_extension("lock"), |dir| dir.join("launcher.lock"))
    }

    pub fn claim(path: &Path, requests: UnboundedSender<()>) -> io::Result<Claim> {
        // The lock may sit in the config folder while the socket sits in a shorter folder, so each folder is made here.
        let lock = lock_path(path);
        if let Some(dir) = lock.parent() {
            std::fs::create_dir_all(dir)?;
        }
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(lock_path(path))?;
        match file.try_lock() {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => return ask_holder(path),
            Err(TryLockError::Error(error)) => return Err(error),
        }
        // Holding the lock, any socket still at this path was left by a launcher that ended, so it is replaced.
        let _ = HELD.set(file);
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

    /// Asks the launcher that holds the lock to show its window. That launcher may still be starting its listener, so the
    /// connection is tried for a moment. One that never answers is not a reason to run a second launcher, so this one exits.
    fn ask_holder(path: &Path) -> io::Result<Claim> {
        let deadline = Instant::now() + REACH_FOR;
        loop {
            if let Ok(mut stream) = UnixStream::connect(path) {
                // The message is only a line to read, so a failed write still leaves the other launcher to be shown.
                let _ = stream.write_all(b"show\n");
                return Ok(Claim::Taken);
            }
            if Instant::now() >= deadline {
                warn!("Another launcher holds the lock but does not answer, so this one exits");
                return Ok(Claim::Taken);
            }
            thread::sleep(REACH_EVERY);
        }
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
