//! The root message type: every event the launcher reacts to, from window
//! chrome and navigation to page messages and background-task results.

use crate::app::components::account_menu::Message as AccountMenuMessage;
use crate::app::components::alert::Message as AlertMessage;
use crate::app::components::art_cache::ArtKey;
use crate::app::coop_sync::Message as CoopSyncMessage;
use crate::app::database::Database;
use crate::app::downloads::Message as DownloadsMessage;
use crate::app::export::Message as ExportMessage;
use crate::app::import::Message as ImportMessage;
use crate::app::launch::GameMessage;
use crate::app::mod_actions;
use crate::app::notifications;
use crate::app::pages::Page;
use crate::app::pages::discover::Message as DiscoverMessage;
use crate::app::pages::instance::Message as InstanceMessage;
use crate::app::pages::library::Message as LibraryMessage;
use crate::app::pages::library::instance::Instance;
use crate::app::pages::login::Message as LoginMessage;
use crate::app::pages::news::Message as NewsMessage;
use crate::app::pages::resource::Message as ResourceMessage;
use crate::app::pages::servers::Message as ServersMessage;
use crate::app::pages::settings::Message as SettingsMessage;
use crate::app::pages::skins::capes::CapeArt;
use crate::app::pages::skins::previews::Drawn;
use crate::app::pages::skins::{Appearance, Message as SkinsMessage, Scan};
use crate::app::pages::worlds::Message as WorldsMessage;
use crate::app::perf;
use crate::app::portable::Message as PortableMessage;
use crate::app::servers::dialog::Message as ServersDialogMessage;
use crate::app::servers::join::Message as JoinDialogMessage;
use crate::app::shared_library::{ModSource, SharedRecord};
use crate::app::titlebar::TitlebarMessage;
use crate::app::updater::Message as UpdaterMessage;
use crate::app::world_actions::WorldMessage;
use iced::Size;
use iced::widget::image::Handle;
use iced::window::{Direction, Id};
use lodestone_minecraft_authentication::{AuthProbe, Profile, Session};

#[derive(Debug, Clone)]
pub enum LodestoneAppMessage {
    WindowCreation(Option<Id>),
    WindowStartResize(Direction),
    WindowSizeChanged(Id, Size),
    TitlebarMessage(TitlebarMessage),
    WindowMaximized(bool),
    /// Pushes `page` onto the router's history — the `useNavigate` of this
    /// framework. Produced by the rail and by any page's `Navigate(Page)`.
    Navigate(Page),
    /// Steps back through the router's history.
    NavBack,
    /// Steps forward through the router's history.
    NavForward,
    Login(LoginMessage),
    Library(LibraryMessage),
    /// The Discover page: a search, a filter or a layout change, and the answers the root sends back.
    Discover(DiscoverMessage),
    /// The Worlds page: its search, filters and menus, and the saves it reads.
    Worlds(WorldsMessage),
    /// A world's request or result, from the Worlds page or the instance's Worlds tab.
    World(WorldMessage),
    /// The pinned worlds finished loading from the database, as (instance id, save folder) rows.
    WorldPinsLoaded(Result<Vec<(String, String)>, String>),
    /// The resource page: its tabs, its answers, and its actions.
    Resource(ResourceMessage),
    Instance(InstanceMessage),
    Settings(SettingsMessage),
    /// The download queue: an install's progress or result, and the Downloads popover's controls.
    Downloads(DownloadsMessage),
    /// The notifications the database keeps: the bell's panel, and the reads and writes that answer it.
    Notifications(notifications::Message),
    /// The News page: its refresh, its lists and its reader, and the feed and pictures its work brings back.
    News(NewsMessage),
    /// The Servers page: its tabs and search, its pings and the games heard on the network, and the row actions.
    Servers(ServersMessage),
    /// The Add server or Edit server dialog.
    ServersDialog(ServersDialogMessage),
    /// The Join Server dialog.
    JoinDialog(JoinDialogMessage),
    /// The Co-op Sync dialog.
    CoopSync(CoopSyncMessage),
    /// The export dialog: its fields, its folder listing, and the export it starts.
    Export(ExportMessage),
    /// The Import Instance dialog: its tabs and fields, the files it is given, and the import it reads and queues.
    Import(ImportMessage),
    /// The portable launcher: its check, its install and its buttons.
    Portable(PortableMessage),
    /// The update check: the release check's answer, the installer's download, and the update button.
    Updater(UpdaterMessage),
    /// One second has passed, so the Network tab takes a speed sample.
    SpeedTick,
    /// A switch to another account, which signed in again, finished.
    AccountSwitched(usize, Result<Session, String>),
    /// The account menu on the player's face: its toggle, its dismissal, a switch to an account, and the add.
    AccountMenu(AccountMenuMessage),
    /// The answer to the question whether to make the change to the profile that was asked for: a skin or a cape.
    ProfileChangeConfirmed(bool),
    /// A change to the profile finished, with the profile it leaves.
    ProfileChanged(Result<Profile, String>),
    /// The Skins page's own messages, and the requests it makes of the root.
    Skins(SkinsMessage),
    /// The skin folders were read.
    SkinsLoaded(Scan),
    /// The capes the account owns were read, each with its front and its picture.
    CapesLoaded(Vec<CapeArt>),
    /// A drawing of the skin previews finished, with the frame it made.
    PreviewsReady(Drawn),
    /// The profile was read again when the Skins page opened.
    ProfileRefreshed(Result<Profile, String>),
    /// A skin file was moved to the trash, or could not be.
    SkinFileMoved(Result<(), String>),
    /// The answer to the question whether to clear the saved settings and the sign-ins.
    LocalDataAnswered(bool),
    /// The Minecraft version list fetched at launch, when Auto-update game versions is on.
    VersionsPrefetched(Result<std::sync::Arc<crate::app::install::catalog::Releases>, String>),
    /// The launcher database finished opening (schema migrations ran).
    DatabaseReady(Result<Database, String>),
    /// The instance list finished loading, with each mod count read from disk.
    InstancesLoaded(Result<Vec<Instance>, String>),
    /// The downloads the Discover page made into the shared folders, as they were recorded.
    SharedLoaded(Result<Vec<SharedRecord>, String>),
    /// The mods the Discover page added to instances, as they were recorded.
    ModSourcesLoaded(Result<Vec<ModSource>, String>),
    /// What the platforms and the disk answered for the Mods tab.
    ModActions(mod_actions::Message),
    /// The delete alert was answered for `instance`.
    InstanceDeleteAnswered {
        instance: String,
        confirmed: bool,
    },
    /// The instance's folder and row were removed, or the removal failed.
    InstanceDeleted {
        instance: String,
        result: Result<(), String>,
    },
    /// An instance's edited row was written, or the write failed.
    InstanceSaved {
        instance: String,
        result: Result<(), String>,
    },
    /// The reset alert was answered for `instance`. A reset goes ahead only when it is confirmed.
    InstanceResetAnswered {
        instance: String,
        confirmed: bool,
    },
    /// The global defaults were written to the preferences, or the write failed.
    SettingsSaved(Result<(), String>),
    /// The folder picker answered, with the folder picked. A cancel sends `None`.
    DataFolderPicked(Option<std::path::PathBuf>),
    /// The question whether to move the launcher's data to `folder` was answered.
    DataFolderAnswered {
        folder: std::path::PathBuf,
        confirmed: bool,
    },
    /// The launcher's data moved to `folder`, or the move failed, and the data stayed where it was.
    DataFolderMoved {
        folder: std::path::PathBuf,
        result: Result<(), String>,
    },
    /// A reinstall of a version's game files finished, or failed. `job` is its queue entry.
    ReinstallFinished {
        job: u64,
        result: Result<(), String>,
    },
    /// An instance folder was opened in the file manager, or could not be.
    FolderOpened(Result<(), String>),
    /// A modpack's page was opened in the browser, or could not be.
    SourceOpened(Result<(), String>),
    /// A scene picture finished rendering, or failed (`None`).
    ArtReady {
        key: ArtKey,
        handle: Option<Handle>,
    },
    /// TEMPORARY resize-latency probe tick — active only with
    /// `LODESTONE_RESIZE_PROFILE=1`; drives `window::resize` through a
    /// fixed size sequence for screen-recording measurements.
    ResizeProbe,
    /// Boot-time restore: silently re-login a stored Microsoft refresh
    /// token before showing the sign-in card.
    RestoreStart {
        username: String,
        refresh_token: String,
    },
    /// Tick the auth-services reachability probe.
    AuthProbeTick,
    /// The auth-services probe completed.
    AuthProbeResult(Result<AuthProbe, String>),
    /// A sidebar nav item was pressed. The Library, Discover and Worlds (indices 0 to 2) navigate; the
    /// rest are display-only.
    SidebarNav {
        group: NavGroup,
        index: usize,
    },
    /// The account's skin and cape finished loading, for the sidebar's face and the Skins page's viewer.
    AppearanceReady(Result<Appearance, String>),
    /// A background credential write finished.
    CredentialsSaved(Result<(), String>),
    /// The open alert was answered (see `components::alert`).
    Alert(AlertMessage),
    /// A game session message: a launch, a running game's output or exit, or a stop.
    Game(GameMessage),
    /// A Microsoft session refresh finished, with the new session or the reason it failed.
    SessionRefreshed(Result<Session, String>),
    /// The performance overlay: its F3 toggle and its one-second refresh. See `perf`.
    Perf(perf::Message),
}

/// Which sidebar group a nav selection came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavGroup {
    Top,
    Bottom,
}
