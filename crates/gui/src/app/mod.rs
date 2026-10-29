mod auth;
mod boot;
pub mod components;
pub mod database;
pub mod face;
mod message;
pub mod pages;
pub mod router;
pub mod storage;
mod subscription;
pub mod theme;
mod titlebar;
mod update;
mod view;

use crate::app::database::Database;
use crate::app::pages::instance::InstancePage;
use crate::app::pages::library::Library;
use crate::app::pages::login::Login;
use crate::app::pages::settings::Settings;
use crate::app::pages::showcase::Showcase;
use crate::app::router::Router;
use crate::app::storage::{CredentialWriter, StoredCredentials};
use crate::app::titlebar::Titlebar;
use iced::Size;
use iced::widget::image;
use iced::window::Id;
use lodestone_minecraft_authentication::{AuthProbe, Authentication, Session};
use std::path::PathBuf;
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
const AZURE_CLIENT_ID: &str = "b1156b7e-e0fe-4f86-a835-afed728d093d";

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
    /// Where instance folders live (`<data>/lodestone/instances`).
    instances_dir: PathBuf,
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
    /// TEMPORARY resize-latency probe counter (env-gated).
    probe_step: u32,
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
            instances_dir: dirs::data_dir().unwrap_or_else(std::env::temp_dir).join("lodestone").join("instances"),
            window_size: Size::new(1280.0, 720.0),
            account: None,
            credentials: StoredCredentials::load(),
            credential_writer: CredentialWriter::default(),
            auth: Authentication::new(AZURE_CLIENT_ID),
            auth_generation: 0,
            face: None,
            probe_step: 0,
            auth_status: None,
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
