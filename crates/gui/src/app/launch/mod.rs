//! Starting the game: the version files a launch reads, the arguments it passes, who the player
//! is, and the sessions the launcher tracks while a game is being prepared or runs.

pub mod arguments;
pub mod games;
pub mod identity;
pub mod libraries;
pub mod manifest;
pub mod prepare;
pub mod rules;

use std::process::ExitStatus;

use lodestone_launch::LogStream;

use prepare::{Prepared, Progress};

/// The messages the game sessions send: what a launch does, and what a running game reports.
#[derive(Debug, Clone)]
pub enum GameMessage {
    /// Starts the game of the instance `id`. A game already running has its console opened.
    Launch(String),
    /// Stops the game of the instance `id`, or aborts its preparation.
    Kill(String),
    /// Starts the game of the instance `id` again: it is stopped, and started once it has exited.
    Restart(String),
    /// Stops the game the title bar and the sidebar act on.
    KillLatest,
    /// Opens the console of the game the title bar and the sidebar act on.
    ConsoleLatest,
    /// A preparation reported its progress.
    Preparing { id: String, serial: u64, progress: Progress },
    /// A preparation finished, with the launch it describes or the reason it failed.
    Prepared {
        id: String,
        serial: u64,
        result: Result<Box<Prepared>, String>,
    },
    /// A running game wrote a line.
    Output {
        id: String,
        serial: u64,
        stream: LogStream,
        line: String,
    },
    /// A running game exited.
    Exited { id: String, serial: u64, status: ExitStatus },
    /// A game's event stream ended. Nothing needs to happen.
    StreamEnded,
}
