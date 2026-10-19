//! The event stream produced by a running game process.

use std::process::ExitStatus;

/// Which of the game's output streams a [`LaunchEvent::Log`] line came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogStream {
    /// The game's standard output.
    Stdout,
    /// The game's standard error.
    Stderr,
}

/// One occurrence in a running game's event stream.
///
/// Exactly one `Exited` is emitted per process and it is always the final event:
/// it is sent once both output pipes have hit end-of-stream and the child's exit
/// status has been collected. After it, the channel reports `Disconnected` —
/// the gui's "game is gone" signal.
#[derive(Debug, Clone)]
pub enum LaunchEvent {
    /// A decoded line from one of the game's output streams, without its line
    /// terminator.
    Log {
        /// Which stream the line came from.
        stream: LogStream,
        /// The line's content, decoded with `from_utf8_lossy` (Windows console
        /// code pages are not guaranteed UTF-8).
        line: String,
    },
    /// The game has exited and its exit status was collected.
    Exited {
        /// The child's exit status.
        status: ExitStatus,
    },
}
