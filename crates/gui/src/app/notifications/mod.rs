//! The notification system: the events the launcher raises (a crash, a newer modpack, mod or launcher release, an install
//! or export that ended), the bell's list of them, and the actions their buttons start. The database keeps each one under
//! its key, so an event that is raised again is kept once.

mod checks;
mod crash;
mod model;
mod panel;

use std::future::Future;
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime};

use chrono::{DateTime, Local};
use iced::Task;
use lodestone_platforms::ContentType;
use lodestone_updater::Release;
use tracing::warn;

use super::components::alert::Alert;
use super::downloads::{Job, JobKind};
use super::instances::reveal_folder;
use super::lang;
use super::pages::discover::Source;
use super::pages::instance::Tab;
use super::pages::library::instance::Instance;
use super::pages::resource::Listing;
use super::{LodestoneAppMessage, LodestoneLauncher, updater};

pub use model::{Draft, Kind, Notification, event_key};
pub use panel::bell;

/// The bell's state: the notifications the database keeps, newest first, and whether the bell's panel is open.
#[derive(Debug, Default)]
pub struct Notifications {
    items: Vec<Notification>,
    open: bool,
    /// Whether a background check is under way.
    checking: bool,
    /// When the last background check started.
    checked_at: Option<Instant>,
}

impl Notifications {
    /// How many kept notifications have not been read.
    pub fn unread(&self) -> usize {
        self.items.iter().filter(|item| !item.read).count()
    }

    /// The kept notifications, newest first.
    pub fn items(&self) -> &[Notification] {
        &self.items
    }

    /// Whether the bell's panel is open.
    pub fn is_open(&self) -> bool {
        self.open
    }
}

/// What the bell sends, and what the tasks that read or write notifications send back.
#[derive(Debug, Clone)]
pub enum Message {
    /// The kept notifications were read, or the read failed.
    Loaded(Result<Vec<Notification>, String>),
    /// A write finished: a notification was recorded, dismissed, marked read or cleared. The list is read again.
    Written(Result<(), String>),
    /// A game crashed. The run's crash report, when it left one, is offered by the crash alert. `saved` is the write of
    /// the bell's record of the crash.
    Crashed {
        code: String,
        report: Option<PathBuf>,
        saved: Result<(), String>,
    },
    /// The crash alert was answered. A yes opens the crash report.
    CrashAnswered { report: PathBuf, confirmed: bool },
    /// The bell was pressed, which opens or shuts its panel.
    TogglePanel,
    /// A press outside the panel, or Escape, shuts it.
    DismissPanel,
    /// A row's dismiss button was pressed.
    Dismiss(i64),
    /// The panel's button that marks every notification read.
    MarkAllRead,
    /// The panel's button that clears every notification.
    ClearAll,
    /// A row's button was pressed.
    Act(Action),
    /// The clock ticked: the background checks start if they are due.
    CheckDue,
    /// The background checks finished, so the list and the instances are read again.
    Checked,
}

/// What a notification's button starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Opens the instance's Logs tab.
    Logs(String),
    /// Opens the instance's page, on its default tab.
    Instance(String),
    /// Opens the instance's Mods tab.
    Mods(String),
    /// Opens the instance's Versions tab, where its modpack can be moved onto the newer release.
    Versions(String),
    /// Opens a file in the app its type belongs to.
    OpenFile(PathBuf),
    /// Opens the folder a file is in.
    ShowFolder(PathBuf),
    /// Opens a modpack's page in Discover.
    Modpack { platform: String, project: String, name: String },
    /// Starts the launcher update, as the library's update button does.
    Update,
}

/// The draft of an install that ended with its instance ready.
pub(super) fn install_done(job: u64, instance: &Instance) -> Draft {
    let now = Local::now();
    Draft {
        key: event_key(&format!("install:{job}"), now),
        kind: Kind::InstallDone,
        subject: instance.name.clone(),
        detail: String::new(),
        instance_id: Some(instance.id.clone()),
        target: None,
        created_at: now,
    }
}

/// The draft of an instance moved onto a release of its modpack, in place. Its detail is the release's version number.
pub(super) fn pack_updated(job: u64, instance: &Instance, version: &str) -> Draft {
    let now = Local::now();
    Draft {
        key: event_key(&format!("pack-update:{job}"), now),
        kind: Kind::PackUpdated,
        subject: instance.name.clone(),
        detail: version.to_owned(),
        instance_id: Some(instance.id.clone()),
        target: None,
        created_at: now,
    }
}

/// The draft of an install that did not finish, with the reason it gave.
pub(super) fn install_failed(job: u64, name: String, error: &str) -> Draft {
    let now = Local::now();
    Draft {
        key: event_key(&format!("install:{job}"), now),
        kind: Kind::InstallFailed,
        subject: name,
        detail: error.to_owned(),
        instance_id: None,
        target: None,
        created_at: now,
    }
}

/// The draft of an export that wrote its package at `path`.
pub(super) fn export_done(job: u64, path: &Path) -> Draft {
    let now = Local::now();
    Draft {
        key: event_key(&format!("export:{job}"), now),
        kind: Kind::ExportDone,
        subject: path.file_name().map_or_else(String::new, |name| name.to_string_lossy().into_owned()),
        detail: String::new(),
        instance_id: None,
        target: Some(path.display().to_string()),
        created_at: now,
    }
}

/// The draft of an export that did not write its package, with the reason it gave.
pub(super) fn export_failed(job: u64, name: String, error: &str) -> Draft {
    let now = Local::now();
    Draft {
        key: event_key(&format!("export:{job}"), now),
        kind: Kind::ExportFailed,
        subject: name,
        detail: error.to_owned(),
        instance_id: None,
        target: None,
        created_at: now,
    }
}

/// The name an install or an export shows in the queue, which its notification names too. Other jobs have none.
pub(super) fn job_name(job: &Job) -> String {
    match &job.kind {
        JobKind::Install(request) => request.name.clone(),
        JobKind::Export(export) => export.name.clone(),
        JobKind::PackUpdate(update) => update.instance.name.clone(),
        JobKind::Mod(_) | JobKind::Reinstall(_) => String::new(),
    }
}

/// Runs a write to the database, then reads the list again through [`Message::Written`].
fn written<F>(write: F) -> Task<LodestoneAppMessage>
where
    F: Future<Output = Result<(), String>> + Send + 'static,
{
    Task::perform(write, |result| LodestoneAppMessage::Notifications(Message::Written(result)))
}

/// Opens `path` in the app its type belongs to, on the blocking pool. A failure returns as `FolderOpened`, as a folder's does.
fn open_path(path: PathBuf) -> Task<LodestoneAppMessage> {
    Task::perform(
        async move {
            match tokio::task::spawn_blocking(move || open::that_detached(&path)).await {
                Ok(opened) => opened.map_err(|error| error.to_string()),
                Err(error) => Err(error.to_string()),
            }
        },
        LodestoneAppMessage::FolderOpened,
    )
}

impl LodestoneLauncher {
    /// Reads the kept notifications. The root does it once the database opens, and after every write.
    pub(super) fn load_notifications(&self) -> Task<LodestoneAppMessage> {
        let Some(db) = self.db.clone() else {
            return Task::none();
        };
        Task::perform(async move { db.notifications().await }, |result| {
            LodestoneAppMessage::Notifications(Message::Loaded(result))
        })
    }

    /// Records a notification, then reads the list again.
    pub(super) fn record_notification(&self, draft: Draft) -> Task<LodestoneAppMessage> {
        let Some(db) = self.db.clone() else {
            return Task::none();
        };
        written(async move { db.record_notification(&draft).await })
    }

    /// Records that the run of `instance` that started at `started` crashed with `code`, along with the crash report
    /// the run left, when there is one.
    pub(super) fn record_crash(&self, instance: &str, started: DateTime<Local>, code: String) -> Task<LodestoneAppMessage> {
        let (Some(db), Some(record)) = (self.db.clone(), self.library.instance(instance)) else {
            return Task::none();
        };
        let folder = self.paths.instance(&record.directory_name);
        let now = Local::now();
        let draft = Draft {
            key: event_key(&format!("crash:{instance}"), now),
            kind: Kind::Crash,
            subject: record.name.clone(),
            detail: code.clone(),
            instance_id: Some(instance.to_owned()),
            target: None,
            created_at: now,
        };
        let since = SystemTime::from(started);
        // The report is looked for once: the bell keeps it, and the crash alert offers it.
        Task::perform(
            async move {
                let report = tokio::task::spawn_blocking(move || crash::newest_report_since(&folder, since))
                    .await
                    .ok()
                    .flatten();
                let saved = db
                    .record_notification(&Draft {
                        target: report.as_ref().map(|path| path.display().to_string()),
                        ..draft
                    })
                    .await;
                (code, report, saved)
            },
            |(code, report, saved)| LodestoneAppMessage::Notifications(Message::Crashed { code, report, saved }),
        )
    }

    /// Tells the player a game crashed. When the run left a crash report, the alert offers to show it.
    fn crash_alert(&mut self, code: &str, report: Option<PathBuf>) -> Task<LodestoneAppMessage> {
        let message = lang::t_with("launch.exited_with_error", &[("code", code)]);
        match report {
            Some(report) => self
                .alerts
                .show(
                    Alert::error(message)
                        .title(lang::t("launch.crash.title"))
                        .labelled(lang::t("launch.crash.show_report"), Some(lang::t("common.close"))),
                )
                .map(move |confirmed| {
                    LodestoneAppMessage::Notifications(Message::CrashAnswered {
                        report: report.clone(),
                        confirmed,
                    })
                }),
            None => self.alerts.show(Alert::error(message)).discard(),
        }
    }

    /// Records that a newer launcher release is out. The release's tag is its key, so it is recorded once.
    pub(super) fn notify_launcher_update(&self, release: &Release) -> Task<LodestoneAppMessage> {
        self.record_notification(Draft {
            key: format!("launcher_update:{}", release.tag),
            kind: Kind::LauncherUpdate,
            subject: release.tag.clone(),
            detail: String::new(),
            instance_id: None,
            target: None,
            created_at: Local::now(),
        })
    }

    /// Applies a message from the bell, or from a task that read or wrote a notification.
    pub(super) fn notifications_message(&mut self, message: Message) -> Task<LodestoneAppMessage> {
        match message {
            Message::Loaded(Ok(items)) => {
                self.notifications.items = items;
                Task::none()
            }
            Message::Loaded(Err(error)) => {
                warn!("Could not read the notifications: {error}");
                Task::none()
            }
            Message::Crashed { code, report, saved } => {
                let mut tasks = vec![self.crash_alert(&code, report)];
                match saved {
                    Ok(()) => tasks.push(self.load_notifications()),
                    Err(error) => warn!("Could not record the crash: {error}"),
                }
                Task::batch(tasks)
            }
            Message::CrashAnswered { report, confirmed } => {
                if confirmed {
                    open_path(report)
                } else {
                    Task::none()
                }
            }
            Message::Written(Ok(())) => self.load_notifications(),
            Message::Written(Err(error)) => {
                warn!("Could not write a notification: {error}");
                self.load_notifications()
            }
            Message::TogglePanel => {
                self.notifications.open = !self.notifications.open;
                Task::none()
            }
            Message::DismissPanel => {
                self.notifications.open = false;
                Task::none()
            }
            Message::Dismiss(id) => match self.db.clone() {
                Some(db) => written(async move { db.dismiss_notification(id).await }),
                None => Task::none(),
            },
            Message::MarkAllRead => match self.db.clone() {
                Some(db) => written(async move { db.mark_notifications_read(Local::now()).await }),
                None => Task::none(),
            },
            Message::ClearAll => match self.db.clone() {
                Some(db) => written(async move { db.clear_notifications().await }),
                None => Task::none(),
            },
            Message::Act(action) => {
                self.notifications.open = false;
                self.notification_action(action)
            }
            Message::CheckDue => self.check_updates_if_due(),
            Message::Checked => {
                self.notifications.checking = false;
                // The checks record the mods they identify, so the Discover page reads its records again to show them as added.
                Task::batch([self.load_notifications(), self.load_instances(), self.load_mod_sources()])
            }
        }
    }

    /// Starts the background checks, unless one is under way or the last one is recent. They run once the library has
    /// loaded, and then on the clock every few hours.
    pub(super) fn check_updates_if_due(&mut self) -> Task<LodestoneAppMessage> {
        let Some(db) = self.db.clone() else {
            return Task::none();
        };
        let recent = self.notifications.checked_at.is_some_and(|at| at.elapsed() < checks::EVERY);
        if self.notifications.checking || recent {
            return Task::none();
        }
        self.notifications.checking = true;
        self.notifications.checked_at = Some(Instant::now());
        let targets = self.check_targets();
        Task::perform(checks::run(db, targets), |()| LodestoneAppMessage::Notifications(Message::Checked))
    }

    /// The installed instances the checks look at, each with its modpack when it came from a platform.
    fn check_targets(&self) -> Vec<checks::Target> {
        self.library
            .instances()
            .iter()
            .filter(|instance| instance.version_id.is_some())
            .map(|instance| checks::Target {
                id: instance.id.clone(),
                name: instance.name.clone(),
                mc_version: instance.mc_version.clone(),
                loader: instance.loader.to_db().to_string(),
                folder: self.paths.instance(&instance.directory_name),
                pack: match (&instance.source_platform, &instance.source_project_id) {
                    (Some(platform), Some(project_id)) => Some(checks::Pack {
                        platform: platform.clone(),
                        project_id: project_id.clone(),
                        installed: instance.source_version.clone(),
                    }),
                    _ => None,
                },
            })
            .collect()
    }

    /// Carries out a notification's button: a page or tab it names, a file, or the launcher update.
    fn notification_action(&mut self, action: Action) -> Task<LodestoneAppMessage> {
        match action {
            Action::Logs(id) => self.open_instance_page(&id, Tab::Logs),
            Action::Instance(id) => self.open_instance_page(&id, Tab::Content),
            Action::Versions(id) => self.open_instance_page(&id, Tab::Versions),
            Action::Mods(id) => self.open_instance_page(&id, Tab::Content),
            Action::OpenFile(path) => open_path(path),
            Action::ShowFolder(path) => match path.parent() {
                Some(folder) => reveal_folder(folder.to_path_buf()),
                None => Task::none(),
            },
            Action::Modpack { platform, project, name } => match Source::from_key(&platform) {
                Some(source) => self.open_resource(
                    Listing {
                        source,
                        content: ContentType::Modpack,
                        project_id: project,
                        name,
                        author: None,
                        summary: String::new(),
                        downloads: 0,
                        icon_url: None,
                        web_url: None,
                        updated: None,
                    },
                    None,
                ),
                None => Task::none(),
            },
            Action::Update => self.updater_message(updater::Message::Pressed),
        }
    }
}

#[cfg(test)]
#[path = "../../../tests/app/notifications/mod.rs"]
mod tests;
