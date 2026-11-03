//! The portable launcher. A launcher built with a package in its file (see `lodestone_export::portable`) boots here
//! instead of the library. It waits for a sign-in, installs the package's instance the first time, adds the package's
//! own files to it, and launches the game. Later runs find the instance installed and go straight to the launch.

use std::io::Read;
use std::path::PathBuf;
use std::sync::OnceLock;

use iced::widget::{Space, Text, column, container};
use iced::{Element, Fill, Length, Task, alignment};
use lodestone_export::format::lodestone::{self, Manifest};
use lodestone_export::portable::{self as package, Located};
use tracing::{info, warn};

use super::{LodestoneAppMessage, LodestoneLauncher};
use crate::app::components::{ProgressBar, TextButton};
use crate::app::database::Database;
use crate::app::discover::resolve_curseforge_files;
use crate::app::install::InstallRequest;
use crate::app::install::package as install_package;
use crate::app::lang;
use crate::app::pages::library::instance::{Instance, Loader};
use crate::app::paths::{self, LauncherPaths};
use crate::app::theme::{color, fonts};

/// The width of the column the page's content sits in.
const COLUMN_WIDTH: f32 = 420.0;

/// The package this launcher carries. It is set once, before the window opens.
static PACKAGE: OnceLock<Portable> = OnceLock::new();

/// The package this launcher carries, and how its launch has got to.
#[derive(Debug, Clone)]
pub struct Portable {
    /// The package's manifest: the instance's name, its versions, and its files.
    pub manifest: Manifest,
    /// Where the package is, in this launcher's file or beside it.
    pub located: Located,
    /// This launcher's own file, which the package is read from.
    pub executable: PathBuf,
    /// Where the launch has got to.
    pub phase: Phase,
    /// The instance the package installs, once it is known.
    pub instance: Option<String>,
    /// The instance's folder, once it is known.
    pub folder: Option<PathBuf>,
    /// The install's job in the downloads queue, while it runs.
    pub job: Option<u64>,
}

/// Where a portable launch has got to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Phase {
    /// Waiting for a sign-in, or for the database to open.
    Waiting,
    /// Looking for the instance the package installs.
    Checking,
    /// The instance's files are downloading.
    Installing,
    /// The package's own files are being added to the instance.
    Extracting,
    /// The game has been asked to start, or it is running.
    Launched,
    /// The launch stopped for this reason.
    Failed(String),
}

/// The portable launcher's messages.
#[derive(Debug, Clone)]
pub enum Message {
    /// Closes the launcher. A game that is running keeps running.
    Close,
    /// Starts the launch again after it stopped.
    Retry,
    /// Starts the game again after it closed.
    Play,
    /// The check for the instance answered.
    Prepared(Result<Prepared, String>),
    /// The package's own files were added to the instance, or could not be.
    Extracted(Result<(), String>),
}

/// What the check found: the instance, already installed, or the request that installs it.
#[derive(Debug, Clone)]
pub enum Prepared {
    Installed(Box<Instance>),
    Install { request: Box<InstallRequest>, folder: PathBuf },
}

/// Looks for a package in this launcher's file, or beside it in a macOS app bundle. A launcher that carries one takes
/// the portable roots, and its window opens as a portable launcher. A package that cannot be read is logged, and the
/// launcher opens as usual.
pub fn prepare() {
    let Ok(executable) = std::env::current_exe() else {
        return;
    };
    let located = match package::locate(&executable) {
        Ok(Some(located)) => located,
        Ok(None) => return,
        Err(error) => {
            warn!("could not look for a package in the launcher: {error}");
            return;
        }
    };
    let manifest = match package::open(&executable, &located)
        .map_err(|error| error.to_string())
        .and_then(|reader| lodestone::read_manifest(reader).map_err(|error| error.to_string()))
    {
        Ok(manifest) => manifest,
        Err(error) => {
            warn!("the launcher's package cannot be read: {error}");
            return;
        }
    };
    info!("Starting the portable launcher for {}", manifest.name);
    paths::use_portable_roots(&manifest.id);
    let _ = PACKAGE.set(Portable {
        manifest,
        located,
        executable,
        phase: Phase::Waiting,
        instance: None,
        folder: None,
        job: None,
    });
}

/// The portable launcher this window opens as, when the launcher carries a package.
pub fn launch() -> Option<Portable> {
    PACKAGE.get().cloned()
}

impl LodestoneLauncher {
    /// Whether the portable launcher is waiting for a sign-in. The sign-in page covers the window until one is made.
    pub(super) fn portable_signing_in(&self) -> bool {
        self.portable.is_some() && self.account.is_none()
    }

    /// Enters the launcher after a sign-in. A portable launcher starts its launch; the others open the library.
    pub(super) fn enter_launcher(&mut self) -> Task<LodestoneAppMessage> {
        // Every launcher leaves the sign-in page. A portable one then shows its own page rather than the library.
        let startup = crate::app::pages::settings::startup_page(self.settings_values.startup_page);
        self.router.replace(startup);
        // The startup page does not go through navigation, so the News home loads its feed here.
        let news = if startup == crate::app::pages::Page::News {
            self.open_news()
        } else {
            Task::none()
        };
        if self.portable.is_some() {
            return Task::batch([news, self.start_portable()]);
        }
        news
    }

    /// Starts a portable launch that was waiting for the database, once someone is signed in.
    pub(super) fn portable_database_ready(&mut self) -> Task<LodestoneAppMessage> {
        if self.account.is_some() { self.start_portable() } else { Task::none() }
    }

    /// Starts the portable launch. It checks whether the package's instance is installed, and then installs it or
    /// launches it. It waits while the database is closed, and does nothing while a launch is already under way.
    fn start_portable(&mut self) -> Task<LodestoneAppMessage> {
        let Some(db) = self.db.clone() else {
            return Task::none();
        };
        let Some(portable) = self.portable.as_mut() else {
            return Task::none();
        };
        if !matches!(portable.phase, Phase::Waiting | Phase::Failed(_)) {
            return Task::none();
        }
        portable.phase = Phase::Checking;
        let manifest = portable.manifest.clone();
        let paths = self.paths.clone();
        Task::perform(check_install(db, paths, manifest), |result| {
            LodestoneAppMessage::Portable(Message::Prepared(result))
        })
    }

    /// Applies a portable message: the check's answer, the install's and the extraction's ends, and the buttons.
    pub(super) fn portable_message(&mut self, message: Message) -> Task<LodestoneAppMessage> {
        match message {
            Message::Close => iced::exit(),
            Message::Retry => self.start_portable(),
            Message::Play => self.launch_portable(),
            Message::Prepared(Ok(Prepared::Installed(instance))) => {
                let id = instance.id.clone();
                self.library.add_instance(*instance);
                self.set_portable(Phase::Launched, Some(id.clone()), None);
                self.launch_game(&id)
            }
            Message::Prepared(Ok(Prepared::Install { request, folder })) => {
                let id = request.id.clone();
                let job = self.downloads.enqueue(*request);
                self.set_portable(Phase::Installing, Some(id), Some(folder));
                if let Some(portable) = self.portable.as_mut() {
                    portable.job = Some(job);
                }
                self.start_ready_downloads()
            }
            Message::Prepared(Err(error)) => {
                self.set_portable(Phase::Failed(error), None, None);
                Task::none()
            }
            Message::Extracted(Ok(())) => {
                self.set_portable(Phase::Launched, None, None);
                match self.portable.as_ref().and_then(|portable| portable.instance.clone()) {
                    Some(id) => self.launch_game(&id),
                    None => Task::none(),
                }
            }
            Message::Extracted(Err(error)) => {
                self.set_portable(Phase::Failed(error), None, None);
                Task::none()
            }
        }
    }

    /// Ends the install of the portable instance when `job` was its job. A finished install has the package's own
    /// files added; a failed one stops the launch. Other jobs are not the portable launcher's, and pass by.
    pub(super) fn portable_install_ended(&mut self, job: u64, outcome: Result<(), String>) -> Task<LodestoneAppMessage> {
        let Some(portable) = self.portable.as_mut() else {
            return Task::none();
        };
        if portable.job != Some(job) {
            return Task::none();
        }
        portable.job = None;
        match (outcome, portable.folder.clone()) {
            (Ok(()), Some(folder)) => {
                portable.phase = Phase::Extracting;
                let (located, executable) = (portable.located.clone(), portable.executable.clone());
                Task::perform(extract(located, executable, folder), |result| {
                    LodestoneAppMessage::Portable(Message::Extracted(result))
                })
            }
            (Ok(()), None) => {
                portable.phase = Phase::Failed(lang::t("portable.no_folder").to_string());
                Task::none()
            }
            (Err(error), _) => {
                portable.phase = Phase::Failed(error);
                Task::none()
            }
        }
    }

    /// Starts the game again, once it has closed.
    fn launch_portable(&mut self) -> Task<LodestoneAppMessage> {
        match self.portable.as_ref().and_then(|portable| portable.instance.clone()) {
            Some(id) => {
                self.set_portable(Phase::Launched, None, None);
                self.launch_game(&id)
            }
            None => Task::none(),
        }
    }

    /// Records the launch's phase, and the instance, folder or nothing else it learned, when given.
    fn set_portable(&mut self, phase: Phase, instance: Option<String>, folder: Option<PathBuf>) {
        if let Some(portable) = self.portable.as_mut() {
            portable.phase = phase;
            if instance.is_some() {
                portable.instance = instance;
            }
            if folder.is_some() {
                portable.folder = folder;
            }
        }
    }

    /// The portable launcher's page: the package's name, what the launch is doing, and the button for what comes next.
    pub(super) fn portable_page(&self) -> Element<'_, LodestoneAppMessage> {
        let Some(portable) = self.portable.as_ref() else {
            return Space::new().into();
        };
        let instance = portable.instance.as_deref();
        let (status, bar, action) = match &portable.phase {
            Phase::Waiting | Phase::Checking => (lang::t("portable.checking").to_owned(), None, None),
            Phase::Installing => (lang::t("portable.installing").to_owned(), self.downloads.summary().fraction, None),
            Phase::Extracting => (lang::t("portable.extracting").to_owned(), None, None),
            Phase::Launched => match instance {
                Some(id) if self.games.is_active(id) => (
                    lang::t("portable.running").to_owned(),
                    Some(1.0),
                    Some((lang::t("common.close"), Message::Close)),
                ),
                Some(id) if self.games.is_preparing(id) => (lang::t("portable.starting").to_owned(), None, None),
                _ => (
                    lang::t("portable.closed").to_owned(),
                    Some(1.0),
                    Some((lang::t("portable.play"), Message::Play)),
                ),
            },
            Phase::Failed(error) => (
                lang::t_with("portable.failed", &[("error", error.as_str())]).to_string(),
                Some(0.0),
                Some((lang::t("portable.retry"), Message::Retry)),
            ),
        };
        let mut content = column![
            Text::new(portable.manifest.name.clone())
                .size(26)
                .font(fonts::Inter::BOLD)
                .color(color::text_headline()),
            Text::new(format!(
                "{} · {}",
                portable.manifest.minecraft,
                Loader::from_db(&portable.manifest.loader.id).label()
            ))
            .size(11)
            .font(fonts::JetBrainsMono::REGULAR)
            .color(color::text_muted()),
            ProgressBar::new(bar.unwrap_or(0.0)).indeterminate(bar.is_none()).width(Fill),
            Text::new(status).size(13).font(fonts::Inter::REGULAR).color(color::text_default()),
        ]
        .spacing(14)
        .align_x(alignment::Horizontal::Center)
        .width(Length::Fixed(COLUMN_WIDTH));
        if let Some((label, message)) = action {
            content = content.push(TextButton::primary(label).on_press(LodestoneAppMessage::Portable(message)));
        }
        container(content).center_x(Fill).center_y(Fill).width(Fill).height(Fill).into()
    }
}

/// Finds whether the package's instance is installed. When it is not, builds the request that installs it, with the
/// CurseForge files resolved to their downloads first.
async fn check_install(db: Database, paths: LauncherPaths, manifest: Manifest) -> Result<Prepared, String> {
    let installed = db.instances().await?.into_iter().find(|instance| instance.id == manifest.id);
    if let Some(mut instance) = installed {
        instance.resolve_files(&paths.instances());
        return Ok(Prepared::Installed(Box::new(instance)));
    }
    let mut payload = install_package::payload(&manifest, None);
    if payload.files.iter().any(|file| file.url.is_empty()) {
        payload.files = resolve_curseforge_files(payload.files).await?;
    }
    let request = install_package::request(&paths, &manifest, payload, manifest.id.clone(), &[]);
    let folder = paths.instance(&request.directory_name);
    Ok(Prepared::Install {
        request: Box::new(request),
        folder,
    })
}

/// Adds the package's own files to the instance folder, on the blocking pool.
async fn extract(located: Located, executable: PathBuf, folder: PathBuf) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let reader: Box<dyn Read + Send> = package::open(&executable, &located).map_err(|error| error.to_string())?;
        lodestone::extract_overrides(reader, &folder).map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}
