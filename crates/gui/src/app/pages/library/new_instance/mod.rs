//! The New Instance modal. It keeps the form's fields, the pickers' choices and the
//! install while one runs. Starting and finishing an install need the database, so the
//! root does those. This module only builds the request for it.

mod banner;
mod banner_editor;
mod form;
mod icon_editor;
pub mod naming;
mod upload;

use std::sync::Arc;

use iced::task::Handle;
use iced::widget::image;
use iced::widget::stack;
use iced::{Element, Task};
use lodestone_loaders::LoaderVersion;

use crate::app::database::Database;
use crate::app::install::catalog::{self, BundledJava, Releases, VersionInfo};
use crate::app::install::{self, IconSource, InstallProgress, InstallRequest};
use crate::app::pages::library::instance::{Instance, InstanceIcon, Loader};
use crate::app::paths::LauncherPaths;

/// An uploaded image: its PNG bytes for the install, and a handle the previews draw from,
/// so a redraw does not copy the bytes again.
#[derive(Debug, Clone)]
pub struct Upload {
    pub png: Vec<u8>,
    pub handle: image::Handle,
}

impl Upload {
    pub fn new(png: Vec<u8>) -> Self {
        let handle = image::Handle::from_bytes(png.clone());
        Self { png, handle }
    }
}

impl PartialEq for Upload {
    fn eq(&self, other: &Self) -> bool {
        self.png == other.png
    }
}

/// The icon the form will save.
#[derive(Debug, Clone, PartialEq)]
pub enum IconChoice {
    /// A background preset and a symbol, by catalog id.
    Composed { background: &'static str, symbol: &'static str },
    /// An uploaded square image.
    Uploaded(Upload),
}

/// The banner the form will save.
#[derive(Debug, Clone, PartialEq)]
pub enum BannerChoice {
    /// One of the scene artwork files, by its index in the picker.
    Scene(usize),
    /// An uploaded image.
    Uploaded(Upload),
}

/// What an install reports: progress along the way, then the result.
#[derive(Debug, Clone)]
pub enum InstallEvent {
    Progress(InstallProgress),
    /// The new instance, boxed to keep the messages small.
    Finished(Result<Box<Instance>, String>),
}

/// The dialog's messages. The root takes the ones that start, finish or cancel an install.
#[derive(Debug, Clone)]
pub enum Message {
    /// The ✕ or the Cancel button.
    Close,
    NameChanged(String),
    ReleasesLoaded(Result<Arc<Releases>, String>),
    MinecraftPicked(usize),
    VersionInfoLoaded {
        mc: String,
        result: Result<VersionInfo, String>,
    },
    BundledLoaded(Result<Vec<BundledJava>, String>),
    JavaPicked(usize),
    LoaderPicked(Loader),
    BuildsLoaded {
        loader: Loader,
        mc: String,
        result: Result<Vec<LoaderVersion>, String>,
    },
    BuildPicked(usize),
    RecentsLoaded(Result<Vec<InstanceIcon>, String>),
    OpenIconEditor,
    IconEditor(icon_editor::Message),
    /// The banner strip was pressed: open the banner editor.
    OpenBannerEditor,
    BannerEditor(banner_editor::Message),
    /// A banner was read from disk, or the dialog was cancelled.
    BannerUploaded(Result<Option<Vec<u8>>, String>),
    /// An icon was read from disk, or the dialog was cancelled.
    IconUploaded(Result<Option<Vec<u8>>, String>),
    /// Create the instance. The root starts the install.
    Submit,
    Install(InstallEvent),
    /// Cancel the running install, after the user confirms.
    CancelInstall,
    CancelAnswered(bool),
}

/// An install in progress: its latest report, and the handle that cancels it.
pub struct Installing {
    progress: Option<InstallProgress>,
    abort: Handle,
}

/// The dialog's state.
pub struct NewInstance {
    paths: LauncherPaths,
    name: String,
    releases: Option<Arc<Releases>>,
    releases_error: Option<String>,
    mc_version: Option<String>,
    version_info: Option<VersionInfo>,
    bundled: Vec<BundledJava>,
    /// The Java runtime component picked by hand. `None` follows the Minecraft version.
    java_override: Option<String>,
    loader: Loader,
    builds: Option<Vec<LoaderVersion>>,
    loader_version: Option<String>,
    recents: Vec<InstanceIcon>,
    icon: IconChoice,
    icon_editor: Option<icon_editor::IconEditor>,
    banner_editor: Option<banner_editor::BannerEditor>,
    banner: BannerChoice,
    error: Option<String>,
    installing: Option<Installing>,
}

impl std::fmt::Debug for NewInstance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NewInstance").field("name", &self.name).finish_non_exhaustive()
    }
}

impl NewInstance {
    /// A dialog with a random icon and banner, Fabric chosen, and nothing loaded yet.
    pub fn new(paths: LauncherPaths) -> Self {
        let (background, symbol) = crate::app::components::instance_icon::random();
        Self {
            paths,
            name: String::new(),
            releases: None,
            releases_error: None,
            mc_version: None,
            version_info: None,
            bundled: Vec::new(),
            java_override: None,
            loader: Loader::Fabric,
            builds: None,
            loader_version: None,
            recents: Vec::new(),
            icon: IconChoice::Composed { background, symbol },
            icon_editor: None,
            banner_editor: None,
            banner: BannerChoice::Scene(banner::random_scene()),
            error: None,
            installing: None,
        }
    }

    /// The tasks a freshly opened dialog starts: Mojang's versions and the bundled runtimes.
    pub fn load(&self) -> Task<Message> {
        Task::batch([
            Task::perform(catalog::releases(), |result| Message::ReleasesLoaded(result.map(Arc::new))),
            Task::perform(catalog::bundled_java(), Message::BundledLoaded),
        ])
    }

    /// Whether an install is running.
    pub fn is_installing(&self) -> bool {
        self.installing.is_some()
    }

    /// Whether the form has what it needs to create an instance. The check does no disk
    /// work, since the view calls it on every redraw.
    pub fn ready(&self) -> bool {
        !self.name.trim().is_empty()
            && self.mc_version.is_some()
            && self.version_info.is_some()
            && (self.loader == Loader::Vanilla || self.loader_version.is_some())
            && !self.is_installing()
    }

    /// Applies a message. The messages the root takes are ignored here.
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::NameChanged(name) => {
                self.name = name;
                self.error = None;
                Task::none()
            }
            Message::ReleasesLoaded(Ok(releases)) => {
                let latest = releases
                    .latest
                    .clone()
                    .filter(|id| releases.ids.contains(id))
                    .or_else(|| releases.ids.first().cloned());
                self.releases_error = None;
                self.releases = Some(releases);
                match latest {
                    Some(mc) => self.select_version(mc),
                    None => Task::none(),
                }
            }
            Message::ReleasesLoaded(Err(error)) => {
                self.releases_error = Some(error);
                Task::none()
            }
            Message::MinecraftPicked(index) => {
                let picked = self.releases.as_ref().and_then(|releases| releases.ids.get(index).cloned());
                match picked {
                    Some(mc) => self.select_version(mc),
                    None => Task::none(),
                }
            }
            Message::VersionInfoLoaded { mc, result } => {
                if self.mc_version.as_deref() == Some(mc.as_str()) {
                    match result {
                        Ok(info) => self.version_info = Some(info),
                        Err(error) => self.error = Some(error),
                    }
                }
                Task::none()
            }
            Message::BundledLoaded(Ok(bundled)) => {
                self.bundled = bundled;
                Task::none()
            }
            Message::BundledLoaded(Err(error)) => {
                tracing::warn!("bundled Java runtimes unavailable: {error}");
                Task::none()
            }
            Message::JavaPicked(index) => {
                self.java_override = index
                    .checked_sub(1)
                    .and_then(|at| self.bundled.get(at))
                    .map(|java| java.component.clone());
                Task::none()
            }
            Message::LoaderPicked(loader) => {
                self.loader = loader;
                self.builds = None;
                self.loader_version = None;
                self.fetch_builds()
            }
            Message::BuildsLoaded { loader, mc, result } => {
                if self.loader == loader && self.mc_version.as_deref() == Some(mc.as_str()) {
                    match result {
                        Ok(builds) => {
                            self.loader_version = builds
                                .iter()
                                .find(|build| build.stable)
                                .or_else(|| builds.first())
                                .map(|build| build.version.clone());
                            self.builds = Some(builds);
                        }
                        Err(error) => {
                            self.builds = Some(Vec::new());
                            self.error = Some(error);
                        }
                    }
                }
                Task::none()
            }
            Message::BuildPicked(index) => {
                self.loader_version = self
                    .builds
                    .as_ref()
                    .and_then(|builds| builds.get(index))
                    .map(|build| build.version.clone());
                Task::none()
            }
            Message::RecentsLoaded(Ok(recents)) => {
                self.recents = recents;
                Task::none()
            }
            Message::RecentsLoaded(Err(error)) => {
                tracing::warn!("icon recents unavailable: {error}");
                Task::none()
            }
            Message::OpenIconEditor => {
                self.icon_editor = Some(icon_editor::IconEditor::new(&self.icon, self.recents.clone()));
                Task::none()
            }
            Message::IconEditor(message) => {
                let Some(editor) = &mut self.icon_editor else {
                    return Task::none();
                };
                // A new symbol may sit in a row the grid is not showing, so bring its row into view.
                let reveal = matches!(message, icon_editor::Message::Randomize | icon_editor::Message::RecentPicked(_));
                match editor.update(message) {
                    icon_editor::Outcome::Stay if reveal => editor.reveal_symbol(),
                    icon_editor::Outcome::Stay => Task::none(),
                    icon_editor::Outcome::Upload => Task::perform(upload::pick_icon(), Message::IconUploaded),
                    icon_editor::Outcome::Save(choice) => {
                        self.icon = choice;
                        self.icon_editor = None;
                        Task::none()
                    }
                    icon_editor::Outcome::Cancel => {
                        self.icon_editor = None;
                        Task::none()
                    }
                }
            }
            Message::IconUploaded(Ok(Some(png))) => {
                let upload = Upload::new(png);
                match &mut self.icon_editor {
                    Some(editor) => editor.set_upload(upload),
                    None => self.icon = IconChoice::Uploaded(upload),
                }
                Task::none()
            }
            Message::IconUploaded(Ok(None)) => Task::none(),
            Message::IconUploaded(Err(error)) => {
                self.error = Some(error);
                Task::none()
            }
            Message::OpenBannerEditor => {
                self.banner_editor = Some(banner_editor::BannerEditor::new(&self.banner));
                Task::none()
            }
            Message::BannerEditor(message) => {
                let Some(editor) = &mut self.banner_editor else {
                    return Task::none();
                };
                match editor.update(message) {
                    banner_editor::Outcome::Stay => Task::none(),
                    banner_editor::Outcome::Upload => Task::perform(upload::pick_banner(), Message::BannerUploaded),
                    banner_editor::Outcome::Save(choice) => {
                        self.banner = choice;
                        self.banner_editor = None;
                        Task::none()
                    }
                    banner_editor::Outcome::Cancel => {
                        self.banner_editor = None;
                        Task::none()
                    }
                }
            }
            Message::BannerUploaded(Ok(Some(png))) => {
                let upload = Upload::new(png);
                match &mut self.banner_editor {
                    Some(editor) => editor.set_upload(upload),
                    None => self.banner = BannerChoice::Uploaded(upload),
                }
                Task::none()
            }
            Message::BannerUploaded(Ok(None)) => Task::none(),
            Message::BannerUploaded(Err(error)) => {
                self.error = Some(error);
                Task::none()
            }
            Message::Install(InstallEvent::Progress(progress)) => {
                if let Some(installing) = &mut self.installing {
                    installing.progress = Some(progress);
                }
                Task::none()
            }
            Message::Close | Message::Submit | Message::Install(InstallEvent::Finished(_)) | Message::CancelInstall | Message::CancelAnswered(_) => {
                Task::none()
            }
        }
    }

    /// Chooses a Minecraft version and loads what depends on it: the version's Java
    /// requirement and the loader's builds.
    fn select_version(&mut self, mc: String) -> Task<Message> {
        self.mc_version = Some(mc.clone());
        self.version_info = None;
        self.builds = None;
        self.loader_version = None;
        let mut tasks = Vec::with_capacity(2);
        if let Some(releases) = self.releases.clone() {
            let query = mc.clone();
            tasks.push(Task::perform(
                async move { catalog::version_info(&releases, &query).await },
                move |result| Message::VersionInfoLoaded { mc, result },
            ));
        }
        tasks.push(self.fetch_builds());
        Task::batch(tasks)
    }

    /// Loads the builds of the chosen loader for the chosen version. Vanilla has none.
    fn fetch_builds(&self) -> Task<Message> {
        let Some(mc) = self.mc_version.clone() else {
            return Task::none();
        };
        if self.loader == Loader::Vanilla {
            return Task::none();
        }
        let loader = self.loader;
        let query = mc.clone();
        Task::perform(async move { catalog::loader_builds(loader, &query).await }, move |result| {
            Message::BuildsLoaded { loader, mc, result }
        })
    }

    /// Validates the form and starts the install. The returned task runs it, and the
    /// cancel handle is kept here.
    pub fn start_install(&mut self, db: Database) -> Result<Task<Message>, String> {
        let request = self.install_request()?;
        let (task, abort) = Task::sip(
            install::install(request, db),
            |progress| Message::Install(InstallEvent::Progress(progress)),
            |result| Message::Install(InstallEvent::Finished(result.map(Box::new))),
        )
        .abortable();
        self.error = None;
        self.installing = Some(Installing { progress: None, abort });
        Ok(task)
    }

    /// Stops a running install. Its sipper is dropped, which removes the partial folder.
    pub fn cancel_install(&mut self) {
        if let Some(installing) = self.installing.take() {
            installing.abort.abort();
        }
    }

    /// Records why the install stopped, and leaves the dialog open to try again.
    pub fn install_failed(&mut self, error: String) {
        self.installing = None;
        self.error = Some(error);
    }

    /// Builds the install's request from the form. It reserves the instance's folder name,
    /// so call it once per install.
    fn install_request(&self) -> Result<InstallRequest, String> {
        let name = self.name.trim();
        if name.is_empty() {
            return Err("Give the instance a name.".to_string());
        }
        let mc = self.mc_version.clone().ok_or("Choose a Minecraft version.")?;
        let info = self.version_info.as_ref().ok_or("Minecraft's version details are still loading.")?;
        let loader_version = match self.loader {
            Loader::Vanilla => None,
            _ => Some(self.loader_version.clone().ok_or("Choose a loader version.")?),
        };
        let java_component = Some(self.java_override.clone().unwrap_or_else(|| info.component.clone()));
        let directory_name = naming::unique_directory_name(&self.paths.instances(), &naming::clean_directory_name(name));
        let icon = match &self.icon {
            IconChoice::Composed { background, symbol } => IconSource::Composed {
                background: background.to_string(),
                symbol: symbol.to_string(),
            },
            IconChoice::Uploaded(upload) => IconSource::Uploaded(upload.png.clone()),
        };
        let (biome, art_seed, banner) = match &self.banner {
            BannerChoice::Scene(index) => {
                let (biome, seed) = banner::scene(*index);
                (biome, seed, None)
            }
            BannerChoice::Uploaded(upload) => {
                let (biome, seed) = banner::scene(0);
                (biome, seed, Some(upload.png.clone()))
            }
        };
        Ok(InstallRequest {
            paths: self.paths.clone(),
            name: name.to_string(),
            directory_name,
            mc_version: mc,
            loader: self.loader,
            loader_version,
            java_component,
            icon,
            biome,
            art_seed,
            banner,
        })
    }

    /// The dialog and, over it, whichever editor is open.
    pub fn view(&self) -> Element<'_, Message> {
        let dialog = form::modal(self);
        if let Some(editor) = &self.icon_editor {
            return stack![dialog, icon_editor::modal(editor).map(Message::IconEditor)].into();
        }
        match &self.banner_editor {
            Some(editor) => stack![dialog, banner_editor::modal(editor).map(Message::BannerEditor)].into(),
            None => dialog,
        }
    }

    /// Asks the database for the icon recents when the dialog opens.
    pub fn recents(db: Database) -> Task<Message> {
        Task::perform(async move { db.recent_icons().await }, Message::RecentsLoaded)
    }
}

#[cfg(test)]
#[path = "../../../../../tests/app/pages/library/new_instance/mod.rs"]
mod tests;
