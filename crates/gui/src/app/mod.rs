mod auth;
mod boot;
mod clipboard;
pub mod components;
mod coop_sync;
mod data_folder;
pub mod database;
mod discover;
mod downloads;
mod export;
pub mod face;
mod games;
mod import;
mod install;
mod instances;
mod lang;
mod launch;
pub mod launch_settings;
mod local_data;
mod message;
mod mod_actions;
mod mod_sources;
mod new_instance;
mod news;
mod notifications;
pub mod pages;
pub mod paths;
pub mod perf;
pub mod portable;
mod preferences;
mod resource;
pub mod router;
mod screenshot_actions;
mod servers;
mod shared_library;
mod skin_actions;
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
use crate::app::pages::discover::Discover;
use crate::app::pages::instance::InstancePage;
use crate::app::pages::library::Library;
use crate::app::pages::login::Login;
use crate::app::pages::resource::ResourcePage;
use crate::app::pages::settings::Settings;
use crate::app::paths::LauncherPaths;
use crate::app::preferences::SettingsValues;
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
    /// The values the Settings page saved: the launch defaults every instance inherits, and the rest of the page. Loaded with the preferences at boot.
    settings_values: SettingsValues,
    /// Keeps the saves of each instance's row in order, one write in flight per instance.
    instance_saves: KeyedWriter<String, InstanceSave>,
    /// Keeps the saves of the Settings page in order.
    settings_saves: KeyedWriter<&'static str, SettingsValues>,
    /// The Minecraft version list fetched at launch, which New Instance opens with. `None` until the fetch lands.
    game_versions: Option<std::sync::Arc<crate::app::install::catalog::Releases>>,
    /// The download speed the Network tab shows, sampled once a second.
    speed: crate::app::pages::settings::network::speed::Meter,
    /// The Microsoft account a restore at launch is signing in, so a failed restore drops it.
    restoring: Option<String>,
    /// Whether the sign-in page was opened to add an account, so a sign-in returns to Settings.
    adding_account: bool,
    /// The Skins page: the skin and cape the account wears, the skins to choose from, and the capes the account owns.
    skins: pages::skins::Skins,
    /// The change to the profile that the player is asked to confirm, until the answer comes back.
    pending_change: Option<skin_actions::ProfileChange>,
    /// The change to the profile that was confirmed while the Microsoft session refreshes, so it goes ahead once the
    /// refresh lands.
    waiting_change: Option<skin_actions::ProfileChange>,
    auth: Authentication,
    /// Bumped on every new/abandoned sign-in so results from stale polls
    /// (cancelled or superseded) are dropped when they arrive.
    auth_generation: u64,
    /// The player face texture for the sidebar avatar — the real skin face
    /// for Microsoft accounts, the vanilla Steve for offline/demo. `None`
    /// renders the username-initial fallback.
    face: Option<image::Handle>,
    /// Whether the account menu on the player's face is open.
    account_menu_open: bool,
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
    /// The bell's notifications: the ones the database keeps, and whether the bell's panel is open.
    notifications: notifications::Notifications,
    /// The News page: the launcher feed, the pictures the page shows, and what it shows of them.
    news: pages::news::State,
    /// The Servers page: the saved servers, their pings, and the games heard on the network.
    servers: pages::servers::State,
    /// The Add server or Edit server dialog, while it is open.
    servers_dialog: Option<servers::dialog::Dialog>,
    /// The Join Server dialog, while it is open.
    join_dialog: Option<servers::join::JoinDialog>,
    /// Whether the Co-op Sync dialog is open.
    coop_sync_open: bool,
    /// The export dialog, while it is open. It sits above the page, beside the alerts.
    export: Option<export::Dialog>,
    /// The Import Instance dialog, while it is open. It sits above the page, beside the export dialog.
    import: Option<import::Dialog>,
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
    /// The folder the data is moving to, while its move runs. The window is covered until it lands.
    data_move: Option<std::path::PathBuf>,
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
            settings_values: SettingsValues::default(),
            instance_saves: KeyedWriter::default(),
            settings_saves: KeyedWriter::default(),
            game_versions: None,
            speed: Default::default(),
            restoring: None,
            adding_account: false,
            skins: pages::skins::Skins::default(),
            pending_change: None,
            waiting_change: None,
            auth: Authentication::new(AZURE_CLIENT_ID),
            auth_generation: 0,
            face: None,
            account_menu_open: false,
            probe_step: 0,
            auth_status: None,
            alerts: Alerts::default(),
            clipboard: clipboard::Worker::default(),
            games: launch::games::Games::default(),
            runs_open: launch::games::RunsOpen::default(),
            downloads: downloads::Downloads::default(),
            notifications: notifications::Notifications::default(),
            news: pages::news::State::default(),
            servers: pages::servers::State::default(),
            servers_dialog: None,
            join_dialog: None,
            coop_sync_open: false,
            export: None,
            import: None,
            portable: portable::launch(),
            updater: updater::Updater::default(),
            art: components::art_cache::ArtCache::default(),
            perf: perf::Hud::default(),
            data_move: None,
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
