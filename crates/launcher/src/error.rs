//! Errors surfaced by validating and launching a game.

use std::env::JoinPathsError;
use std::path::PathBuf;

/// Everything that can go wrong between validating a launch description and
/// supervising the resulting process.
///
/// Variants are user-presentable: gui surfaces these messages verbatim, so each
/// carries the context needed to explain itself — the offending path where there
/// is one, and the underlying [`std::io::Error`] where one exists. Validation
/// errors are attached to the offending path, never a bare "failed to launch".
#[derive(Debug, thiserror::Error)]
pub enum LaunchError {
    /// The java executable is not usable: it is relative, does not exist, or is
    /// not a file.
    #[error("java executable is missing, is not a file, or is not an absolute path: {}", path.display())]
    JavaNotFound {
        /// The offending java path.
        path: PathBuf,
    },

    /// The configured working directory does not exist or is not a directory.
    #[error("working directory does not exist: {}", path.display())]
    WorkingDirectoryMissing {
        /// The offending working directory path.
        path: PathBuf,
    },

    /// A main class was configured but the classpath is empty, so there is
    /// nothing to run it from.
    #[error("classpath is empty but a main class ({main_class}) was configured")]
    EmptyClasspath {
        /// The configured main class, echoed for context.
        main_class: String,
    },

    /// The classpath cannot be joined on this platform: an entry contains the
    /// platform separator.
    #[error("classpath contains an entry with the platform path separator in it: {source}")]
    InvalidClasspath {
        /// The underlying join failure.
        #[source]
        source: JoinPathsError,
    },

    /// The JVM could not be spawned.
    #[error("failed to spawn java at {}: {source}", java_path.display())]
    SpawnFailed {
        /// The java path this crate attempted to execute.
        java_path: PathBuf,
        /// The underlying spawn failure.
        #[source]
        source: std::io::Error,
    },

    /// The running game could not be killed.
    #[error("failed to kill the game process: {source}")]
    KillFailed {
        /// The underlying failure.
        #[source]
        source: std::io::Error,
    },

    /// Waiting on the game process failed.
    #[error("failed to wait for the game process: {source}")]
    WaitFailed {
        /// The underlying failure.
        #[source]
        source: std::io::Error,
    },
}
