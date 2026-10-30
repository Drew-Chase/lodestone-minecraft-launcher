//! The game sessions as the root handles them: starting a game, or another run of one, the output and
//! exit each run reports, the controls that stop or restart them, the games a previous launcher left
//! running, and the state those leave in the database.

use std::collections::HashMap;
use std::process::ExitStatus;
use std::time::Duration;

use chrono::{DateTime, Local, Utc};
use iced::Task;
use lodestone_minecraft_authentication::Session;
use sipper::Sender as ProgressSender;
use sipper::{Sipper, sipper};
use tracing::{info, warn};

use super::storage::StoredMicrosoft;
use super::{Account, LodestoneAppMessage, LodestoneLauncher};
use crate::app::components::alert::Alert;
use crate::app::lang;
use crate::app::launch::GameMessage;
use crate::app::launch::games::{GameBar, GameView, LaunchStatus, Run, RunsPlace, pump};
use crate::app::launch::pidfile::{self, Record};
use crate::app::launch::prepare::{self, Prepared, Progress, Request, SessionCheck, Target};
use crate::app::pages::instance::Tab;
use crate::app::pages::library::instance::InstanceState;

/// How often the process of a reattached run is checked for its exit.
const WATCH_INTERVAL: Duration = Duration::from_secs(2);

impl LodestoneLauncher {
    /// Routes one game message.
    pub(super) fn game_message(&mut self, message: GameMessage) -> Task<LodestoneAppMessage> {
        match message {
            GameMessage::Launch(id) => self.launch_game(&id),
            GameMessage::LaunchNew(id) => self.launch_new_game(&id),
            GameMessage::Kill(id) => self.kill_game(&id),
            GameMessage::Restart(id) => self.restart_game(&id),
            GameMessage::KillRun(serial) => self.kill_run(serial),
            GameMessage::ConsoleRun(serial) => self.open_run_console(serial),
            GameMessage::ToggleRuns(place) => {
                self.runs_open.toggle(place);
                Task::none()
            }
            GameMessage::DismissRuns(place) => {
                self.runs_open.dismiss(place);
                Task::none()
            }
            GameMessage::Reattach(records) => self.reattach_games(records),
            GameMessage::Preparing { id, serial, progress } => {
                let Progress { fraction, .. } = progress;
                self.games.set_progress(&id, serial, fraction);
                Task::none()
            }
            GameMessage::Prepared { id, serial, result } => self.prepared_game(&id, serial, result),
            GameMessage::Output { serial, stream, line } => {
                self.games.push_output(serial, stream, line);
                Task::none()
            }
            GameMessage::Exited { serial, status } => self.game_exited(serial, status),
            GameMessage::StreamEnded => Task::none(),
        }
    }

    /// Starts the game of `id`. While the instance has a run going, its console opens instead.
    pub(super) fn launch_game(&mut self, id: &str) -> Task<LodestoneAppMessage> {
        if self.games.is_running(id) {
            return self.open_instance_page(id, Tab::Logs);
        }
        self.launch_new_game(id)
    }

    /// Starts another run of `id`, even while it has runs going. A preparation of the instance that
    /// is still under way is not started again.
    pub(super) fn launch_new_game(&mut self, id: &str) -> Task<LodestoneAppMessage> {
        if self.games.is_preparing(id) {
            return Task::none();
        }
        if self.library.instance(id).is_none() || self.account.is_none() {
            warn!("Launch of {id} ignored: the instance is not loaded or nobody is signed in");
            return Task::none();
        }
        let serial = self.games.next_serial();
        self.begin_launch(id, serial)
    }

    /// Starts the launch `serial` of `id` once its session is fresh. A Microsoft session close to
    /// expiry is refreshed first, one refresh at a time, and the launch waits for that refresh.
    fn begin_launch(&mut self, id: &str, serial: u64) -> Task<LodestoneAppMessage> {
        let Some(account) = self.account.clone() else {
            self.games.finish_preparing(id, serial);
            return Task::none();
        };
        match prepare::session_check(&account, prepare::unix_now()) {
            SessionCheck::Fresh => self.start_preparing(id, serial),
            SessionCheck::SignInAgain => {
                self.games.finish_preparing(id, serial);
                warn!("Launching {id} failed: the Microsoft session needs a sign in");
                self.alerts
                    .show(Alert::error(lang::t_with("launch.failed", &[("error", lang::t("launch.sign_in_again"))])))
                    .discard()
            }
            SessionCheck::Refresh(token) => {
                self.games.wait_for_session(id, serial);
                if !self.games.start_refresh(token.clone()) {
                    // A refresh is already running, and this launch is among the ones waiting for it.
                    return Task::none();
                }
                info!("Refreshing the Microsoft session before launching {id}");
                let auth = self.auth.clone();
                Task::perform(
                    async move { auth.login_with_refresh_token(&token).await.map_err(|error| error.to_string()) },
                    LodestoneAppMessage::SessionRefreshed,
                )
            }
        }
    }

    /// Starts preparing the launch `serial` of `id` with the account signed in now.
    fn start_preparing(&mut self, id: &str, serial: u64) -> Task<LodestoneAppMessage> {
        let (Some(instance), Some(account)) = (self.library.instance(id), self.account.clone()) else {
            warn!("Launch of {id} ignored: the instance is not loaded or nobody is signed in");
            self.games.finish_preparing(id, serial);
            return Task::none();
        };
        let request = Request {
            paths: self.paths.clone(),
            target: Target {
                mc_version: instance.mc_version.clone(),
                loader: instance.loader,
                loader_version: instance.loader_version.clone(),
                version_id: instance.version_id.clone(),
                directory: self.paths.instance(&instance.directory_name),
            },
            account,
        };
        info!("Preparing {} for launch", instance.name);
        let (progress_id, done_id) = (id.to_string(), id.to_string());
        let (task, abort) = Task::sip(
            prepare::prepare(request),
            move |progress| {
                LodestoneAppMessage::Game(GameMessage::Preparing {
                    id: progress_id.clone(),
                    serial,
                    progress,
                })
            },
            move |result| {
                LodestoneAppMessage::Game(GameMessage::Prepared {
                    id: done_id.clone(),
                    serial,
                    result: result.map(Box::new),
                })
            },
        )
        .abortable();
        self.games.begin(id, serial, abort);
        task
    }

    /// Applies the result of a Microsoft session refresh. The launches that waited for it start with
    /// the new session, or all fail with the reason when the refresh failed.
    pub(super) fn session_refreshed(&mut self, result: Result<Session, String>) -> Task<LodestoneAppMessage> {
        let (spent, waiting) = self.games.finish_refresh();
        match result {
            Ok(session) => {
                // The new session replaces the signed-in one only while that one still holds the token the refresh spent.
                let applied = spent.is_some_and(|token| self.signed_in_with_refresh_token(&token));
                let mut tasks = vec![if applied { self.keep_session(session) } else { Task::none() }];
                for (id, serial) in waiting {
                    if !self.games.is_preparing_with(&id, serial) {
                        // The launch was stopped while it waited.
                        continue;
                    }
                    tasks.push(if applied {
                        self.start_preparing(&id, serial)
                    } else {
                        self.begin_launch(&id, serial)
                    });
                }
                Task::batch(tasks)
            }
            Err(error) => {
                warn!("Refreshing the Microsoft session failed: {error}");
                let mut failed = false;
                for (id, serial) in waiting {
                    failed |= self.games.finish_preparing(&id, serial);
                }
                if failed {
                    self.alerts
                        .show(Alert::error(lang::t_with("launch.failed", &[("error", error.as_str())])))
                        .discard()
                } else {
                    Task::none()
                }
            }
        }
    }

    /// Whether the signed-in account is a Microsoft session holding `token` as its refresh token.
    fn signed_in_with_refresh_token(&self, token: &str) -> bool {
        matches!(&self.account, Some(Account::Microsoft { session }) if session.refresh_token.as_deref() == Some(token))
    }

    /// Stops every run of `id`, and its preparation if one is under way. A reattached run is killed
    /// off the UI thread, and its watcher reports the exit.
    pub(super) fn kill_game(&mut self, id: &str) -> Task<LodestoneAppMessage> {
        info!("Stopping {id}");
        Task::batch(self.games.kill_instance(id).into_iter().map(kill_record))
    }

    /// Stops the run `serial`.
    fn kill_run(&mut self, serial: u64) -> Task<LodestoneAppMessage> {
        info!("Stopping run {serial}");
        match self.games.kill_run(serial) {
            Some(record) => kill_record(record),
            None => Task::none(),
        }
    }

    /// Stops every run of `id` and starts them again: each run is stopped, and a new run starts once
    /// it has exited. An instance with no run starts at once.
    pub(super) fn restart_game(&mut self, id: &str) -> Task<LodestoneAppMessage> {
        if self.games.is_running(id) {
            self.games.mark_restart(id);
            return self.kill_game(id);
        }
        Task::batch([self.kill_game(id), self.launch_game(id)])
    }

    /// Opens the instance page on the console of the run `serial`, and shuts the runs panels, since
    /// the page is what the user goes to next.
    fn open_run_console(&mut self, serial: u64) -> Task<LodestoneAppMessage> {
        let Some(id) = self.games.instance_of(serial).map(str::to_owned) else {
            return Task::none();
        };
        self.runs_open.dismiss(RunsPlace::Sidebar);
        self.runs_open.dismiss(RunsPlace::Titlebar);
        let open = self.open_instance_page(&id, Tab::Logs);
        self.instance_page.focus_console(Some(serial));
        open
    }

    /// Starts the game a finished preparation describes, and records that the instance is playing.
    /// A preparation that was aborted or replaced is dropped.
    fn prepared_game(&mut self, id: &str, serial: u64, result: Result<Box<Prepared>, String>) -> Task<LodestoneAppMessage> {
        if !self.games.finish_preparing(id, serial) {
            return Task::none();
        }
        let prepared = match result {
            Ok(prepared) => prepared,
            Err(error) => {
                warn!("Launching {id} failed: {error}");
                return self
                    .alerts
                    .show(Alert::error(lang::t_with("launch.failed", &[("error", error.as_str())])))
                    .discard();
            }
        };
        let mut process = match prepared.launcher.launch() {
            Ok(process) => process,
            Err(error) => {
                warn!("Launching {id} failed: {error}");
                return self
                    .alerts
                    .show(Alert::error(lang::t_with("launch.failed", &[("error", error.to_string().as_str())])))
                    .discard();
            }
        };
        let events = process.take_events();
        let started = Local::now();
        let pid = self.games.start(id, serial, process, started);
        info!("Started {id} as run {serial}");

        let instance = id.to_string();
        let record: Task<LodestoneAppMessage> = Task::future(async move {
            let recorded = tokio::task::spawn_blocking({
                let instance = instance.clone();
                move || pidfile::record_started(&instance, pid, started.timestamp())
            })
            .await;
            match recorded {
                Ok(Ok(())) => {}
                Ok(Err(error)) => warn!("Could not write the pid file of {instance}: {error}"),
                Err(error) => warn!("Could not write the pid file of {instance}: {error}"),
            }
        })
        .discard();
        Task::batch([
            self.record_game_state(id, InstanceState::Playing),
            record,
            Task::sip(pump(serial, events), LodestoneAppMessage::Game, |()| {
                LodestoneAppMessage::Game(GameMessage::StreamEnded)
            }),
        ])
    }

    /// Records that the run `serial` ended. Its playtime is added and its pid file removed. The
    /// instance goes idle once its last run has ended. A restart starts a new run, and an unexpected
    /// failure is reported. A reattached run's exit status is not known, so its end is never a failure.
    fn game_exited(&mut self, serial: u64, status: Option<ExitStatus>) -> Task<LodestoneAppMessage> {
        let Some(run) = self.games.take_running(serial) else {
            return Task::none();
        };
        let id = run.instance.clone();
        let killed = self.games.take_killed(serial);
        let restart = self.games.take_restart(serial);
        match status {
            Some(status) => info!("{id} run {serial} exited with code {:?}", status.code()),
            None => info!("{id} run {serial} stopped"),
        }
        let seconds = u32::try_from(run.since.elapsed().as_secs()).unwrap_or(u32::MAX);
        let mut tasks = vec![self.finish_game_session(&id, run.started, seconds), remove_pid_file(run.pid)];
        if !self.games.is_active(&id) {
            tasks.push(self.record_game_state(&id, InstanceState::Idle));
        }
        if let Some(status) = status
            && !killed
            && !status.success()
        {
            let code = status
                .code()
                .map_or_else(|| lang::t("launch.exit_by_signal").to_owned(), |code| code.to_string());
            tasks.push(
                self.alerts
                    .show(Alert::error(lang::t_with("launch.exited_with_error", &[("code", code.as_str())])))
                    .discard(),
            );
        }
        if restart {
            tasks.push(Task::done(LodestoneAppMessage::Game(GameMessage::LaunchNew(id))));
        }
        Task::batch(tasks)
    }

    /// Attaches to the runs a previous launcher left running. Each instance with one is recorded as
    /// playing, every other instance the database still shows as playing goes idle, and then the
    /// library reloads.
    fn reattach_games(&mut self, records: Vec<Record>) -> Task<LodestoneAppMessage> {
        let Some(db) = self.db.clone() else {
            return Task::none();
        };
        let mut tasks = Vec::new();
        let mut playing = Vec::new();
        for record in records {
            let started = DateTime::<Utc>::from_timestamp(record.started_at, 0).map_or_else(Local::now, |utc| utc.with_timezone(&Local));
            let id = record.instance.clone();
            info!("Reattached to {id}: its game runs as pid {}", record.pid);
            let serial = self.games.attach(record.clone(), started);
            tasks.push(Task::sip(watch_game(serial, record), LodestoneAppMessage::Game, |()| {
                LodestoneAppMessage::Game(GameMessage::StreamEnded)
            }));
            playing.push(id);
        }
        let states = Task::future(async move {
            if let Err(error) = db.reset_playing().await {
                warn!("Could not reset the play states: {error}");
            }
            for id in playing {
                if let Err(error) = db.set_state(&id, InstanceState::Playing).await {
                    warn!("Could not record the state of {id}: {error}");
                }
            }
        })
        .discard()
        .chain(self.load_instances());
        tasks.push(states);
        Task::batch(tasks)
    }

    /// Sends `make` for the instance the page shows, when one is shown.
    pub(super) fn current_game(&mut self, make: fn(String) -> GameMessage) -> Task<LodestoneAppMessage> {
        match self.instance_page.instance().map(|instance| instance.id.clone()) {
            Some(id) => self.game_message(make(id)),
            None => Task::none(),
        }
    }

    /// What the instance page needs for the instance it shows: its status, and the console of the run it
    /// shows (the one the runs panel opened, else the newest).
    pub(super) fn page_game_view(&self) -> GameView<'_> {
        match self.instance_page.instance() {
            Some(instance) => {
                let run = self
                    .instance_page
                    .focused_run()
                    .filter(|serial| self.games.instance_of(*serial) == Some(instance.id.as_str()))
                    .or_else(|| self.games.newest_run(&instance.id));
                GameView {
                    status: self.games.status(&instance.id),
                    console: run.and_then(|serial| self.games.console(serial)),
                }
            }
            None => GameView {
                status: LaunchStatus::Idle,
                console: None,
            },
        }
    }

    /// The game controls' state: every running game with the name it goes by.
    pub(super) fn game_bar(&self) -> GameBar {
        let runs = self.games.runs();
        let mut counts: HashMap<&str, usize> = HashMap::new();
        for &(_, instance) in &runs {
            *counts.entry(instance).or_default() += 1;
        }
        let mut numbered: HashMap<&str, usize> = HashMap::new();
        let runs = runs
            .iter()
            .map(|&(serial, instance)| {
                let name = self.instance_name(instance);
                let name = if counts[instance] > 1 {
                    let number = numbered.entry(instance).or_default();
                    *number += 1;
                    format!("{name} ({number})")
                } else {
                    name
                };
                Run {
                    serial,
                    instance: instance.to_string(),
                    name,
                }
            })
            .collect();
        GameBar { runs }
    }

    /// The name of the instance `id`. The id stands in for the name when the library no longer lists it.
    fn instance_name(&self, id: &str) -> String {
        self.library.instance_name(id).unwrap_or(id).to_string()
    }

    /// Writes the state of the instance `id`, then reloads the library so the cards show it.
    fn record_game_state(&mut self, id: &str, state: InstanceState) -> Task<LodestoneAppMessage> {
        let Some(db) = self.db.clone() else {
            return Task::none();
        };
        let id = id.to_string();
        Task::future(async move {
            if let Err(error) = db.set_state(&id, state).await {
                warn!("Could not record the state of {id}: {error}");
            }
        })
        .discard()
        .chain(self.load_instances())
    }

    /// Adds a finished run to the instance `id`'s playtime, then reloads the library so the cards show
    /// the new playtime.
    fn finish_game_session(&mut self, id: &str, started: DateTime<Local>, seconds: u32) -> Task<LodestoneAppMessage> {
        let Some(db) = self.db.clone() else {
            return Task::none();
        };
        let id = id.to_string();
        Task::future(async move {
            if let Err(error) = db.finish_session(&id, started, seconds).await {
                warn!("Could not record the play session of {id}: {error}");
            }
        })
        .discard()
        .chain(self.load_instances())
    }

    /// Keeps a Microsoft session that a refresh produced, and the refresh token it carries, so the
    /// next launch and the next start of the launcher use them.
    fn keep_session(&mut self, session: Session) -> Task<LodestoneAppMessage> {
        let save = match &session.refresh_token {
            Some(refresh_token) => {
                self.credentials.microsoft = Some(StoredMicrosoft {
                    refresh_token: refresh_token.clone(),
                    username: session.profile.name.clone(),
                });
                self.save_credentials()
            }
            None => Task::none(),
        };
        self.account = Some(Account::Microsoft { session });
        save
    }
}

/// Kills a reattached run's process off the UI thread. Its exit is then seen by its watcher.
fn kill_record(record: Record) -> Task<LodestoneAppMessage> {
    Task::future(async move {
        match tokio::task::spawn_blocking(move || pidfile::kill(&record)).await {
            Ok(true) => {}
            Ok(false) => info!("The game was already gone when it was stopped"),
            Err(error) => warn!("Could not stop a reattached game: {error}"),
        }
    })
    .discard()
}

/// Deletes the pid file of the game with process id `pid`, off the UI thread.
fn remove_pid_file(pid: u32) -> Task<LodestoneAppMessage> {
    Task::future(async move {
        if let Err(error) = tokio::task::spawn_blocking(move || pidfile::remove(pid)).await {
            warn!("Could not remove a pid file: {error}");
        }
    })
    .discard()
}

/// Watches the process of a reattached run until it is gone. The process is all there is to watch,
/// so the run's end is reported with no exit status.
fn watch_game(serial: u64, record: Record) -> impl Sipper<(), GameMessage> {
    sipper(move |mut progress: ProgressSender<GameMessage>| async move {
        loop {
            tokio::time::sleep(WATCH_INTERVAL).await;
            let check = record.clone();
            let alive = tokio::task::spawn_blocking(move || pidfile::Processes::new().is_game(&check))
                .await
                .unwrap_or(false);
            if !alive {
                progress.send(GameMessage::Exited { serial, status: None }).await;
                return;
            }
        }
    })
}
