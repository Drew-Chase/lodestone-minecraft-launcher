//! The update check. At launch the launcher asks GitHub for a newer release. Where this copy can install it, the release's
//! file downloads in the background when Auto-update Lodestone is on, and the update button installs it when the user
//! presses it. Where this copy cannot, the button opens the release page. The Settings page decides whether the download
//! runs in the background, and whether prereleases count.

mod target;
mod tasks;

pub use target::Target;

use std::path::PathBuf;
use std::time::Duration;

use aurora_leaf::Icon;
use iced::widget::{Space, markdown, row, text};
use iced::{Element, Fill, Task, Theme};
use lodestone_updater::{Answer, Release};
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
pub const CHECK_EVERY: Duration = Duration::from_secs(30 * 60);

/// The update check's messages.
#[derive(Debug, Clone)]
pub enum Message {
    /// The release check answered: a newer release, the same answer as before, or why it could not answer.
    Checked(Result<Answer, String>),
    /// The release's file was downloaded, checked and unpacked, or the download failed.
    Downloaded(Result<Verified, String>),
    /// The update button was pressed. A newer release opens its changelog, and a downloaded one is installed.
    Pressed,
    /// The changelog's Update button was pressed.
    Confirm,
    /// The changelog was dismissed with Cancel, the scrim or Escape.
    Dismiss,
    /// A link in the changelog was clicked.
    OpenLink(String),
    /// The check's clock fell due. The check asks GitHub again unless an update is already under way.
    CheckDue,
    /// The answer to "install while something is still running?".
    InstallConfirmed(bool),
    /// The update was put in place and the launcher is about to restart, or it could not be.
    Applied(Result<(), String>),
}

/// A downloaded release file that passed its checksum, unpacked where the install uses it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verified {
    release: Release,
    /// What the install uses: the setup on Windows, or the new executable or app bundle on Linux and macOS.
    payload: PathBuf,
}

/// Where the update check has got to.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Status {
    /// Nothing to show: no newer release, or the check could not answer.
    #[default]
    Idle,
    /// A newer release is out, and its file is not downloaded.
    Available(Release),
    /// The file is downloading, and its checksum is not yet checked. `install` is set when the changelog's Update button
    /// asked for the install that follows the download.
    Downloading { release: Release, install: bool },
    /// The verified file waits for the user to press the button.
    Ready(Verified),
    /// The update is being put in place, and the launcher is about to restart.
    Installing(Verified),
    /// The update could not be fetched or checked. It is dropped until the next launch.
    Failed(String),
}

/// The update check's state, kept by the root.
#[derive(Debug)]
pub struct Updater {
    status: Status,
    /// The changelog of the newer release, parsed, while its dialog is open.
    dialog: Option<markdown::Content>,
    /// The tag of the last answer. The next check sends it, so GitHub can say that nothing has changed.
    etag: Option<String>,
    /// How this copy takes an update.
    target: Target,
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
    /// The update check for a copy that takes updates the way `target` says.
    pub fn new(target: Target) -> Self {
        Self {
            status: Status::Idle,
            dialog: None,
            etag: None,
            target,
        }
    }

    /// Whether this copy can install an update itself. The Settings page shows its auto-update switch only then.
    pub fn installs(&self) -> bool {
        self.target.installs()
    }

    /// The topbar button for the current status, or none when there is no update to show.
    pub fn button(&self) -> Option<Button> {
        let (tooltip, enabled) = match &self.status {
            Status::Idle | Status::Failed(_) => return None,
            Status::Available(_) => (lang::t("updater.button.available"), true),
            Status::Downloading { .. } => (lang::t("updater.button.downloading"), false),
            Status::Ready(_) => (lang::t("updater.button.ready"), true),
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

    /// Whether the status is already working on the release with `tag`, so that release need not start again.
    fn holds(&self, tag: &str) -> bool {
        match &self.status {
            Status::Downloading { release, .. } => release.tag == tag,
            Status::Ready(verified) | Status::Installing(verified) => verified.release.tag == tag,
            _ => false,
        }
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

/// Asks GitHub for the newest release, once, in the background. The beta channel counts prereleases too. `etag` is the
/// tag of the last answer, which lets GitHub say that nothing has changed.
pub fn check(beta: bool, etag: Option<String>) -> Task<LodestoneAppMessage> {
    Task::perform(ask_github(beta, etag), |result| LodestoneAppMessage::Updater(Message::Checked(result)))
}

async fn ask_github(beta: bool, etag: Option<String>) -> Result<Answer, String> {
    let client = lodestone_updater::client(USER_AGENT).map_err(|error| error.to_string())?;
    lodestone_updater::check(&client, env!("CARGO_PKG_VERSION"), beta, etag.as_deref())
        .await
        .map_err(|error| error.to_string())
}

impl LodestoneLauncher {
    /// Applies an update message.
    pub(super) fn updater_message(&mut self, message: Message) -> Task<LodestoneAppMessage> {
        match message {
            Message::Checked(Ok(Answer::Newer { release, etag })) => {
                self.updater.etag = etag;
                let notice = self.notify_launcher_update(&release);
                Task::batch([notice, self.newer_release(release)])
            }
            Message::Checked(Ok(Answer::Latest { etag })) => {
                self.updater.etag = etag;
                if matches!(self.updater.status, Status::Available(_)) {
                    self.updater.status = Status::Idle;
                }
                Task::none()
            }
            Message::Checked(Ok(Answer::Unchanged)) => Task::none(),
            Message::Checked(Err(error)) => {
                warn!("The update check failed: {error}");
                Task::none()
            }
            Message::Downloaded(Ok(verified)) => self.downloaded(verified),
            Message::Downloaded(Err(error)) => {
                warn!("The update could not be downloaded: {error}");
                // Only a download that the status is still waiting for may end in a failure, so a late one never
                // overwrites a newer state.
                if matches!(self.updater.status, Status::Downloading { .. }) {
                    self.updater.status = Status::Failed(error.clone());
                }
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
            Message::InstallConfirmed(true) => self.apply(),
            Message::InstallConfirmed(false) => Task::none(),
            Message::Applied(Ok(())) => iced::exit(),
            Message::Applied(Err(error)) => self.apply_refused(error),
        }
    }

    /// A newer release is out. Where this copy can install it, and Auto-update Lodestone is on, the release's file downloads
    /// now and waits for the button. Otherwise the button shows, and its changelog opens the release page.
    fn newer_release(&mut self, release: Release) -> Task<LodestoneAppMessage> {
        if matches!(self.updater.status, Status::Installing(_)) || self.updater.holds(&release.tag) {
            return Task::none();
        }
        if self.updater.installs() && self.settings_values.auto_update {
            return self.download(release, false);
        }
        // The check runs again every half hour, so the release is logged the first time it is seen.
        if self.updater.status != Status::Available(release.clone()) {
            info!("Update {} is available", release.tag);
        }
        self.updater.status = Status::Available(release);
        Task::none()
    }

    /// Downloads the file of `release`, and checks and unpacks it. `install` is set when the changelog's Update button asked
    /// for the install that follows.
    fn download(&mut self, release: Release, install: bool) -> Task<LodestoneAppMessage> {
        let Some(payload) = self.updater.target.payload() else {
            return Task::none();
        };
        info!("Downloading update {}", release.tag);
        self.updater.status = Status::Downloading {
            release: release.clone(),
            install,
        };
        Task::perform(tasks::fetch(release, payload), |result| {
            LodestoneAppMessage::Updater(Message::Downloaded(result))
        })
    }

    /// The file is downloaded and checked. It waits for the button, unless the user already asked for the install.
    fn downloaded(&mut self, verified: Verified) -> Task<LodestoneAppMessage> {
        let install = match &self.updater.status {
            Status::Downloading { release, install } if release.tag == verified.release.tag => *install,
            _ => return Task::none(),
        };
        info!("Update {} is ready to install", verified.release.tag);
        self.updater.status = Status::Ready(verified);
        if install { self.install_now() } else { Task::none() }
    }

    /// The button was pressed. A newer release opens its changelog, where Update starts the download. A downloaded one is
    /// installed, after a question when something is still running.
    fn pressed(&mut self) -> Task<LodestoneAppMessage> {
        match self.updater.status.clone() {
            Status::Available(release) => {
                info!("Showing the changelog of {}", release.tag);
                self.updater.dialog = Some(markdown::Content::parse(&release.body));
                Task::none()
            }
            Status::Ready(_) => self.install_now(),
            _ => Task::none(),
        }
    }

    /// The changelog's Update button. A copy that can install downloads the release and installs it. Any other copy opens
    /// the release page.
    fn confirm(&mut self) -> Task<LodestoneAppMessage> {
        self.updater.dialog = None;
        match self.updater.status.clone() {
            Status::Available(release) if self.updater.installs() => self.download(release, true),
            Status::Available(release) => super::instances::open_source(release.html_url),
            _ => Task::none(),
        }
    }

    /// Installs the downloaded update now, or asks first when closing the launcher would stop something.
    fn install_now(&mut self) -> Task<LodestoneAppMessage> {
        if !self.busy() {
            return self.apply();
        }
        self.alerts
            .show(
                Alert::warn(lang::t("updater.confirm.message"))
                    .title(lang::t("updater.confirm.title"))
                    .yes_no(),
            )
            .map(|install| LodestoneAppMessage::Updater(Message::InstallConfirmed(install)))
    }

    /// Whether closing the launcher would stop something: a game that is starting or running, or a download, an install or
    /// an export. The update waits for none of them on its own, so the question is asked first.
    fn busy(&self) -> bool {
        self.games.any_active() || self.downloads.has_work()
    }

    /// Puts the downloaded update in place, off the UI thread. `Applied` says how it went; on success the launcher exits.
    fn apply(&mut self) -> Task<LodestoneAppMessage> {
        let Status::Ready(verified) = self.updater.status.clone() else {
            return Task::none();
        };
        info!("Installing update {}", verified.release.tag);
        let target = self.updater.target.clone();
        let payload = verified.payload.clone();
        self.updater.status = Status::Installing(verified);
        Task::perform(tasks::apply(target, payload), |result| {
            LodestoneAppMessage::Updater(Message::Applied(result))
        })
    }

    /// The update could not be put in place. It stays ready, so the button tries again.
    fn apply_refused(&mut self, error: String) -> Task<LodestoneAppMessage> {
        if let Status::Installing(verified) = self.updater.status.clone() {
            self.updater.status = Status::Ready(verified);
        }
        warn!("The update did not start: {error}");
        self.alerts
            .show(Alert::warn(lang::t_with("updater.not_started", &[("error", error.as_str())])))
            .discard()
    }

    /// The clock asked for a check. It runs when the launcher may hear of a newer release, and the updates are off for a
    /// portable launcher and a debug build as they are at launch.
    fn check_due(&mut self) -> Task<LodestoneAppMessage> {
        if !enabled(self.portable.is_some()) || !self.updater.awaiting_check() {
            return Task::none();
        }
        check(self.settings_values.beta_channel, self.updater.etag.clone())
    }
}

#[cfg(test)]
#[path = "../../../tests/app/updater/mod.rs"]
mod tests;
