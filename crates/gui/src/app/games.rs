//! The game sessions as the root handles them: starting a game, the output and exit it reports,
//! the controls that stop or restart it, and the state those leave in the database.

use std::process::ExitStatus;

use chrono::{DateTime, Local};
use iced::Task;
use lodestone_minecraft_authentication::Session;
use tracing::{info, warn};

use super::storage::StoredMicrosoft;
use super::{Account, LodestoneAppMessage, LodestoneLauncher};
use crate::app::components::alert::Alert;
use crate::app::lang;
use crate::app::launch::GameMessage;
use crate::app::launch::games::{GameView, LaunchStatus, pump};
use crate::app::launch::prepare::{self, Prepared, Progress, Request, Target};
use crate::app::pages::instance::Tab;
use crate::app::pages::library::instance::InstanceState;

impl LodestoneLauncher {
    /// Routes one game message.
    pub(super) fn game_message(&mut self, message: GameMessage) -> Task<LodestoneAppMessage> {
        match message {
            GameMessage::Launch(id) => self.launch_game(&id),
            GameMessage::Kill(id) => self.kill_game(&id),
            GameMessage::Restart(id) => self.restart_game(&id),
            GameMessage::KillLatest => match self.games.latest_running().map(str::to_owned) {
                Some(id) => self.kill_game(&id),
                None => Task::none(),
            },
            GameMessage::ConsoleLatest => match self.games.latest_running().map(str::to_owned) {
                Some(id) => self.open_instance_page(&id, Tab::Logs),
                None => Task::none(),
            },
            GameMessage::Preparing { id, serial, progress } => {
                let Progress { fraction, .. } = progress;
                self.games.set_progress(&id, serial, fraction);
                Task::none()
            }
            GameMessage::Prepared { id, serial, result } => self.prepared_game(&id, serial, result),
            GameMessage::Output { id, serial, stream, line } => {
                self.games.push_output(&id, serial, stream, line);
                Task::none()
            }
            GameMessage::Exited { id, serial, status } => self.game_exited(&id, serial, status),
            GameMessage::StreamEnded => Task::none(),
        }
    }

    /// Starts the game of `id`. A game that is already running has its console opened instead.
    pub(super) fn launch_game(&mut self, id: &str) -> Task<LodestoneAppMessage> {
        if self.games.is_running(id) {
            return self.open_instance_page(id, Tab::Logs);
        }
        if self.games.is_active(id) {
            return Task::none();
        }
        let (Some(instance), Some(account)) = (self.library.instance(id), self.account.clone()) else {
            warn!("Launch of {id} ignored: the instance is not loaded or nobody is signed in");
            return Task::none();
        };
        let serial = self.games.next_serial();
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
            auth: self.auth.clone(),
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

    /// Stops the game of `id`, or aborts its preparation.
    pub(super) fn kill_game(&mut self, id: &str) -> Task<LodestoneAppMessage> {
        info!("Stopping {id}");
        self.games.kill(id);
        Task::none()
    }

    /// Stops the game of `id` and starts it again once it has exited. A game that is not
    /// running starts at once.
    pub(super) fn restart_game(&mut self, id: &str) -> Task<LodestoneAppMessage> {
        if self.games.is_running(id) {
            self.games.mark_restart(id);
            self.games.kill(id);
            return Task::none();
        }
        self.games.kill(id);
        self.launch_game(id)
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
        self.games.start(id, serial, process, Local::now());
        info!("Started {id}");

        let mut tasks = vec![
            self.record_game_state(id, InstanceState::Playing),
            Task::sip(pump(id.to_string(), serial, events), LodestoneAppMessage::Game, |()| {
                LodestoneAppMessage::Game(GameMessage::StreamEnded)
            }),
        ];
        if let Some(session) = prepared.refreshed {
            tasks.push(self.keep_session(session));
        }
        Task::batch(tasks)
    }

    /// Records that the game of `id` ended. Its playtime is added, and a restart or an
    /// unexpected failure is handled.
    fn game_exited(&mut self, id: &str, serial: u64, status: ExitStatus) -> Task<LodestoneAppMessage> {
        let Some(session) = self.games.take_running(id, serial) else {
            return Task::none();
        };
        let killed = self.games.take_killed(id);
        let restart = self.games.take_restart(id);
        info!("{id} exited with code {:?}", status.code());
        let seconds = u32::try_from(session.since.elapsed().as_secs()).unwrap_or(u32::MAX);
        let mut tasks = vec![self.finish_game_session(id, session.started, seconds)];
        if !killed && !status.success() {
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
            tasks.push(Task::done(LodestoneAppMessage::Game(GameMessage::Launch(id.to_string()))));
        }
        Task::batch(tasks)
    }

    /// Sends `make` for the instance the page shows, when one is shown.
    pub(super) fn current_game(&mut self, make: fn(String) -> GameMessage) -> Task<LodestoneAppMessage> {
        match self.instance_page.instance().map(|instance| instance.id.clone()) {
            Some(id) => self.game_message(make(id)),
            None => Task::none(),
        }
    }

    /// What the instance page needs to draw the game of `id`.
    pub(super) fn game_view(&self, id: &str) -> GameView<'_> {
        GameView {
            status: self.games.status(id),
            console: self.games.console(id),
        }
    }

    /// What the instance page needs for the instance it shows.
    pub(super) fn page_game_view(&self) -> GameView<'_> {
        match self.instance_page.instance() {
            Some(instance) => self.game_view(&instance.id),
            None => GameView {
                status: LaunchStatus::Idle,
                console: None,
            },
        }
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

    /// Adds a finished play session to the instance `id`, then reloads the library so the cards
    /// show the new playtime.
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

    /// Keeps a Microsoft session that was refreshed for a launch, and the refresh token it
    /// carries, so the next launch and the next start of the launcher use them.
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
