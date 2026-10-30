//! Starting the game: the version files a launch reads, the arguments it passes, who the player
//! is, and the sessions the launcher tracks while a game is being prepared or runs.

pub mod arguments;
pub mod games;
pub mod identity;
pub mod libraries;
pub mod manifest;
pub mod pidfile;
pub mod prepare;
pub mod rules;

use std::process::ExitStatus;

use lodestone_launch::LogStream;

use games::RunsPlace;
use pidfile::Record;
use prepare::{Prepared, Progress};

/// The messages the game sessions send: what a launch does, and what a running game reports.
#[derive(Debug, Clone)]
pub enum GameMessage {
    /// Starts the game of the instance `id`. While it has a run going, its console opens instead.
    Launch(String),
    /// Starts another run of the instance `id`, even while it has runs going.
    LaunchNew(String),
    /// Stops every run of the instance `id`, or aborts its preparation.
    Kill(String),
    /// Starts the runs of the instance `id` again: each is stopped, and a new run starts once it exits.
    Restart(String),
    /// Stops the run with this serial.
    KillRun(u64),
    /// Opens the instance page on the console of the run with this serial.
    ConsoleRun(u64),
    /// Opens or shuts the panel of running games at `place`.
    ToggleRuns(RunsPlace),
    /// Shuts the panel of running games at `place`.
    DismissRuns(RunsPlace),
    /// The runs a previous launcher left running, found through their pid files, to attach to.
    Reattach(Vec<Record>),
    /// A preparation reported its progress.
    Preparing { id: String, serial: u64, progress: Progress },
    /// A preparation finished, with the launch it describes or the reason it failed.
    Prepared {
        id: String,
        serial: u64,
        result: Result<Box<Prepared>, String>,
    },
    /// A running game wrote a line.
    Output { serial: u64, stream: LogStream, line: String },
    /// A running game exited. A reattached game's exit status went with the launcher that started it,
    /// so its exit comes through as `None`.
    Exited { serial: u64, status: Option<ExitStatus> },
    /// A game's event stream ended. Nothing needs to happen.
    StreamEnded,
}
