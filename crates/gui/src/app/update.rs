//! The root `update`: window chrome, navigation, page delegation, the
//! database, and the probes. Sign-in and account messages are routed to
//! `auth.rs`.

use crate::app::database::Database;
use crate::app::face;
use crate::app::lang;
use crate::app::launch::GameMessage;
use crate::app::launch::pidfile;
use crate::app::pages::Page;
use crate::app::pages::instance::Message as InstanceMessage;
use crate::app::pages::instance::Tab;
use crate::app::pages::instance::files;
use crate::app::pages::instance::settings::Message as InstanceSettingsMessage;
use crate::app::pages::instance::{screenshots, worlds};
use crate::app::pages::library::Message as LibraryMessage;
use crate::app::pages::library::instance::Instance;
use crate::app::pages::login::Message as LoginMessage;
use crate::app::pages::settings::Message as SettingsMessage;
use crate::app::pages::showcase::Message as ShowcaseMessage;
use crate::app::perf;
use crate::app::preferences::Preferences;
use iced::widget::image;
use iced::{Size, Task, window};
use iced_borderless_window_resize_handle_widget::drag_resize;
use std::path::PathBuf;
use tracing::{debug, info, warn};

use super::{LodestoneAppMessage, LodestoneLauncher, NavGroup, downloads, probe_enabled};

impl LodestoneLauncher {
    pub fn update(&mut self, message: LodestoneAppMessage) -> Task<LodestoneAppMessage> {
        let _scope = perf::scope(perf::message_kind(&message));
        let task = self.route(message);
        self.sync_discover_installs();
        Task::batch([task, self.sync_art()])
    }

    /// Keeps the library's scene art rendered for the rows and the hero it shows, and starts the renders it
    /// lacks. The library is on show only on Home, so the art is only kept up to date there.
    fn sync_art(&mut self) -> Task<LodestoneAppMessage> {
        if self.router.current() != Page::Home {
            return Task::none();
        }
        let keys = self.library.art_keys(self.window_size);
        self.art.retain(&keys.iter().copied().collect());
        let missing = self.art.missing(&keys);
        self.art.request(missing, |key, handle| LodestoneAppMessage::ArtReady { key, handle })
    }

    /// Carries out one message and returns the task it starts.
    fn route(&mut self, message: LodestoneAppMessage) -> Task<LodestoneAppMessage> {
        match message {
            LodestoneAppMessage::ArtReady { key, handle } => {
                self.art.finish(key, handle);
                Task::none()
            }
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
            // Play starts the game, or opens the console of one already running.
            LodestoneAppMessage::Library(LibraryMessage::Play(id)) => self.launch_game(&id),
            LodestoneAppMessage::Library(LibraryMessage::LaunchNew(id)) => self.launch_new_game(&id),
            // Opening a card/hero hands the instance to the detail page and
            // pushes the route — the page itself never owns navigation.
            LodestoneAppMessage::Library(LibraryMessage::OpenInstance(id)) => self.open_instance_page(&id, Tab::Overview),
            LodestoneAppMessage::Library(LibraryMessage::EditInstance(id)) => self.open_instance_page(&id, Tab::Settings),
            LodestoneAppMessage::Library(LibraryMessage::RevealInstance(id)) => self.reveal_instance(&id),
            LodestoneAppMessage::Library(LibraryMessage::InstanceMenu { instance, action }) => self.instance_menu_action(&instance, action),
            LodestoneAppMessage::Library(LibraryMessage::OpenNewInstance) => self.open_new_instance(),
            LodestoneAppMessage::Library(LibraryMessage::NewInstance(message)) => self.new_instance_message(message),
            LodestoneAppMessage::Downloads(message) => self.downloads_message(message),
            LodestoneAppMessage::Discover(message) => self.discover_message(message),
            LodestoneAppMessage::SharedLoaded(Ok(records)) => {
                info!("Loaded {} shared download(s)", records.len());
                self.discover.set_library(&records);
                Task::none()
            }
            LodestoneAppMessage::SharedLoaded(Err(error)) => {
                warn!("Loading the shared downloads failed: {error}");
                Task::none()
            }
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
            // The page's reveal and delete actions need the root's IO and alerts, so they are routed here.
            LodestoneAppMessage::Instance(InstanceMessage::RevealFolder) => self.reveal_current_instance(),
            LodestoneAppMessage::Instance(InstanceMessage::OpenSource) => self.open_current_source(),
            LodestoneAppMessage::Instance(InstanceMessage::Menu(action)) => self.current_instance_menu(action),
            LodestoneAppMessage::SourceOpened(result) => self.source_opened(result),
            LodestoneAppMessage::Instance(InstanceMessage::Settings(InstanceSettingsMessage::RevealFolder)) => self.reveal_current_instance(),
            LodestoneAppMessage::Instance(InstanceMessage::Settings(InstanceSettingsMessage::DeleteInstance)) => self.ask_current_instance_delete(),
            // The page's game controls act on the instance it shows.
            LodestoneAppMessage::Instance(InstanceMessage::Launch) => self.current_game(GameMessage::Launch),
            LodestoneAppMessage::Instance(InstanceMessage::LaunchNew(id)) => self.launch_new_game(&id),
            LodestoneAppMessage::Instance(InstanceMessage::Kill) => self.current_game(GameMessage::Kill),
            LodestoneAppMessage::Instance(InstanceMessage::Restart) => self.current_game(GameMessage::Restart),
            // The Screenshots and Worlds tabs ask the root for their actions, and their results come back here.
            LodestoneAppMessage::Instance(InstanceMessage::Screenshots(screenshots::Message::Menu { path, action })) => {
                self.screenshot_action(path, action)
            }
            LodestoneAppMessage::Instance(InstanceMessage::Screenshots(screenshots::Message::DeleteAnswered { path, confirmed })) => {
                self.answer_screenshot_delete(path, confirmed)
            }
            LodestoneAppMessage::Instance(InstanceMessage::Screenshots(screenshots::Message::Deleted { path, result })) => {
                self.screenshot_deleted(path, result)
            }
            LodestoneAppMessage::Instance(InstanceMessage::Screenshots(screenshots::Message::Copied(result))) => self.screenshot_copied(result),
            LodestoneAppMessage::Instance(InstanceMessage::Screenshots(screenshots::Message::Opened(result))) => self.screenshot_opened(result),
            LodestoneAppMessage::Instance(InstanceMessage::Worlds(worlds::Message::Menu { path, action })) => self.world_action(path, action),
            LodestoneAppMessage::Instance(InstanceMessage::Worlds(worlds::Message::Import)) => self.import_world(),
            LodestoneAppMessage::Instance(InstanceMessage::Worlds(worlds::Message::NewWorld)) => self.new_world(),
            LodestoneAppMessage::Instance(InstanceMessage::Worlds(worlds::Message::DeleteAnswered { path, confirmed })) => {
                self.answer_world_delete(path, confirmed)
            }
            LodestoneAppMessage::Instance(InstanceMessage::Worlds(worlds::Message::Deleted { path, result })) => self.world_deleted(path, result),
            LodestoneAppMessage::Instance(InstanceMessage::Worlds(worlds::Message::BackedUp(result))) => self.world_backed_up(result),
            LodestoneAppMessage::Instance(InstanceMessage::Worlds(worlds::Message::Imported(result))) => self.world_imported(result),
            LodestoneAppMessage::Instance(message) => self.instance_page.update(message).map(LodestoneAppMessage::Instance),
            LodestoneAppMessage::DatabaseReady(Ok(db)) => {
                info!("Launcher database ready");
                let sweep = downloads::sweep_partial_installs(db.clone(), self.paths.instances());
                self.db = Some(db);
                // The games a previous launcher left running are found through their pid files. Attaching
                // to them sets their state, and then the library loads.
                let reattach = Task::perform(
                    async { tokio::task::spawn_blocking(pidfile::load_live).await.unwrap_or_default() },
                    |records| LodestoneAppMessage::Game(GameMessage::Reattach(records)),
                );
                Task::batch([sweep, reattach, self.load_shared()])
            }
            LodestoneAppMessage::DatabaseReady(Err(error)) => {
                warn!("Launcher database unavailable: {error}");
                Task::none()
            }
            LodestoneAppMessage::InstancesLoaded(Ok(instances)) => {
                info!("Loaded {} instance(s)", instances.len());
                self.library.set_instances(instances);
                self.refresh_instance_page();
                Task::none()
            }
            LodestoneAppMessage::InstancesLoaded(Err(error)) => {
                warn!("Loading instances failed: {error}");
                Task::none()
            }
            LodestoneAppMessage::InstanceDeleteAnswered { instance, confirmed } => self.answer_instance_delete(instance, confirmed),
            LodestoneAppMessage::InstanceDeleted { instance, result } => self.instance_deleted(instance, result),
            LodestoneAppMessage::FolderOpened(result) => self.folder_opened(result),
            // Only `Navigate` exists today; when the page grows more
            // variants, add the delegation arm per the `pages` contract.
            LodestoneAppMessage::Settings(SettingsMessage::Navigate(page)) => self.navigate(page),
            LodestoneAppMessage::Showcase(ShowcaseMessage::Navigate(page)) => self.navigate(page),
            // Every page's language picker routes through the same handler.
            LodestoneAppMessage::Showcase(ShowcaseMessage::SelectLanguage(code)) => select_language(code),
            LodestoneAppMessage::Login(LoginMessage::SelectLanguage(code)) => select_language(code),
            // Sign-in and account handling lives in `auth.rs`.
            LodestoneAppMessage::Login(message) => self.update_login(message),
            LodestoneAppMessage::RestoreStart { username, refresh_token } => self.restore_sign_in(username, refresh_token),
            LodestoneAppMessage::CredentialsSaved(result) => self.credentials_saved(result),
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
            // Answering resolves the task `Alerts::show` handed out.
            LodestoneAppMessage::Alert(message) => {
                self.alerts.update(message);
                Task::none()
            }
            LodestoneAppMessage::Game(message) => self.game_message(message),
            LodestoneAppMessage::SessionRefreshed(result) => self.session_refreshed(result),
            LodestoneAppMessage::Perf(message) => {
                self.perf.update(message);
                Task::none()
            }
        }
    }

    /// Routes to `page` through the router's history.
    pub(super) fn navigate(&mut self, page: Page) -> Task<LodestoneAppMessage> {
        let from = self.router.current();
        if self.router.push(page) {
            info!("Navigated: {} → {}", from.title(), page.title());
            if page == Page::Discover {
                return self.open_discover();
            }
        } else {
            debug!("Navigation ignored: already on {}", page.title());
        }
        Task::none()
    }

    /// Loads the instance list from the database.
    pub(super) fn load_instances(&mut self) -> Task<LodestoneAppMessage> {
        let Some(db) = self.db.clone() else {
            return Task::none();
        };
        Task::perform(read_instances(db, self.paths.instances()), LodestoneAppMessage::InstancesLoaded)
    }
}

/// Makes `code` the active language, then saves the choice. The catalogue switches
/// at once, so the next view is already translated. The save runs on the blocking
/// pool and only logs a failure.
fn select_language(code: &'static str) -> Task<LodestoneAppMessage> {
    if !lang::set_active(code) {
        warn!("Language {code} is not bundled; keeping the current one");
        return Task::none();
    }
    let language = code.to_owned();
    Task::future(async move {
        match tokio::task::spawn_blocking(move || {
            // Read the file again, so the other choices (the Discover platform) are kept.
            let mut preferences = Preferences::load();
            preferences.language = Some(language);
            preferences.store()
        })
        .await
        {
            Ok(Ok(())) => {}
            Ok(Err(error)) => warn!("Saving the language choice failed: {error}"),
            Err(error) => warn!("Saving the language choice failed: {error}"),
        }
    })
    .discard()
}

/// Reads the instance rows, makes their file paths absolute, then counts each
/// instance's mods from its `mods/` folder, so the library card and the detail
/// page agree.
async fn read_instances(db: Database, instances_dir: PathBuf) -> Result<Vec<Instance>, String> {
    let _scope = perf::scope("io.instances");
    let mut instances = db.instances().await?;
    tokio::task::spawn_blocking(move || {
        for instance in &mut instances {
            instance.resolve_files(&instances_dir);
            instance.mod_count = files::count_mods(&instances_dir.join(&instance.directory_name)) as u32;
        }
        instances
    })
    .await
    .map_err(|error| error.to_string())
}
