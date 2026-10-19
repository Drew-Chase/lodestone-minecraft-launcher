//! Spawning the JVM and supervising it: the crate's only side-effectful module.
//!
//! All platform-conditional code lives here — deliberately no `windows.rs` /
//! `macos.rs` / `linux.rs` split; the crate's platform surface is one creation
//! flag wide (DESIGN.md §6.2).
//!
//! Shape of a launch: `launch()` re-validates, spawns, and starts two drain
//! threads **before returning** (an undrained pipe fills and the JVM blocks on
//! write — the classic launcher deadlock). A finalizer thread emits exactly one
//! [`LaunchEvent::Exited`] after both pipes end, after which the event channel
//! reports `Disconnected`.

use std::io::{BufRead, BufReader, Read};
use std::process::{Child, ExitStatus};
use std::sync::mpsc::{self, Receiver, Sender, SyncSender, sync_channel};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use tracing::warn;

use crate::command::build_command;
use crate::config::Launcher;
use crate::error::LaunchError;
use crate::events::{LaunchEvent, LogStream};

/// Queued-event capacity before producers block (DESIGN.md §6.1, ratified §8.1):
/// a stalled consumer applies backpressure to the game instead of ballooning
/// launcher memory.
const EVENT_CHANNEL_BOUND: usize = 1024;

/// How often the finalizer re-polls the child while waiting for its exit status.
/// Both output pipes have already ended at this point, so the child is exiting
/// (or has closed its own stdio); the first poll normally succeeds.
const EXIT_POLL_INTERVAL: Duration = Duration::from_millis(10);

/// `CREATE_NO_WINDOW`: do not flash a console window behind the GUI.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

impl Launcher {
    /// Spawns the game described by this configuration and returns a handle to it.
    ///
    /// Cheap: validates, spawns, starts the output drain threads, and returns
    /// immediately — safe to call directly from a UI update step. Waiting belongs
    /// to the event channel (see [`GameProcess::take_events`]), not to a blocking
    /// call here. Launching the same configuration twice is legal and stateless.
    ///
    /// The filesystem facts validated at build time are re-checked here: they can
    /// change between `build()` and `launch()`.
    pub fn launch(&self) -> Result<GameProcess, LaunchError> {
        if !self.java_path().is_absolute() || !self.java_path().is_file() {
            return Err(LaunchError::JavaNotFound {
                path: self.java_path().to_path_buf(),
            });
        }
        if !self.working_directory().is_dir() {
            return Err(LaunchError::WorkingDirectoryMissing {
                path: self.working_directory().to_path_buf(),
            });
        }

        let mut command = build_command(self)?;

        // Keep the crate's platform surface one creation flag wide.
        #[cfg(windows)]
        command.creation_flags(CREATE_NO_WINDOW);

        let mut child = command.spawn().map_err(|source| LaunchError::SpawnFailed {
            java_path: self.java_path().to_path_buf(),
            source,
        })?;

        // Invariant: build_command pipes both output streams.
        let stdout = child.stdout.take().expect("build_command always pipes the game's stdout");
        let stderr = child.stderr.take().expect("build_command always pipes the game's stderr");

        let child = Arc::new(Mutex::new(child));

        let (events_tx, events_rx) = sync_channel::<LaunchEvent>(EVENT_CHANNEL_BOUND);
        let (eof_tx, eof_rx) = mpsc::channel::<()>();
        let (status_tx, status_rx) = mpsc::channel::<ExitStatus>();

        spawn_drain(stdout, LogStream::Stdout, events_tx.clone(), eof_tx.clone());
        spawn_drain(stderr, LogStream::Stderr, events_tx.clone(), eof_tx);

        thread::Builder::new()
            .name("lodestone-launch-finalizer".into())
            .spawn({
                let child = Arc::clone(&child);
                move || finalize(child, eof_rx, status_tx, events_tx)
            })
            .expect("failed to spawn the finalizer thread");

        Ok(GameProcess {
            child,
            events: Some(events_rx),
            status: status_rx,
            suppress_drop_warning: false,
        })
    }
}

/// Starts one drain thread for a game output pipe. It starts immediately —
/// before `launch()` returns — which is what keeps the JVM from deadlocking on
/// a full pipe.
fn spawn_drain(pipe: impl Read + Send + 'static, stream: LogStream, events: SyncSender<LaunchEvent>, eof: Sender<()>) {
    thread::Builder::new()
        .name(format!("lodestone-launch-{stream:?}"))
        .spawn(move || drain(pipe, stream, events, eof))
        .expect("failed to spawn an output drain thread");
}

/// Reads `pipe` to end-of-stream, forwarding decoded lines as log events.
///
/// Uses `read_until` + `from_utf8_lossy` rather than `BufRead::lines`, which
/// errors out on non-UTF-8 bytes (Windows console code pages are not guaranteed
/// UTF-8).
fn drain(pipe: impl Read, stream: LogStream, events: SyncSender<LaunchEvent>, eof: Sender<()>) {
    let mut reader = BufReader::new(pipe);
    let mut raw = Vec::new();
    loop {
        raw.clear();
        match reader.read_until(b'\n', &mut raw) {
            Ok(0) => break, // end of stream: the pipe is closed for good
            Ok(_) => {
                let mut line = String::from_utf8_lossy(&raw).into_owned();
                // `read_until` keeps the terminator; strip `\n` and a Windows `\r`.
                if line.ends_with('\n') {
                    line.pop();
                }
                if line.ends_with('\r') {
                    line.pop();
                }
                // Blocking send: backpressure onto the game (see the channel
                // bound). If the consumer is gone, the game outran us on
                // purpose; stop draining rather than spin.
                if events.send(LaunchEvent::Log { stream, line }).is_err() {
                    break;
                }
            }
            Err(error) => {
                warn!(stream = ?stream, %error, "failed to read the game's output stream");
                break;
            }
        }
    }
    let _ = eof.send(());
    // `events` and `eof` drop here; when every sender is gone the receiver
    // reports `Disconnected`.
}

/// Waits for both output pipes to end, then reports the exit status exactly once.
///
/// Runs on its own thread. After it sends [`LaunchEvent::Exited`] and drops its
/// senders, the receiver reports `Disconnected`.
fn finalize(child: Arc<Mutex<Child>>, eof: Receiver<()>, status_tx: Sender<ExitStatus>, events: SyncSender<LaunchEvent>) {
    // One signal per drain thread; both must have finished before `Exited` is
    // emitted, so it always lands after the last log line.
    let _ = eof.recv();
    let _ = eof.recv();

    let exit_status = loop {
        let polled = child.lock().expect("game process lock poisoned").try_wait();
        match polled {
            Ok(Some(status)) => break status,
            Ok(None) => thread::sleep(EXIT_POLL_INTERVAL),
            Err(error) => {
                // The child's status is unknowable; report disconnection
                // without an exit status rather than inventing one.
                warn!(%error, "failed to collect the game's exit status");
                return;
            }
        }
    };

    let _ = status_tx.send(exit_status);
    let _ = events.send(LaunchEvent::Exited { status: exit_status });
}

/// A handle to one running game instance.
///
/// The game may outlive the launcher: dropping this handle logs a warning and
/// deliberately does **not** kill the game ([`GameProcess::kill`] is explicit;
/// ratified §8.2/§8.4). The finalizer thread keeps reaping the child even after
/// the handle is gone, so no zombie process is left behind on Unix.
#[derive(Debug)]
pub struct GameProcess {
    child: Arc<Mutex<Child>>,
    events: Option<Receiver<LaunchEvent>>,
    status: Receiver<ExitStatus>,
    suppress_drop_warning: bool,
}

impl GameProcess {
    /// The child's process id.
    #[must_use]
    pub fn id(&self) -> u32 {
        self.child.lock().expect("game process lock poisoned").id()
    }

    /// Non-blocking exit check; `Some` once the game has exited.
    pub fn try_wait(&mut self) -> Result<Option<ExitStatus>, LaunchError> {
        self.child
            .lock()
            .expect("game process lock poisoned")
            .try_wait()
            .map_err(|source| LaunchError::WaitFailed { source })
    }

    /// Blocks until the game exits and returns its status.
    pub fn wait(mut self) -> Result<ExitStatus, LaunchError> {
        let result = match self.status.recv() {
            Ok(status) => Ok(status),
            Err(_) => {
                // The finalizer failed to collect a status; make one direct
                // attempt before giving up.
                self.child
                    .lock()
                    .expect("game process lock poisoned")
                    .wait()
                    .map_err(|source| LaunchError::WaitFailed { source })
            }
        };
        // A caller that waited actively supervised the game to the end; the
        // dropped-handle warning is not for them.
        self.suppress_drop_warning = true;
        result
    }

    /// Forcibly kills the game. No-op if it already exited.
    pub fn kill(&mut self) -> Result<(), LaunchError> {
        let mut child = self.child.lock().expect("game process lock poisoned");
        if child.try_wait().map_err(|source| LaunchError::WaitFailed { source })?.is_some() {
            return Ok(());
        }
        child.kill().map_err(|source| LaunchError::KillFailed { source })
    }

    /// Takes exclusive ownership of this game's event stream.
    ///
    /// One owner pumps the stream (std mpsc receivers are not `Clone`), which
    /// matches gui's one-subscription model exactly. After the single
    /// [`LaunchEvent::Exited`], the channel reports `Disconnected`.
    ///
    /// # Panics
    /// Panics if the receiver was already taken: the stream has exactly one
    /// owner, so a second take is a caller bug.
    pub fn take_events(&mut self) -> Receiver<LaunchEvent> {
        self.events
            .take()
            .expect("launch events were already taken: the event stream has exactly one owner")
    }
}

impl Drop for GameProcess {
    fn drop(&mut self) {
        if !self.suppress_drop_warning {
            // Deliberate warn-and-orphan (ratified §8.4): the game may outlive
            // the launcher, and `kill()` is the explicit way to stop it.
            warn!("GameProcess dropped while the game may still be running; the game was not killed");
        }
    }
}
