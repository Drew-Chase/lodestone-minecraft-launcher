pub mod components;
pub mod database;
pub mod face;
pub mod pages;
pub mod router;
pub mod storage;
pub mod theme;
mod titlebar;

use crate::app::components::backdrop::LoginBackdrop;
use crate::app::components::sidebar::sidebar as sidebar_view;
use crate::app::database::Database;
use crate::app::pages::Page;
use crate::app::pages::instance::{InstancePage, Message as InstanceMessage};
use crate::app::pages::library::instance::Instance;
use crate::app::pages::library::{Library, LibraryView, Message as LibraryMessage};
use crate::app::pages::login::{AuthStatus, Login, Message as LoginMessage};
use crate::app::pages::settings::{Message as SettingsMessage, Settings};
use crate::app::pages::showcase::{Message as ShowcaseMessage, Showcase};
use crate::app::router::Router;
use crate::app::storage::{StoredCredentials, StoredMicrosoft};
use crate::app::theme::color::Surface;
use crate::app::titlebar::{Titlebar, TitlebarMessage};
use chrono::Local;
use iced::widget::{column, container, image, row, stack};
use iced::window::{Direction, Id, Level, Position};
use iced::{Element, Fill, Size, Subscription, Task, event, mouse, window};
use iced_borderless_window_resize_handle_widget::{ResizeHandles, drag_resize};
use lodestone_minecraft_authentication::{AuthProbe, Authentication, Session};
use std::path::PathBuf;
use std::sync::OnceLock;
use tracing::{debug, info, warn};

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
            auth: Authentication::new(AZURE_CLIENT_ID),
            auth_generation: 0,
            face: None,
            probe_step: 0,
            auth_status: None,
        }
    }
}

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
    /// The launcher database finished opening (schema migrations ran).
    DatabaseReady(Result<Database, String>),
    /// The instance list finished loading (after any first-boot seed).
    InstancesLoaded(Result<Vec<Instance>, String>),
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
}

/// Which sidebar group a nav selection came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavGroup {
    Top,
    Bottom,
}

impl LodestoneLauncher {
    pub const TITLE: &'static str = "Lodestone Launcher";
    /// The release major and minor versions.
    /// Ex: `0.0.0`
    pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");
    /// A unique build number that is created automatically at compile time.
    /// Ex: `20260827.032580`
    pub const BUILD: &'static str = env!("BUILD");

    pub fn boot() -> (Self, Task<LodestoneAppMessage>) {
        let id = window::latest();
        let mut app = Self { ..Self::default() };
        let mut tasks = vec![id.map(LodestoneAppMessage::WindowCreation)];
        // TEMPORARY library-view verification — env-gated: boots the Library
        // into `compact` or `table` so each view can be screenshotted.
        if let Some(view) = std::env::var_os("LODESTONE_LIBRARY_VIEW") {
            let view = match view.to_string_lossy().as_ref() {
                "compact" => LibraryView::Compact,
                "table" => LibraryView::Table,
                _ => LibraryView::Grid,
            };
            info!("PROBE library view override: {view:?}");
            app.library.set_view(view);
        }
        // Open the launcher database — migrations run on connect, and the
        // first boot seeds the instance list.
        tasks.push(Task::perform(Database::connect(), |result| LodestoneAppMessage::DatabaseReady(result)));
        // TEMPORARY resize-latency probe kick-off — env-gated.
        if probe_enabled() {
            info!("PROBE resize-latency profiling active");
            tasks.push(Task::perform(
                async {
                    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
                },
                |_| LodestoneAppMessage::ResizeProbe,
            ));
        }
        // TEMPORARY auth-path verification — env-gated: auto-presses the
        // Microsoft button after 2s so the network flow can be measured
        // without interaction.
        if std::env::var_os("LODESTONE_AUTH_TEST").is_some() {
            info!("PROBE auth verification active");
            tasks.push(Task::perform(
                async {
                    tokio::time::sleep(std::time::Duration::from_millis(2000)).await;
                },
                |_| LodestoneAppMessage::Login(LoginMessage::SignInMicrosoft),
            ));
        }
        // TEMPORARY offline-form verification — env-gated: opens the offline
        // username prompt after 2s and confirms it at 3.5s (entering the
        // launcher and exercising the Steve-face fetch).
        if std::env::var_os("LODESTONE_OFFLINE_TEST").is_some() {
            info!("PROBE offline-form verification active");
            tasks.push(Task::perform(
                async {
                    tokio::time::sleep(std::time::Duration::from_millis(2000)).await;
                },
                |_| LodestoneAppMessage::Login(LoginMessage::OfflineMode),
            ));
            tasks.push(Task::perform(
                async {
                    tokio::time::sleep(std::time::Duration::from_millis(3500)).await;
                },
                |_| {
                    LodestoneAppMessage::Login(LoginMessage::OfflineConfirm {
                        username: "Demo Player".to_string(),
                    })
                },
            ));
        }
        // Silent re-login when a Microsoft refresh token is on file — the
        // launcher goes straight to Home on success.
        if let Some(ms) = app.credentials.microsoft.clone() {
            info!("PROBE restoring stored Microsoft sign-in for {}", ms.username);
            tasks.push(Task::perform(
                async {
                    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
                },
                move |_| LodestoneAppMessage::RestoreStart {
                    username: ms.username,
                    refresh_token: ms.refresh_token,
                },
            ));
        }
        // First auth-services probe shortly after boot; the subscription
        // keeps it going every 15s.
        tasks.push(Task::perform(
            async {
                tokio::time::sleep(std::time::Duration::from_millis(800)).await;
            },
            |_| LodestoneAppMessage::AuthProbeTick,
        ));
        (app, Task::batch(tasks))
    }

    pub fn update(&mut self, message: LodestoneAppMessage) -> Task<LodestoneAppMessage> {
        match message {
            LodestoneAppMessage::TitlebarMessage(message) => match self.id {
                Some(id) => self.titlebar.update(message, id),
                None => Task::none(),
            },
            LodestoneAppMessage::WindowCreation(id) => {
                info!("Window id acquired: {id:?}");
                self.id = id;
                Task::none()
            }
            LodestoneAppMessage::WindowStartResize(direction) => match self.id {
                Some(id) => {
                    debug!("Starting window resize: {direction:?}");
                    drag_resize(id, direction)
                }
                None => Task::none(),
            },
            LodestoneAppMessage::WindowSizeChanged(id, size) => {
                debug!("Window {id} size changed");
                self.window_size = size;
                if probe_enabled() {
                    info!("PROBE resized event: {}x{}", size.width as u32, size.height as u32);
                }
                window::is_maximized(id).map(LodestoneAppMessage::WindowMaximized)
            }
            LodestoneAppMessage::WindowMaximized(maximized) => {
                if self.maximized != maximized {
                    info!("Window maximized state changed: {maximized}");
                }
                self.maximized = maximized;
                Task::none()
            }
            LodestoneAppMessage::Navigate(page) => self.navigate(page),
            LodestoneAppMessage::NavBack => {
                if self.router.go_back() {
                    info!("Navigated back to {}", self.router.current().title());
                }
                Task::none()
            }
            LodestoneAppMessage::NavForward => {
                if self.router.go_forward() {
                    info!("Navigated forward to {}", self.router.current().title());
                }
                Task::none()
            }
            // The reload button re-queries the database through the root;
            // every other Library message is pure display state.
            LodestoneAppMessage::Library(LibraryMessage::Reload) => self.load_instances(),
            // Opening a card/hero hands the instance to the detail page and
            // pushes the route — the page itself never owns navigation.
            LodestoneAppMessage::Library(LibraryMessage::OpenInstance(id)) => match self.library.instance(&id) {
                Some(instance) => {
                    self.instance_page.show(instance);
                    self.navigate(Page::Instance)
                }
                None => {
                    warn!("Open instance {id} ignored: the instance is no longer loaded");
                    Task::none()
                }
            },
            LodestoneAppMessage::Library(message) => {
                self.library.update(message);
                Task::none()
            }
            LodestoneAppMessage::Instance(InstanceMessage::Navigate(page)) => self.navigate(page),
            LodestoneAppMessage::Instance(InstanceMessage::Back) => {
                if self.router.go_back() {
                    info!("Navigated back to {}", self.router.current().title());
                }
                Task::none()
            }
            LodestoneAppMessage::Instance(message) => {
                self.instance_page.update(message);
                Task::none()
            }
            LodestoneAppMessage::DatabaseReady(Ok(db)) => {
                info!("Launcher database ready");
                self.db = Some(db);
                self.load_instances()
            }
            LodestoneAppMessage::DatabaseReady(Err(error)) => {
                warn!("Launcher database unavailable: {error}");
                Task::none()
            }
            LodestoneAppMessage::InstancesLoaded(Ok(instances)) => {
                info!("Loaded {} instance(s)", instances.len());
                self.library.set_instances(instances);
                Task::none()
            }
            LodestoneAppMessage::InstancesLoaded(Err(error)) => {
                warn!("Loading instances failed: {error}");
                Task::none()
            }
            // Only `Navigate` exists today; when the page grows more
            // variants, add the delegation arm per the `pages` contract.
            LodestoneAppMessage::Settings(SettingsMessage::Navigate(page)) => self.navigate(page),
            LodestoneAppMessage::Showcase(ShowcaseMessage::Navigate(page)) => self.navigate(page),
            LodestoneAppMessage::Login(LoginMessage::Navigate(page)) => self.navigate(page),
            LodestoneAppMessage::Login(LoginMessage::SignInMicrosoft) => {
                // Bump the generation so results from any abandoned poll
                // are dropped when they eventually arrive.
                self.auth_generation = self.auth_generation.wrapping_add(1);
                let generation = self.auth_generation;
                info!("Beginning Microsoft device-code sign-in (generation {generation})");
                self.login.update(LoginMessage::Opening);
                let auth = self.auth.clone();
                Task::perform(
                    async move { auth.begin_device_code_login().await.map_err(|error| error.to_string()) },
                    move |result| LodestoneAppMessage::Login(LoginMessage::DeviceCodeStarted(generation, result)),
                )
            }
            LodestoneAppMessage::Login(LoginMessage::DeviceCodeStarted(generation, Ok(start))) if generation == self.auth_generation => {
                let user_code = start.user_code.clone();
                let verification_uri = start.verification_uri.clone();
                info!("Device code ready: {user_code} at {verification_uri}");
                self.login.update(LoginMessage::CodeReady { user_code, verification_uri });
                let auth = self.auth.clone();
                Task::perform(
                    async move { auth.finish_device_code_login(start).await.map_err(|error| error.to_string()) },
                    move |result| LodestoneAppMessage::Login(LoginMessage::SignInFinished(generation, result)),
                )
            }
            LodestoneAppMessage::Login(LoginMessage::DeviceCodeStarted(generation, Err(error))) if generation == self.auth_generation => {
                warn!("Device-code request failed: {error}");
                self.login.update(LoginMessage::SignInFailed(error));
                Task::none()
            }
            LodestoneAppMessage::Login(LoginMessage::SignInFinished(generation, Ok(session))) if generation == self.auth_generation => {
                info!("Signed in as {}", session.profile.name);
                // Persist the refresh token so the next boot restores this
                // sign-in silently.
                match &session.refresh_token {
                    Some(refresh_token) => {
                        self.credentials.microsoft = Some(StoredMicrosoft {
                            refresh_token: refresh_token.clone(),
                            username: session.profile.name.clone(),
                        });
                        if let Err(error) = self.credentials.store() {
                            warn!("Storing credentials failed: {error}");
                        }
                    }
                    None => warn!("Microsoft issued no refresh token; the sign-in will not persist"),
                }
                self.account = Some(Account::Microsoft { session });
                self.login.update(LoginMessage::Reset);
                // Fetch the player's face for the sidebar avatar — the
                // ACTIVE skin's texture, or the vanilla Steve when the
                // profile reports none (default-skin accounts).
                let auth = self.auth.clone();
                let skin_url = self.account.as_ref().and_then(|account| match account {
                    Account::Microsoft { session } => session.profile.skin_url.clone(),
                    Account::Offline { .. } => None,
                });
                let face_task = match skin_url {
                    Some(url) => Task::perform(async move { auth.fetch_skin(&url).await.map_err(|error| error.to_string()) }, |result| {
                        LodestoneAppMessage::FaceReady(result)
                    }),
                    None => Task::perform(
                        async move { auth.fetch_default_steve_face().await.map_err(|error| error.to_string()) },
                        |result| LodestoneAppMessage::FaceReady(result),
                    ),
                };
                self.router.replace(Page::Home);
                face_task
            }
            LodestoneAppMessage::Login(LoginMessage::SignInFinished(generation, Err(error))) if generation == self.auth_generation => {
                warn!("Sign-in failed: {error}");
                // A failed sign-in also invalidates whatever refresh token
                // was on file — it could not have produced this attempt.
                if self.credentials.microsoft.take().is_some() {
                    if let Err(storage_error) = self.credentials.store() {
                        warn!("Clearing stored credentials failed: {storage_error}");
                    }
                }
                self.login.update(LoginMessage::SignInFailed(error));
                Task::none()
            }
            // Results from cancelled generations — dropped silently.
            LodestoneAppMessage::Login(LoginMessage::DeviceCodeStarted(generation, _))
            | LodestoneAppMessage::Login(LoginMessage::SignInFinished(generation, _)) => {
                debug!("Stale sign-in result from generation {generation} dropped");
                Task::none()
            }
            LodestoneAppMessage::Login(LoginMessage::CopyCode { code }) => {
                info!("Device code copied to clipboard");
                self.login.update(LoginMessage::CodeCopied);
                iced::clipboard::write(code)
            }
            LodestoneAppMessage::Login(LoginMessage::OpenVerificationLink { url }) => {
                match open::that_detached(&url) {
                    Ok(()) => info!("Opened {url} in the default browser"),
                    Err(error) => warn!("Failed to open {url}: {error}"),
                }
                Task::none()
            }
            LodestoneAppMessage::Login(LoginMessage::CancelSignIn) => {
                info!("Sign-in cancelled");
                self.auth_generation = self.auth_generation.wrapping_add(1);
                self.login.update(LoginMessage::Reset);
                Task::none()
            }
            LodestoneAppMessage::Login(LoginMessage::OfflineMode) => {
                // Open the offline prompt, pre-filled with the stored
                // username when there is one.
                let username = self.credentials.offline_username.clone().unwrap_or_default();
                self.login.update(LoginMessage::OfflinePrompt { username });
                Task::none()
            }
            LodestoneAppMessage::Login(LoginMessage::OfflineUsernameChanged(username)) => {
                self.login.update(LoginMessage::OfflineUsernameChanged(username));
                Task::none()
            }
            LodestoneAppMessage::SidebarNav {
                group: NavGroup::Top,
                index: 0,
            } => self.navigate(Page::Home),
            LodestoneAppMessage::SidebarNav { group, index } => {
                debug!("Sidebar nav item {group:?}#{index} has no page yet");
                Task::none()
            }
            LodestoneAppMessage::FaceReady(Ok(skin_png)) => {
                self.face = face::face_from_skin_png(&skin_png)
                    .and_then(|face| face::expanded_avatar(&face))
                    .and_then(|expanded| Some(image::Handle::from_rgba(face::AVATAR_SIZE as u32, face::AVATAR_SIZE as u32, expanded)));
                Task::none()
            }
            LodestoneAppMessage::FaceReady(Err(error)) => {
                warn!("Player face unavailable: {error}");
                self.face = None;
                Task::none()
            }
            LodestoneAppMessage::Login(LoginMessage::OfflineConfirm { username }) => {
                let username = username.trim().to_string();
                if username.is_empty() {
                    return Task::none();
                }
                info!("Entering offline mode as {username}");
                self.credentials.offline_username = Some(username.clone());
                if let Err(error) = self.credentials.store() {
                    warn!("Storing credentials failed: {error}");
                }
                self.account = Some(Account::Offline { username });
                self.login.update(LoginMessage::Reset);
                // The canonical Steve face — fetched from Mojang's own
                // profile service for the "Steve" account.
                let auth = self.auth.clone();
                let face_task = Task::perform(
                    async move { auth.fetch_default_steve_face().await.map_err(|error| error.to_string()) },
                    |result| LodestoneAppMessage::FaceReady(result),
                );
                self.router.replace(Page::Home);
                face_task
            }
            LodestoneAppMessage::RestoreStart { username, refresh_token } => {
                self.auth_generation = self.auth_generation.wrapping_add(1);
                let generation = self.auth_generation;
                info!("Restoring Microsoft sign-in for {username} (generation {generation})");
                self.login.update(LoginMessage::Restoring { username });
                let auth = self.auth.clone();
                Task::perform(
                    async move { auth.login_with_refresh_token(&refresh_token).await.map_err(|error| error.to_string()) },
                    move |result| LodestoneAppMessage::Login(LoginMessage::SignInFinished(generation, result)),
                )
            }
            LodestoneAppMessage::Login(LoginMessage::DemoMode) => {
                info!("Entering offline mode as Demo Player");
                let username = "Demo Player".to_string();
                self.credentials.offline_username = Some(username.clone());
                if let Err(error) = self.credentials.store() {
                    warn!("Storing credentials failed: {error}");
                }
                self.account = Some(Account::Offline { username });
                self.login.update(LoginMessage::Reset);
                self.router.replace(Page::Home);
                Task::none()
            }
            LodestoneAppMessage::Login(LoginMessage::LearnMore) => {
                info!("'Learn more' requested — opening a browser is not wired yet");
                Task::none()
            }
            LodestoneAppMessage::ResizeProbe => {
                self.probe_step = self.probe_step.wrapping_add(1);
                const SIZES: [(f32, f32); 6] = [
                    (1200.0, 800.0),
                    (1920.0, 1080.0),
                    (1400.0, 900.0),
                    (1920.0, 1080.0),
                    (1000.0, 700.0),
                    (1920.0, 1080.0),
                ];
                let (w, h) = SIZES[(self.probe_step as usize - 1) % SIZES.len()];
                info!("PROBE resize #{} -> {}x{}", self.probe_step, w, h);
                let apply = match self.id {
                    Some(id) => window::resize(id, Size::new(w, h)),
                    None => Task::none(),
                };
                let tick = Task::perform(
                    async {
                        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                    },
                    |_| LodestoneAppMessage::ResizeProbe,
                );
                Task::batch([apply, tick])
            }
            // Display-state messages: pure page state, forwarded as-is.
            LodestoneAppMessage::Login(
                message @ (LoginMessage::Opening
                | LoginMessage::CodeReady { .. }
                | LoginMessage::CodeCopied
                | LoginMessage::SignInFailed(_)
                | LoginMessage::Reset
                | LoginMessage::CodeHover(_)
                | LoginMessage::LinkHover(_)
                | LoginMessage::Restoring { .. }
                | LoginMessage::OfflinePrompt { .. }),
            ) => {
                self.login.update(message);
                Task::none()
            }
            LodestoneAppMessage::AuthProbeTick => {
                let auth = self.auth.clone();
                Task::perform(async move { auth.probe().await.map_err(|error| error.to_string()) }, |result| {
                    LodestoneAppMessage::AuthProbeResult(result)
                })
            }
            LodestoneAppMessage::AuthProbeResult(result) => {
                match &result {
                    Ok(probe) => info!("Auth services reachable: {}ms", probe.delay_ms),
                    Err(error) => warn!("Auth services unreachable: {error}"),
                }
                self.auth_status = Some(result);
                Task::none()
            }
            LodestoneAppMessage::Showcase(message) => {
                self.showcase.update(message);
                Task::none()
            }
        }
    }

    /// Routes to `page` through the router's history.
    fn navigate(&mut self, page: Page) -> Task<LodestoneAppMessage> {
        let from = self.router.current();
        if self.router.push(page) {
            info!("Navigated: {} → {}", from.title(), page.title());
        } else {
            debug!("Navigation ignored: already on {}", page.title());
        }
        Task::none()
    }

    /// Loads the instance list from the database, seeding it on the very
    /// first boot (empty table) with the demo pack.
    fn load_instances(&mut self) -> Task<LodestoneAppMessage> {
        let Some(db) = self.db.clone() else {
            return Task::none();
        };
        let instances_dir = self.instances_dir.clone();
        Task::perform(
            async move {
                if db.instances_empty().await? {
                    let pack = Instance::seed_pack(Local::now());
                    info!("Seeding {} starter instances", pack.len());
                    db.seed(&pack, &instances_dir).await?;
                }
                db.instances().await
            },
            |result| LodestoneAppMessage::InstancesLoaded(result),
        )
    }

    pub fn view(&self) -> Element<'_, LodestoneAppMessage> {
        let content: Element<'_, LodestoneAppMessage> = if self.router.current() == Page::Login {
            // The login gate: the backdrop fills the whole window and the
            // chrome floats over it, transparent. No rail, no history.
            let auth_status = self.auth_status.as_ref().map(|result| match result {
                Ok(probe) => {
                    let region = match (&probe.region_scope, &probe.region_sub_scope) {
                        (Some(scope), Some(sub)) if !sub.is_empty() => Some(format!("{scope}-{sub}")),
                        (Some(scope), _) => Some(scope.clone()),
                        _ => None,
                    };
                    AuthStatus {
                        online: true,
                        delay_ms: probe.delay_ms,
                        region,
                    }
                }
                Err(_) => AuthStatus {
                    online: false,
                    delay_ms: 0,
                    region: None,
                },
            });
            stack![
                LoginBackdrop::new(),
                column![
                    self.titlebar
                        .view(self.router.can_go_back(), self.router.can_go_forward(), true)
                        .map(LodestoneAppMessage::TitlebarMessage),
                    container(self.login.view(auth_status).map(LodestoneAppMessage::Login))
                        .width(Fill)
                        .height(Fill),
                ]
                .width(Fill)
                .height(Fill),
            ]
            .into()
        } else {
            column![
                self.titlebar
                    .view(self.router.can_go_back(), self.router.can_go_forward(), false)
                    .map(LodestoneAppMessage::TitlebarMessage),
                row![self.rail_column(), self.body()].width(Fill).height(Fill),
            ]
            .width(Fill)
            .height(Fill)
            .into()
        };
        stack![
            content,
            ResizeHandles::new(LodestoneAppMessage::WindowStartResize)
                .gutter(4.0)
                .enabled(!self.maximized)
        ]
        .width(Fill)
        .height(Fill)
        .into()
    }

    /// The full sidebar — logo, all nav groups, and the player avatar. The
    /// top group's first item (Library) navigates; the rest are display-only
    /// until their pages exist.
    fn rail_column(&self) -> Element<'_, LodestoneAppMessage> {
        sidebar_view(
            self.face.clone(),
            self.account.as_ref().map(Account::username).map(str::to_string),
            matches!(self.account, Some(Account::Microsoft { .. })),
            0,
            |index| {
                if index == 0 {
                    LodestoneAppMessage::Navigate(Page::Home)
                } else {
                    debug!("Sidebar nav item {index} has no page yet");
                    LodestoneAppMessage::SidebarNav { group: NavGroup::Top, index }
                }
            },
            |index| LodestoneAppMessage::SidebarNav {
                group: NavGroup::Bottom,
                index,
            },
        )
    }

    /// Left edge chrome — one rail slot per [`Page::RAIL_PAGES`] entry.
    /// The page area: the router's current page.
    fn body(&self) -> Element<'_, LodestoneAppMessage> {
        container(self.outlet())
            .width(Fill)
            .height(Fill)
            .style(|_| container::Style {
                background: Some(iced::Background::Color(Surface::APPLICATION)),
                ..Default::default()
            })
            .into()
    }

    /// The `<Outlet/>` — whichever page the router points at. The login gate
    /// renders through `view`'s dedicated branch, so this arm only serves
    /// callers that ask for the current page generically.
    fn outlet(&self) -> Element<'_, LodestoneAppMessage> {
        match self.router.current() {
            Page::Login => self.login.view(None).map(LodestoneAppMessage::Login),
            Page::Home => self.library.view(self.window_size).map(LodestoneAppMessage::Library),
            Page::Instance => self.instance_page.view(self.window_size).map(LodestoneAppMessage::Instance),
            Page::Settings => self.settings.view().map(LodestoneAppMessage::Settings),
            Page::Showcase => self.showcase.view().map(LodestoneAppMessage::Showcase),
        }
    }

    pub fn subscription(&self) -> Subscription<LodestoneAppMessage> {
        Subscription::batch([
            window::resize_events().map(|(id, size)| LodestoneAppMessage::WindowSizeChanged(id, size)),
            event::listen_with(Self::mouse_navigation),
            // The auth-services probe — keeps the login status bar honest.
            iced::time::every(std::time::Duration::from_secs(15)).map(|_| LodestoneAppMessage::AuthProbeTick),
        ])
    }

    /// Maps the mouse's side buttons onto history navigation — back and
    /// forward, like a browser.
    ///
    /// Only uncaptured presses count, so widgets that consume an event keep
    /// it to themselves. Delivery depends on the platform exposing the
    /// hardware buttons through the windowing system (standard on Windows,
    /// X11 buttons 8/9 on Linux, compositor-dependent on Wayland). The app
    /// is single-window, so the source window is irrelevant.
    fn mouse_navigation(event: iced::Event, status: event::Status, _window: Id) -> Option<LodestoneAppMessage> {
        if status != event::Status::Ignored {
            return None;
        }
        match event {
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Back)) => Some(LodestoneAppMessage::NavBack),
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Forward)) => Some(LodestoneAppMessage::NavForward),
            _ => None,
        }
    }

    pub fn window_settings() -> window::Settings {
        window::Settings {
            size: Size::new(1280.0, 720.0),
            min_size: Some(Size::new(800f32, 600f32)),
            decorations: !cfg!(target_os = "windows"),
            resizable: true,
            #[cfg(target_os = "windows")]
            platform_specific: window::settings::PlatformSpecific {
                corner_preference: window::settings::platform::CornerPreference::Default,
                undecorated_shadow: true,
                drag_and_drop: false,
                skip_taskbar: false,
            },
            #[cfg(target_os = "macos")]
            platform_specific: iced::window::settings::PlatformSpecific {
                title_hidden: true,
                titlebar_transparent: true,
                fullsize_content_view: true,
            },
            #[cfg(target_os = "linux")]
            platform_specific: window::settings::PlatformSpecific {
                #[cfg(debug_assertions)]
                application_id: String::from("lodestone_launcher-dev"),
                #[cfg(not(debug_assertions))]
                application_id: String::from("lodestone_launcher"),
                override_redirect: false,
            },
            maximized: false,
            fullscreen: false,
            position: Position::Default,
            max_size: None,
            visible: true,
            minimizable: true,
            closeable: true,
            transparent: false,
            blur: false,
            level: Level::Normal,
            icon: None,
            exit_on_close_request: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(button: mouse::Button) -> iced::Event {
        iced::Event::Mouse(mouse::Event::ButtonPressed(button))
    }

    #[test]
    fn side_buttons_map_to_history_navigation() {
        let window_id = window::Id::unique();

        assert!(matches!(
            LodestoneLauncher::mouse_navigation(press(mouse::Button::Back), event::Status::Ignored, window_id),
            Some(LodestoneAppMessage::NavBack)
        ));
        assert!(matches!(
            LodestoneLauncher::mouse_navigation(press(mouse::Button::Forward), event::Status::Ignored, window_id),
            Some(LodestoneAppMessage::NavForward)
        ));
    }

    #[test]
    fn captured_events_and_other_input_are_ignored() {
        let window_id = window::Id::unique();

        // A widget consumed the press — it belongs to the widget.
        assert!(LodestoneLauncher::mouse_navigation(press(mouse::Button::Back), event::Status::Captured, window_id).is_none());
        // Ordinary buttons and non-button events never navigate.
        assert!(LodestoneLauncher::mouse_navigation(press(mouse::Button::Left), event::Status::Ignored, window_id).is_none());
        assert!(
            LodestoneLauncher::mouse_navigation(
                iced::Event::Mouse(mouse::Event::CursorMoved {
                    position: iced::Point::ORIGIN
                }),
                event::Status::Ignored,
                window_id
            )
            .is_none()
        );
    }
}
