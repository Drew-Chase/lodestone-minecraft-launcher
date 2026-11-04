//! The update check. At launch the launcher asks GitHub for a newer release. An installed Windows build downloads the
//! installer, checks it against the release's checksums, and runs it silently. Other builds show a button that opens
//! the release page. The Settings page decides whether an installed build updates on its own, and whether prereleases
//! count.

use std::path::PathBuf;
use std::time::Duration;

use aurora_leaf::Icon;
use iced::widget::{Space, markdown, row, text};
use iced::{Element, Fill, Task, Theme};
use lodestone_updater::{Release, fetch_installer, installed_with_installer, run_silent_elevated};
use tracing::{info, warn};

use super::{LodestoneAppMessage, LodestoneLauncher};
use crate::app::components::TextButton;
use crate::app::components::alert::Alert;
use crate::app::components::modal::Modal;
use crate::app::lang;
use crate::app::theme::{color, fonts};

/// The User-Agent of the update check. GitHub's API refuses requests that carry none.
const USER_AGENT: &str = concat!("lodestone_launcher/", env!("CARGO_PKG_VERSION"));

/// How often the update check asks GitHub again while the launcher runs.
pub const CHECK_EVERY: Duration = Duration::from_secs(120);

/// The update check's messages.
#[derive(Debug, Clone)]
pub enum Message {
    /// The release check answered: a newer release, nothing newer, or why it could not answer.
    Checked(Result<Option<Release>, String>),
    /// The installer was downloaded and passed its checksum, or the download failed.
    Downloaded(Result<PathBuf, String>),
    /// The update button was pressed. A newer release opens its changelog.
    Pressed,
    /// The changelog's Update button was pressed: the update starts.
    Confirm,
    /// The changelog was dismissed with Cancel, the scrim or Escape.
    Dismiss,
    /// A link in the changelog was clicked.
    OpenLink(String),
    /// The check's clock fell due. The check asks GitHub again unless an update is already under way.
    CheckDue,
    /// The answer to "install while a game is running?".
    InstallConfirmed(bool),
    /// Windows started the installer, or did not.
    Launched(Result<(), String>),
}

/// A downloaded installer that passed its checksum, with the release it belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verified {
    release: Release,
    installer: PathBuf,
}

/// Where the update check has got to.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Status {
    /// Nothing to show: no newer release, or the check could not answer.
    #[default]
    Idle,
    /// A newer release is out. The button opens its page, or downloads it on an installed build that updates by hand.
    Available(Release),
    /// The installer is downloading, and its checksum is not yet checked.
    Downloading(Release),
    /// The verified installer waits until no game is running.
    Held(Verified),
    /// The verified installer can run now. It is here after Windows refused to start it, so a press tries again.
    Ready(Verified),
    /// The installer was started, and the launcher is about to close.
    Installing(Verified),
    /// The update could not be fetched or checked. It is dropped until the next launch.
    Failed(String),
}

/// The update check's state, kept by the root.
#[derive(Debug, Default)]
pub struct Updater {
    status: Status,
    /// The changelog of the newer release, parsed, while its dialog is open.
    dialog: Option<markdown::Content>,
}

/// What the topbar's update button shows: its hint, and whether a press does anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Button {
    /// The hint that says what the update is doing.
    pub tooltip: &'static str,
    /// Whether a press acts. A download or an install in progress cannot be pressed.
    pub enabled: bool,
}

impl Updater {
    /// The topbar button for the current status, or none when there is no update to show.
    pub fn button(&self) -> Option<Button> {
        let (tooltip, enabled) = match &self.status {
            Status::Idle | Status::Failed(_) => return None,
            Status::Available(_) => (lang::t("updater.button.available"), true),
            Status::Downloading(_) => (lang::t("updater.button.downloading"), false),
            Status::Held(_) | Status::Ready(_) => (lang::t("updater.button.ready"), true),
            Status::Installing(_) => (lang::t("updater.button.installing"), false),
        };
        Some(Button { tooltip, enabled })
    }

    /// The changelog dialog for a newer release, while it is open. Its footer has Update, which starts the update, and Cancel.
    pub fn modal(&self) -> Option<Element<'_, Message>> {
        let notes = self.dialog.as_ref()?;
        let Status::Available(release) = &self.status else {
            return None;
        };
        let body: Element<'_, Message> = if release.body.trim().is_empty() {
            text(lang::t("updater.dialog.no_notes"))
                .size(14)
                .font(fonts::Inter::REGULAR)
                .color(color::text_muted())
                .into()
        } else {
            let links: Element<'_, markdown::Uri> = markdown::view(notes.items(), notes_settings());
            links.map(Message::OpenLink)
        };
        let footer = row![
            Space::new().width(Fill),
            TextButton::ghost(lang::t("updater.dialog.cancel")).on_press(Message::Dismiss),
            TextButton::primary(lang::t("updater.dialog.update")).on_press(Message::Confirm),
        ]
        .spacing(10)
        .align_y(iced::alignment::Vertical::Center);
        let title = lang::t_with("updater.dialog.title", &[("version", release.tag.as_str())]);
        Some(
            Modal::new(title, body)
                .subtitle(lang::t("updater.dialog.subtitle"))
                .icon(Icon::lucide().download(), color::primary(), color::primary_tint())
                .footer(footer)
                .on_close(Message::Dismiss)
                .close_on_scrim(true)
                .into(),
        )
    }

    /// Whether the check should ask GitHub again: nothing is under way, so a newer release can only be found, not acted on.
    fn awaiting_check(&self) -> bool {
        matches!(self.status, Status::Idle | Status::Available(_))
    }
}

/// The markdown settings of the changelog: body text at 14 px, and the platform's link colour.
fn notes_settings() -> markdown::Settings {
    let style = markdown::Style {
        link_color: color::primary(),
        ..markdown::Style::from(&Theme::Dark)
    };
    markdown::Settings::with_text_size(14, style)
}

/// Whether the update check runs. A portable launcher skips it, and so does a debug build unless
/// `LODESTONE_UPDATE_CHECK` is set, so a development run never offers the release's update.
pub fn enabled(portable: bool) -> bool {
    !portable && (!cfg!(debug_assertions) || std::env::var_os("LODESTONE_UPDATE_CHECK").is_some())
}

/// Asks GitHub for the newest release, once, in the background. The beta channel counts prereleases too.
pub fn check(beta: bool) -> Task<LodestoneAppMessage> {
    Task::perform(ask_github(beta), |result| LodestoneAppMessage::Updater(Message::Checked(result)))
}

async fn ask_github(beta: bool) -> Result<Option<Release>, String> {
    let client = lodestone_updater::client(USER_AGENT).map_err(|error| error.to_string())?;
    lodestone_updater::latest_for(&client, env!("CARGO_PKG_VERSION"), beta)
        .await
        .map_err(|error| error.to_string())
}

/// Downloads the installer of `release` into the temporary folder, and checks it.
async fn fetch(release: Release) -> Result<PathBuf, String> {
    let client = lodestone_updater::client(USER_AGENT).map_err(|error| error.to_string())?;
    let dir = std::env::temp_dir().join("lodestone-update");
    fetch_installer(&client, &release, &dir).await.map_err(|error| error.to_string())
}

/// Runs the installer on the blocking pool. Windows may hold the call while it shows the administrator prompt.
async fn run_installer(installer: PathBuf) -> Result<(), String> {
    tokio::task::spawn_blocking(move || run_silent_elevated(&installer))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

/// Whether this launcher can install an update itself: any Linux build (its installer script runs in the user's
/// folders), or an installed Windows build.
fn installer_build() -> bool {
    cfg!(target_os = "linux") || (cfg!(windows) && std::env::current_exe().is_ok_and(|exe| installed_with_installer(&exe)))
}

impl LodestoneLauncher {
    /// Applies an update message.
    pub(super) fn updater_message(&mut self, message: Message) -> Task<LodestoneAppMessage> {
        match message {
            Message::Checked(Ok(Some(release))) => {
                let notice = self.notify_launcher_update(&release);
                Task::batch([notice, self.newer_release(release)])
            }
            Message::Checked(Ok(None)) => Task::none(),
            Message::Checked(Err(error)) => {
                warn!("The update check failed: {error}");
                Task::none()
            }
            Message::Downloaded(Ok(installer)) => self.downloaded(installer),
            Message::Downloaded(Err(error)) => {
                warn!("The update could not be downloaded: {error}");
                self.updater.status = Status::Failed(error.clone());
                self.alerts
                    .show(Alert::warn(lang::t_with("updater.download_failed", &[("error", error.as_str())])))
                    .discard()
            }
            Message::Pressed => self.pressed(),
            Message::Confirm => self.confirm(),
            Message::Dismiss => {
                self.updater.dialog = None;
                Task::none()
            }
            Message::OpenLink(url) => super::instances::open_source(url),
            Message::CheckDue => self.check_due(),
            Message::InstallConfirmed(true) => self.start_install(),
            Message::InstallConfirmed(false) => Task::none(),
            Message::Launched(Ok(())) => iced::exit(),
            Message::Launched(Err(error)) => self.install_refused(error),
        }
    }

    /// A newer release is out. An installed Windows build downloads it now when Auto-update Lodestone is on. Any other
    /// build, or one with Auto-update off, shows the button, and a press on an installed build downloads it then.
    fn newer_release(&mut self, release: Release) -> Task<LodestoneAppMessage> {
        if !installer_build() || !self.settings_values.auto_update {
            // The check runs again every few minutes, so the release is logged the first time it is seen.
            if self.updater.status != Status::Available(release.clone()) {
                info!("Update {} is available", release.tag);
            }
            self.updater.status = Status::Available(release);
            return Task::none();
        }
        self.download(release)
    }

    /// Downloads the installer of `release` and checks it. The button shows the download until it is done.
    fn download(&mut self, release: Release) -> Task<LodestoneAppMessage> {
        info!("Downloading update {}", release.tag);
        self.updater.status = Status::Downloading(release.clone());
        Task::perform(fetch(release), |result| LodestoneAppMessage::Updater(Message::Downloaded(result)))
    }

    /// The installer is downloaded and checked. It runs now, unless a game is running, in which case it waits.
    fn downloaded(&mut self, installer: PathBuf) -> Task<LodestoneAppMessage> {
        let release = match &self.updater.status {
            Status::Downloading(release) => release.clone(),
            _ => return Task::none(),
        };
        let verified = Verified { release, installer };
        if self.games.any_active() {
            info!("Holding the update until the games have closed");
            self.updater.status = Status::Held(verified);
            return Task::none();
        }
        self.updater.status = Status::Ready(verified);
        self.start_install()
    }

    /// The button was pressed. A newer release opens its changelog, where Update starts it. A verified installer runs, after a
    /// question when a game is running.
    fn pressed(&mut self) -> Task<LodestoneAppMessage> {
        match self.updater.status.clone() {
            Status::Available(release) => {
                info!("Showing the changelog of {}", release.tag);
                self.updater.dialog = Some(markdown::Content::parse(&release.body));
                Task::none()
            }
            Status::Held(_) | Status::Ready(_) if self.games.any_active() => self
                .alerts
                .show(
                    Alert::warn(lang::t("updater.confirm.message"))
                        .title(lang::t("updater.confirm.title"))
                        .yes_no(),
                )
                .map(|install| LodestoneAppMessage::Updater(Message::InstallConfirmed(install))),
            Status::Held(_) | Status::Ready(_) => self.start_install(),
            _ => Task::none(),
        }
    }

    /// The changelog's Update button. An installed Windows build downloads the release, and any other build opens its page.
    fn confirm(&mut self) -> Task<LodestoneAppMessage> {
        self.updater.dialog = None;
        match self.updater.status.clone() {
            Status::Available(release) if installer_build() => self.download(release),
            Status::Available(release) => super::instances::open_source(release.html_url),
            _ => Task::none(),
        }
    }

    /// The clock asked for a check. It runs when the launcher may hear of a newer release, and the updates are off for a
    /// portable launcher and a debug build as they are at launch.
    fn check_due(&mut self) -> Task<LodestoneAppMessage> {
        if !enabled(self.portable.is_some()) || !self.updater.awaiting_check() {
            return Task::none();
        }
        check(self.settings_values.beta_channel)
    }

    /// Runs the verified installer silently. Windows asks for administrator rights first, and the launcher closes once
    /// the installer has started.
    fn start_install(&mut self) -> Task<LodestoneAppMessage> {
        let verified = match self.updater.status.clone() {
            Status::Held(verified) | Status::Ready(verified) => verified,
            _ => return Task::none(),
        };
        info!("Installing update {}", verified.release.tag);
        let installer = verified.installer.clone();
        self.updater.status = Status::Installing(verified);
        Task::perform(run_installer(installer), |result| LodestoneAppMessage::Updater(Message::Launched(result)))
    }

    /// Windows did not start the installer, for example because the administrator prompt was declined. The update stays
    /// ready, so the button tries again.
    fn install_refused(&mut self, error: String) -> Task<LodestoneAppMessage> {
        if let Status::Installing(verified) = self.updater.status.clone() {
            self.updater.status = Status::Ready(verified);
        }
        warn!("The update did not start: {error}");
        self.alerts
            .show(Alert::warn(lang::t_with("updater.not_started", &[("error", error.as_str())])))
            .discard()
    }

    /// A game has ended. A held update installs once no game is left.
    pub(super) fn resume_held_update(&mut self) -> Task<LodestoneAppMessage> {
        if matches!(self.updater.status, Status::Held(_)) && !self.games.any_active() {
            return self.start_install();
        }
        Task::none()
    }
}

#[cfg(test)]
#[path = "../../../tests/app/updater/mod.rs"]
mod tests;
