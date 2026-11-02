mod auth;
mod boot;
mod clipboard;
pub mod components;
pub mod database;
mod discover;
mod downloads;
mod export;
pub mod face;
mod games;
mod install;
mod instances;
mod lang;
mod launch;
pub mod launch_settings;
mod message;
mod mod_actions;
mod mod_sources;
mod new_instance;
pub mod pages;
pub mod paths;
pub mod perf;
pub mod portable;
mod preferences;
mod resource;
pub mod router;
mod screenshot_actions;
mod shared_library;
pub mod storage;
mod subscription;
pub mod theme;
mod titlebar;
mod update;
mod updater;
mod view;
mod world_actions;
mod world_pins;
pub mod writes;

use crate::app::components::Alerts;
use crate::app::database::Database;
use crate::app::instances::InstanceSave;
use crate::app::launch_settings::LaunchDefaults;
use crate::app::pages::discover::Discover;
use crate::app::pages::instance::InstancePage;
use crate::app::pages::library::Library;
use crate::app::pages::login::Login;
use crate::app::pages::resource::ResourcePage;
use crate::app::pages::settings::Settings;
use crate::app::pages::showcase::Showcase;
use crate::app::paths::LauncherPaths;
use crate::app::router::Router;
use crate::app::storage::{CredentialWriter, StoredCredentials};
use crate::app::titlebar::Titlebar;
use crate::app::writes::KeyedWriter;
use iced::Size;
use iced::widget::image;
use iced::window::Id;
use lodestone_minecraft_authentication::{AuthProbe, Authentication, Session};
use std::collections::HashMap;
use std::sync::OnceLock;

pub use message::{LodestoneAppMessage, NavGroup};

/// TEMPORARY resize-latency probe — active only with
/// `LODESTONE_RESIZE_PROFILE=1`; drives `window::resize` through a fixed
/// size sequence for render-latency measurement, and logs the timeline
/// (request → compositor Resized event → first frame at the new size).
pub(crate) fn probe_enabled() -> bool {
    static PROBE: OnceLock<bool> = OnceLock::new();
    *PROBE.get_or_init(|| std::env::var_os("LODESTONE_RESIZE_PROFILE").is_some())
}

/// The Azure app registration backing Microsoft sign-in (public client,
/// personal accounts, device-code flow).
pub(crate) const AZURE_CLIENT_ID: &str = "b1156b7e-e0fe-4f86-a835-afed728d093d";

/// The kind of session the launcher is running under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Account {
    /// Signed in through Microsoft — full online capabilities.
    Microsoft { session: Session },
    /// Offline mode — no Microsoft account; the username is self-declared.
    Offline { username: String },
}

impl Account {
    /// The name this account plays under.
    pub fn username(&self) -> &str {
        match self {
            Account::Microsoft { session } => &session.profile.name,
            Account::Offline { username } => username,
        }
    }
}

/// The launcher's root state.
///
/// Not `#[derive(Default)]` because the auth handle needs the client id.
pub struct LodestoneLauncher {
    id: Option<Id>,
    titlebar: Titlebar,
    maximized: bool,
    router: Router,
    library: Library,
    instance_page: InstancePage,
    /// When each instance's mods were last checked against their platforms, and what the check found, for the session.
    mod_checks: HashMap<String, mod_actions::ModCheck>,
    settings: Settings,
    showcase: Showcase,
    /// The Discover page: the platform searches, and the results on screen.
    discover: Discover,
    /// The resource page: the project it shows, and what it has loaded of it.
    resource: ResourcePage,
    /// The Worlds page: every instance's saves, as the last scan read them.
    worlds: crate::app::pages::worlds::State,
    /// The worlds the player pinned. The database keeps them; the instance and Worlds pages read them.
    world_pins: world_pins::WorldPins,
    login: Login,
    /// The launcher database — `None` until the boot-time connect lands.
    db: Option<Database>,
    /// Where the launcher keeps its folders: instances, and the shared Java
    /// and asset downloads.
    paths: LauncherPaths,
    /// The window's inner size — the Library's virtualized list uses it
    /// for the grid's column count and the windowed row count.
    window_size: Size,
    /// The active account, once someone has entered the launcher.
    account: Option<Account>,
    /// The at-rest credentials loaded from disk and rewritten on every
    /// sign-in.
    credentials: StoredCredentials,
    /// Keeps the background credential writes in order (see `auth.rs`).
    credential_writer: CredentialWriter,
    /// The launch settings every instance inherits until it overrides one. Loaded with the preferences at boot.
    launch_defaults: LaunchDefaults,
    /// Keeps the saves of each instance's row in order, one write in flight per instance.
    instance_saves: KeyedWriter<String, InstanceSave>,
    /// Keeps the saves of the global defaults in order.
    defaults_saves: KeyedWriter<&'static str, LaunchDefaults>,
    auth: Authentication,
    /// Bumped on every new/abandoned sign-in so results from stale polls
    /// (cancelled or superseded) are dropped when they arrive.
    auth_generation: u64,
    /// The player face texture for the sidebar avatar — the real skin face
    /// for Microsoft accounts, the vanilla Steve for offline/demo. `None`
    /// renders the username-initial fallback.
    face: Option<image::Handle>,
    /// The last auth-services probe: `Ok` reached it (with delay + region),
    /// `Err` did not. `None` until the first probe lands.
    auth_status: Option<Result<AuthProbe, String>>,
    /// The alert queue; the front alert covers the window (see `view.rs`).
    alerts: Alerts,
    /// The clipboard that copied images go to. Its thread starts on the first copy (see `clipboard.rs`).
    clipboard: clipboard::Worker,
    /// The game sessions: the instances being prepared and the games running.
    games: launch::games::Games,
    /// Which panels of running games are open, beside the sidebar and under the title bar.
    runs_open: launch::games::RunsOpen,
    /// The installs queued, running, paused and finished, which the Downloads popover lists.
    downloads: downloads::Downloads,
    /// The export dialog, while it is open. It sits above the page, beside the alerts.
    export: Option<export::Dialog>,
    /// The package this launcher carries, when it is a portable launcher. Its launch replaces the library.
    portable: Option<portable::Portable>,
    /// The update check: the newest release, and what the update is doing. See `updater`.
    updater: updater::Updater,
    /// The library's scene art, rendered to pixels in the background.
    art: components::art_cache::ArtCache,
    /// TEMPORARY resize-latency probe counter (env-gated).
    probe_step: u32,
    /// The F3 performance overlay. It shows nothing unless built with the `perf` feature.
    perf: perf::Hud,
}
impl Default for LodestoneLauncher {
    fn default() -> Self {
        Self {
            id: None,
            titlebar: Titlebar,
            maximized: false,
            router: Router::default(),
            library: Library::default(),
            instance_page: InstancePage::default(),
            mod_checks: HashMap::new(),
            settings: Settings::default(),
            showcase: Showcase::default(),
            discover: Discover::new(discover::opening_source()),
            resource: ResourcePage::default(),
            worlds: crate::app::pages::worlds::State::default(),
            world_pins: world_pins::WorldPins::default(),
            login: Login::default(),
            db: None,
            paths: LauncherPaths::default(),
            window_size: Size::new(1280.0, 720.0),
            account: None,
            credentials: StoredCredentials::load(),
            credential_writer: CredentialWriter::default(),
            launch_defaults: LaunchDefaults::default(),
            instance_saves: KeyedWriter::default(),
            defaults_saves: KeyedWriter::default(),
            auth: Authentication::new(AZURE_CLIENT_ID),
            auth_generation: 0,
            face: None,
            probe_step: 0,
            auth_status: None,
            alerts: Alerts::default(),
            clipboard: clipboard::Worker::default(),
            games: launch::games::Games::default(),
            runs_open: launch::games::RunsOpen::default(),
            downloads: downloads::Downloads::default(),
            export: None,
            portable: portable::launch(),
            updater: updater::Updater::default(),
            art: components::art_cache::ArtCache::default(),
            perf: perf::Hud::default(),
        }
    }
}

impl LodestoneLauncher {
    pub const TITLE: &'static str = "Lodestone Launcher";
    /// The release major and minor versions.
    /// Ex: `0.0.0`
    pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");
    /// A unique build number that is created automatically at compile time.
    /// Ex: `20260827.032580`
    pub const BUILD: &'static str = env!("BUILD");
}
