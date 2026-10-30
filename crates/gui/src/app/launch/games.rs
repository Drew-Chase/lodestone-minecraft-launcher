//! The game sessions the launcher tracks. An instance is being prepared, or it has one or more runs:
//! each run is one game process, with its own console. The store also holds the Microsoft session
//! refresh that launches wait for, and which runs the title bar and sidebar show.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::mpsc::Receiver;
use std::time::Instant;

use chrono::{DateTime, Local};
use iced::Task;
use iced::task::Handle;
use lodestone_launch::{GameProcess, LaunchEvent, LogStream};
use sipper::Sender as ProgressSender;
use sipper::{Sipper, sipper};

use super::GameMessage;
use super::pidfile::Record;
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

/// The process behind one run.
#[derive(Debug)]
pub enum Process {
    /// A run this launcher started. Its output and its exit come through the pipes it holds.
    Launched(GameProcess),
    /// A run a previous launcher started, found again through its pid file. That launcher held its
    /// output pipes and its exit status, so only the process is watched.
    Reattached(Record),
}

/// One game process that is running and has not yet been seen to exit.
#[derive(Debug)]
pub struct Running {
    pub instance: String,
    pub pid: u32,
    pub process: Process,
    pub started: DateTime<Local>,
    pub since: Instant,
}

/// One preparation that has not finished yet.
struct Preparing {
    serial: u64,
    fraction: Option<f32>,
    abort: Handle,
}

/// One running game as the runs panel names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    /// Identifies the run, for the buttons that act on it.
    pub serial: u64,
    /// The id of the instance the run belongs to, which the button on its row starts again.
    pub instance: String,
    /// The instance's name, with the run's number when the instance has more than one run.
    pub name: String,
}

/// What the game controls show: every running game, oldest first.
#[derive(Debug, Clone, Default)]
pub struct GameBar {
    pub runs: Vec<Run>,
}

/// What the instance page needs to draw an instance's game.
#[derive(Debug, Clone)]
pub struct GameView<'a> {
    pub status: LaunchStatus,
    /// The console lines of the run the page shows, when there are any.
    pub console: Option<&'a VecDeque<LogLine>>,
}

/// Where a runs panel is: beside the sidebar's indicator, or under the Windows title bar's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunsPlace {
    Sidebar,
    Titlebar,
}

/// Which runs panels are open. Each place keeps its own, so the two can be open together.
#[derive(Debug, Clone, Copy, Default)]
pub struct RunsOpen {
    sidebar: bool,
    titlebar: bool,
}

impl RunsOpen {
    /// Whether the panel at `place` is open.
    pub fn is_open(self, place: RunsPlace) -> bool {
        match place {
            RunsPlace::Sidebar => self.sidebar,
            RunsPlace::Titlebar => self.titlebar,
        }
    }

    /// Opens the panel at `place` when it is shut, and shuts it when it is open.
    pub fn toggle(&mut self, place: RunsPlace) {
        let open = self.slot(place);
        *open = !*open;
    }

    /// Shuts the panel at `place`.
    pub fn dismiss(&mut self, place: RunsPlace) {
        *self.slot(place) = false;
    }

    fn slot(&mut self, place: RunsPlace) -> &mut bool {
        match place {
            RunsPlace::Sidebar => &mut self.sidebar,
            RunsPlace::Titlebar => &mut self.titlebar,
        }
    }
}

/// The Microsoft session refresh in flight: the refresh token it spends, and the launches waiting
/// for it. A refresh token is good for one use, so every launch that needs the refresh shares one.
#[derive(Debug, Default)]
struct Refresh {
    /// The refresh token being spent, while a refresh runs.
    token: Option<String>,
    /// The launches waiting for the refresh, as instance id and serial, in the order they asked.
    waiting: Vec<(String, u64)>,
}

/// The launcher's game sessions.
#[derive(Default)]
pub struct Games {
    /// The preparation of each instance, at most one at a time.
    preparing: HashMap<String, Preparing>,
    /// Every run, keyed by its serial.
    running: HashMap<u64, Running>,
    /// The runs to start again once they exit.
    restarting: HashSet<u64>,
    /// The runs the user stopped, so their exit is not reported as a failure.
    killed: HashSet<u64>,
    /// The console of each run this launcher started.
    console: HashMap<u64, VecDeque<LogLine>>,
    /// The Microsoft session refresh in flight, and the launches that wait for it.
    refresh: Refresh,
    serial: u64,
}

impl Games {
    /// A number no earlier launch has used, so a late message from one can be told apart. A run
    /// takes the serial of the preparation it came from.
    pub fn next_serial(&mut self) -> u64 {
        self.serial += 1;
        self.serial
    }

    /// Records a preparation that has started, with the handle that aborts it.
    pub fn begin(&mut self, instance: &str, serial: u64, abort: Handle) {
        self.preparing.insert(
            instance.to_string(),
            Preparing {
                serial,
                fraction: None,
                abort,
            },
        );
    }

    /// Records a launch that waits for a Microsoft session refresh before it prepares. It shows as
    /// preparing, so a second press does not start it again, and it has nothing to abort until then.
    pub fn wait_for_session(&mut self, instance: &str, serial: u64) {
        let (_task, abort) = Task::<()>::none().abortable();
        self.begin(instance, serial, abort);
        self.refresh.waiting.push((instance.to_string(), serial));
    }

    /// Starts a session refresh that spends `token`, unless one is already running. Returns whether
    /// this call started it.
    pub fn start_refresh(&mut self, token: String) -> bool {
        if self.refresh.token.is_some() {
            return false;
        }
        self.refresh.token = Some(token);
        true
    }

    /// Ends the refresh in flight. Returns the refresh token it spent and the launches that waited for it.
    pub fn finish_refresh(&mut self) -> (Option<String>, Vec<(String, u64)>) {
        (self.refresh.token.take(), std::mem::take(&mut self.refresh.waiting))
    }

    /// Whether the launch `serial` of `instance` is still the one being prepared, so it has not been
    /// stopped or replaced since it was recorded.
    pub fn is_preparing_with(&self, instance: &str, serial: u64) -> bool {
        self.preparing.get(instance).is_some_and(|preparing| preparing.serial == serial)
    }

    /// Whether `instance` is being prepared.
    pub fn is_preparing(&self, instance: &str) -> bool {
        self.preparing.contains_key(instance)
    }

    /// Records the fraction a preparation has reached, when it is still the current one.
    pub fn set_progress(&mut self, instance: &str, serial: u64, fraction: Option<f32>) {
        if let Some(preparing) = self.preparing.get_mut(instance).filter(|preparing| preparing.serial == serial) {
            preparing.fraction = fraction;
        }
    }

    /// Ends a preparation. Returns `false` when it was aborted or replaced, so its result is stale.
    pub fn finish_preparing(&mut self, instance: &str, serial: u64) -> bool {
        match self.preparing.get(instance) {
            Some(preparing) if preparing.serial == serial => {
                self.preparing.remove(instance);
                true
            }
            _ => false,
        }
    }

    /// Records a run the process has started, under the preparation's `serial`, and clears its
    /// console. Returns the process id.
    pub fn start(&mut self, instance: &str, serial: u64, process: GameProcess, started: DateTime<Local>) -> u32 {
        let pid = process.id();
        self.console.insert(serial, VecDeque::new());
        self.running.insert(
            serial,
            Running {
                instance: instance.to_string(),
                pid,
                process: Process::Launched(process),
                started,
                since: Instant::now(),
            },
        );
        pid
    }

    /// Records a run a previous launcher started and this one found still running. Its output and
    /// exit are not followed; only its process is watched. Returns the serial its exit is reported under.
    pub fn attach(&mut self, record: Record, started: DateTime<Local>) -> u64 {
        let serial = self.next_serial();
        let elapsed = (Local::now() - started).to_std().unwrap_or_default();
        let since = Instant::now().checked_sub(elapsed).unwrap_or_else(Instant::now);
        self.running.insert(
            serial,
            Running {
                instance: record.instance.clone(),
                pid: record.pid,
                process: Process::Reattached(record),
                started,
                since,
            },
        );
        serial
    }

    /// Takes the run `serial` once its exit arrives.
    pub fn take_running(&mut self, serial: u64) -> Option<Running> {
        self.running.remove(&serial)
    }

    /// Adds a console line to the run `serial`, when it is still running.
    pub fn push_output(&mut self, serial: u64, stream: LogStream, line: String) {
        if !self.running.contains_key(&serial) {
            return;
        }
        let log = self.console.entry(serial).or_default();
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

    /// The status of `instance`: running while any of its runs goes, otherwise preparing or idle.
    pub fn status(&self, instance: &str) -> LaunchStatus {
        if self.is_running(instance) {
            LaunchStatus::Running
        } else if let Some(preparing) = self.preparing.get(instance) {
            LaunchStatus::Preparing(preparing.fraction)
        } else {
            LaunchStatus::Idle
        }
    }

    /// Whether `instance` has at least one run going.
    pub fn is_running(&self, instance: &str) -> bool {
        self.running.values().any(|running| running.instance == instance)
    }

    /// Whether `instance` is being prepared or has a run going.
    pub fn is_active(&self, instance: &str) -> bool {
        self.is_running(instance) || self.is_preparing(instance)
    }

    /// The instance a run belongs to.
    pub fn instance_of(&self, serial: u64) -> Option<&str> {
        self.running.get(&serial).map(|running| running.instance.as_str())
    }

    /// Every run, oldest first, with its instance.
    pub fn runs(&self) -> Vec<(u64, &str)> {
        let mut runs: Vec<(DateTime<Local>, u64, &str)> = self
            .running
            .iter()
            .map(|(serial, running)| (running.started, *serial, running.instance.as_str()))
            .collect();
        runs.sort();
        runs.into_iter().map(|(_, serial, instance)| (serial, instance)).collect()
    }

    /// The most recently started run of `instance`.
    pub fn newest_run(&self, instance: &str) -> Option<u64> {
        self.running
            .iter()
            .filter(|(_, running)| running.instance == instance)
            .max_by_key(|(serial, running)| (running.started, **serial))
            .map(|(serial, _)| *serial)
    }

    /// The console lines of the run `serial`, while it runs and this launcher started it. Once a
    /// run exits, its log file is the record; a reattached run has no output of its own to show.
    pub fn console(&self, serial: u64) -> Option<&VecDeque<LogLine>> {
        match self.running.get(&serial) {
            Some(Running {
                process: Process::Launched(_),
                ..
            }) => self.console.get(&serial),
            _ => None,
        }
    }

    /// Stops `instance`: aborts its preparation, and kills every run of it. A kill is not reported
    /// as a failure. A reattached run is not this launcher's child, so its record comes back for the
    /// caller to kill, off the UI thread.
    pub fn kill_instance(&mut self, instance: &str) -> Vec<Record> {
        if let Some(preparing) = self.preparing.remove(instance) {
            preparing.abort.abort();
        }
        let serials: Vec<u64> = self
            .running
            .iter()
            .filter(|(_, running)| running.instance == instance)
            .map(|(serial, _)| *serial)
            .collect();
        serials.into_iter().filter_map(|serial| self.kill_run(serial)).collect()
    }

    /// Stops the run `serial`. Its record comes back when it is a reattached run, for the caller to
    /// kill off the UI thread.
    pub fn kill_run(&mut self, serial: u64) -> Option<Record> {
        let running = self.running.get_mut(&serial)?;
        self.killed.insert(serial);
        match &mut running.process {
            Process::Launched(process) => {
                if let Err(error) = process.kill() {
                    tracing::warn!("could not kill the game run {serial}: {error}");
                }
                None
            }
            Process::Reattached(record) => Some(record.clone()),
        }
    }

    /// Marks every run of `instance` to start again once it exits.
    pub fn mark_restart(&mut self, instance: &str) {
        let serials: Vec<u64> = self
            .running
            .iter()
            .filter(|(_, running)| running.instance == instance)
            .map(|(serial, _)| *serial)
            .collect();
        self.restarting.extend(serials);
    }

    /// Whether the run `serial` was marked to start again, which clears the mark.
    pub fn take_restart(&mut self, serial: u64) -> bool {
        self.restarting.remove(&serial)
    }

    /// Whether the user stopped the run `serial`, which clears the mark.
    pub fn take_killed(&mut self, serial: u64) -> bool {
        self.killed.remove(&serial)
    }
}

/// The lines of one run's output, as they arrive, then its exit. It ends after the exit.
pub fn pump(serial: u64, events: Receiver<LaunchEvent>) -> impl Sipper<(), GameMessage> {
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
                LaunchEvent::Log { stream, line } => GameMessage::Output { serial, stream, line },
                LaunchEvent::Exited { status } => GameMessage::Exited {
                    serial,
                    status: Some(status),
                },
            };
            progress.send(message).await;
        }
    })
}

#[cfg(test)]
#[path = "../../../tests/app/launch/games.rs"]
mod tests;
