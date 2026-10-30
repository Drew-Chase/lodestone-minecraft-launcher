//! The game sessions the launcher tracks: the instances being prepared or running, each one's
//! console, and which game the title bar and sidebar act on.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::mpsc::Receiver;
use std::time::Instant;

use chrono::{DateTime, Local};
use iced::task::Handle;
use lodestone_launch::{GameProcess, LaunchEvent, LogStream};
use sipper::Sender as ProgressSender;
use sipper::{Sipper, sipper};

use super::GameMessage;
use crate::app::pages::instance::logs::{LogLevel, LogLine, MAX_LINES};

/// What an instance's controls show for its game.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum LaunchStatus {
    #[default]
    Idle,
    /// Files are being checked and fetched; the fraction is `None` until the first count lands.
    Preparing(Option<f32>),
    Running,
}

/// One game the launcher started and has not yet seen exit.
#[derive(Debug)]
pub struct Running {
    pub process: GameProcess,
    pub serial: u64,
    pub started: DateTime<Local>,
    pub since: Instant,
}

/// One preparation that has not finished yet.
struct Preparing {
    serial: u64,
    fraction: Option<f32>,
    abort: Handle,
}

/// What the instance page needs to draw an instance's game.
#[derive(Debug, Clone, Copy)]
pub struct GameView<'a> {
    pub status: LaunchStatus,
    /// The console lines of the game while it runs, when there are any.
    pub console: Option<&'a VecDeque<LogLine>>,
}

/// The launcher's game sessions.
#[derive(Default)]
pub struct Games {
    preparing: HashMap<String, Preparing>,
    running: HashMap<String, Running>,
    /// Instances to start again once their running game exits.
    restarting: HashSet<String>,
    /// Instances whose game the user stopped, so its exit is not reported as a failure.
    killed: HashSet<String>,
    /// The console of each instance's latest game.
    console: HashMap<String, VecDeque<LogLine>>,
    /// The instance launched most recently, which the title bar and sidebar act on.
    latest: Option<String>,
    serial: u64,
}

impl Games {
    /// A number no earlier launch has used, so a late message from one can be told apart.
    pub fn next_serial(&mut self) -> u64 {
        self.serial += 1;
        self.serial
    }

    /// Records a preparation that has started, with the handle that aborts it.
    pub fn begin(&mut self, id: &str, serial: u64, abort: Handle) {
        self.preparing.insert(
            id.to_string(),
            Preparing {
                serial,
                fraction: None,
                abort,
            },
        );
    }

    /// Records the fraction a preparation has reached, when it is still the current one.
    pub fn set_progress(&mut self, id: &str, serial: u64, fraction: Option<f32>) {
        if let Some(preparing) = self.preparing.get_mut(id).filter(|preparing| preparing.serial == serial) {
            preparing.fraction = fraction;
        }
    }

    /// Ends a preparation. Returns `false` when it was aborted or replaced, so its result is stale.
    pub fn finish_preparing(&mut self, id: &str, serial: u64) -> bool {
        match self.preparing.get(id) {
            Some(preparing) if preparing.serial == serial => {
                self.preparing.remove(id);
                true
            }
            _ => false,
        }
    }

    /// Records a game the process has started, and clears the console for it.
    pub fn start(&mut self, id: &str, serial: u64, process: GameProcess, started: DateTime<Local>) {
        self.console.entry(id.to_string()).or_default().clear();
        self.running.insert(
            id.to_string(),
            Running {
                process,
                serial,
                started,
                since: Instant::now(),
            },
        );
        self.latest = Some(id.to_string());
    }

    /// Takes the game of `id` once its exit arrives, when it is still the game that was started.
    pub fn take_running(&mut self, id: &str, serial: u64) -> Option<Running> {
        match self.running.get(id) {
            Some(running) if running.serial == serial => self.running.remove(id),
            _ => None,
        }
    }

    /// Adds a console line to the running game of `id`, when it is still the game that was started.
    pub fn push_output(&mut self, id: &str, serial: u64, stream: LogStream, line: String) {
        if self.running.get(id).is_none_or(|running| running.serial != serial) {
            return;
        }
        let log = self.console.entry(id.to_string()).or_default();
        if log.len() == MAX_LINES {
            log.pop_front();
        }
        let level = match stream {
            LogStream::Stdout => LogLevel::Info,
            LogStream::Stderr => LogLevel::Error,
        };
        log.push_back(LogLine {
            time: Local::now().format("%H:%M:%S").to_string(),
            level,
            message: line,
        });
    }

    /// The status of `id`'s game.
    pub fn status(&self, id: &str) -> LaunchStatus {
        if self.running.contains_key(id) {
            LaunchStatus::Running
        } else if let Some(preparing) = self.preparing.get(id) {
            LaunchStatus::Preparing(preparing.fraction)
        } else {
            LaunchStatus::Idle
        }
    }

    /// Whether `id` is being prepared or is running.
    pub fn is_active(&self, id: &str) -> bool {
        self.running.contains_key(id) || self.preparing.contains_key(id)
    }

    /// Whether `id`'s game is running.
    pub fn is_running(&self, id: &str) -> bool {
        self.running.contains_key(id)
    }

    /// The console lines of `id`'s game, while that game runs. Once it exits, its log file is the
    /// record, so there is no console to show.
    pub fn console(&self, id: &str) -> Option<&VecDeque<LogLine>> {
        if self.running.contains_key(id) { self.console.get(id) } else { None }
    }

    /// The instance whose game the title bar and sidebar act on: the latest launch, while it runs,
    /// or else any running game.
    pub fn latest_running(&self) -> Option<&str> {
        match &self.latest {
            Some(id) if self.running.contains_key(id) => Some(id.as_str()),
            _ => self.running.keys().next().map(String::as_str),
        }
    }

    /// Stops `id`: aborts its preparation, or kills its running game. A kill is not reported as a failure.
    pub fn kill(&mut self, id: &str) {
        if let Some(preparing) = self.preparing.remove(id) {
            preparing.abort.abort();
        }
        if let Some(running) = self.running.get_mut(id) {
            self.killed.insert(id.to_string());
            if let Err(error) = running.process.kill() {
                tracing::warn!("could not kill the game of {id}: {error}");
            }
        }
    }

    /// Marks `id` to start again once its running game exits.
    pub fn mark_restart(&mut self, id: &str) {
        self.restarting.insert(id.to_string());
    }

    /// Whether `id` was marked to start again, which clears the mark.
    pub fn take_restart(&mut self, id: &str) -> bool {
        self.restarting.remove(id)
    }

    /// Whether the user stopped `id`'s game, which clears the mark.
    pub fn take_killed(&mut self, id: &str) -> bool {
        self.killed.remove(id)
    }
}

/// The lines of one game's output, as they arrive, then its exit. It ends after the exit.
pub fn pump(id: String, serial: u64, events: Receiver<LaunchEvent>) -> impl Sipper<(), GameMessage> {
    sipper(move |mut progress: ProgressSender<GameMessage>| async move {
        let (sender, mut received) = tokio::sync::mpsc::unbounded_channel::<LaunchEvent>();
        std::thread::spawn(move || {
            for event in events {
                if sender.send(event).is_err() {
                    break;
                }
            }
        });
        while let Some(event) = received.recv().await {
            let message = match event {
                LaunchEvent::Log { stream, line } => GameMessage::Output {
                    id: id.clone(),
                    serial,
                    stream,
                    line,
                },
                LaunchEvent::Exited { status } => GameMessage::Exited {
                    id: id.clone(),
                    serial,
                    status,
                },
            };
            progress.send(message).await;
        }
    })
}

#[cfg(test)]
#[path = "../../../tests/app/launch/games.rs"]
mod tests;
