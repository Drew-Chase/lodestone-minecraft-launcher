//! The root message type: every event the launcher reacts to, from window
//! chrome and navigation to page messages and background-task results.

use crate::app::components::alert::Message as AlertMessage;
use crate::app::components::art_cache::ArtKey;
use crate::app::database::Database;
use crate::app::downloads::Message as DownloadsMessage;
use crate::app::launch::GameMessage;
use crate::app::pages::Page;
use crate::app::pages::instance::Message as InstanceMessage;
use crate::app::pages::library::Message as LibraryMessage;
use crate::app::pages::library::instance::Instance;
use crate::app::pages::login::Message as LoginMessage;
use crate::app::pages::settings::Message as SettingsMessage;
use crate::app::pages::showcase::Message as ShowcaseMessage;
use crate::app::perf;
use crate::app::titlebar::TitlebarMessage;
use iced::Size;
use iced::widget::image::Handle;
use iced::window::{Direction, Id};
use lodestone_minecraft_authentication::{AuthProbe, Session};

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
    Instance(InstanceMessage),
    Settings(SettingsMessage),
    Showcase(ShowcaseMessage),
    /// The download queue: an install's progress or result, and the Downloads popover's controls.
    Downloads(DownloadsMessage),
    /// The launcher database finished opening (schema migrations ran).
    DatabaseReady(Result<Database, String>),
    /// The instance list finished loading, with each mod count read from disk.
    InstancesLoaded(Result<Vec<Instance>, String>),
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
    /// A sidebar nav item was pressed. Only the Library (index 0) navigates
    /// today; the rest are display-only.
    SidebarNav {
        group: NavGroup,
        index: usize,
    },
    /// The player face finished loading (8×8 RGBA, ready to upscale).
    FaceReady(Result<Vec<u8>, String>),
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
