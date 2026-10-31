mod auth;
mod boot;
mod clipboard;
pub mod components;
pub mod database;
mod downloads;
pub mod face;
mod games;
mod install;
mod instances;
mod lang;
mod launch;
mod message;
mod new_instance;
pub mod pages;
pub mod paths;
pub mod perf;
mod preferences;
pub mod router;
mod screenshot_actions;
pub mod storage;
mod subscription;
pub mod theme;
mod titlebar;
mod update;
mod view;
mod world_actions;

use crate::app::components::Alerts;
use crate::app::database::Database;
use crate::app::pages::instance::InstancePage;
use crate::app::pages::library::Library;
use crate::app::pages::login::Login;
use crate::app::pages::settings::Settings;
use crate::app::pages::showcase::Showcase;
use crate::app::paths::LauncherPaths;
use crate::app::router::Router;
use crate::app::storage::{CredentialWriter, StoredCredentials};
use crate::app::titlebar::Titlebar;
use iced::Size;
use iced::widget::image;
use iced::window::Id;
use lodestone_minecraft_authentication::{AuthProbe, Authentication, Session};
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
    settings: Settings,
    showcase: Showcase,
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
            settings: Settings,
            showcase: Showcase::default(),
            login: Login::default(),
            db: None,
            paths: LauncherPaths::default(),
            window_size: Size::new(1280.0, 720.0),
            account: None,
            credentials: StoredCredentials::load(),
            credential_writer: CredentialWriter::default(),
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
