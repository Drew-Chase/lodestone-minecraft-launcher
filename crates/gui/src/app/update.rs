//! The root `update`: window chrome, navigation, page delegation, the
//! database, and the probes. Sign-in and account messages are routed to
//! `auth.rs`.

use crate::app::database::Database;
use crate::app::lang;
use crate::app::launch::GameMessage;
use crate::app::launch::pidfile;
use crate::app::pages::Page;
use crate::app::pages::instance::Message as InstanceMessage;
use crate::app::pages::instance::Tab;
use crate::app::pages::instance::files;
use crate::app::pages::instance::settings::Message as InstanceSettingsMessage;
use crate::app::pages::instance::{mods, screenshots, worlds};
use crate::app::pages::library::Message as LibraryMessage;
use crate::app::pages::library::instance::Instance;
use crate::app::pages::login::Message as LoginMessage;
use crate::app::pages::settings::Message as SettingsMessage;
use crate::app::pages::settings::accounts::AccountAction;
use crate::app::pages::showcase::Message as ShowcaseMessage;
use crate::app::pages::worlds::Message as WorldsMessage;
use crate::app::perf;
use crate::app::preferences::{Preferences, SettingsValues};
use crate::app::world_actions::WorldMessage;
use iced::{Size, Task, window};
use iced_borderless_window_resize_handle_widget::drag_resize;
use std::path::PathBuf;
use tracing::{debug, info, warn};

use super::{LodestoneAppMessage, LodestoneLauncher, NavGroup, downloads, probe_enabled, updater};

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
                    return self.rescan_mods_on_return();
                }
                Task::none()
            }
            LodestoneAppMessage::NavForward => {
                if self.router.go_forward() {
                    info!("Navigated forward to {}", self.router.current().title());
                    return self.rescan_mods_on_return();
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
            // The update button in the header is the updater's to handle.
            LodestoneAppMessage::Library(LibraryMessage::UpdatePressed) => self.updater_message(updater::Message::Pressed),
            LodestoneAppMessage::Downloads(message) => self.downloads_message(message),
            LodestoneAppMessage::Export(message) => self.export_message(message),
            LodestoneAppMessage::Portable(message) => self.portable_message(message),
            LodestoneAppMessage::Updater(message) => self.updater_message(message),
            LodestoneAppMessage::Discover(message) => self.discover_message(message),
            LodestoneAppMessage::Resource(message) => self.resource_message(message),
            LodestoneAppMessage::ModSourcesLoaded(Ok(records)) => {
                info!("Loaded {} mod(s) from a platform", records.len());
                self.discover.set_added(&records);
                Task::none()
            }
            LodestoneAppMessage::ModSourcesLoaded(Err(error)) => {
                warn!("Loading the mods from platforms failed: {error}");
                Task::none()
            }
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
                    return self.rescan_mods_on_return();
                }
                Task::none()
            }
            // The page's reveal and delete actions need the root's IO and alerts, so they are routed here.
            LodestoneAppMessage::Instance(InstanceMessage::RevealFolder) => self.reveal_current_instance(),
            LodestoneAppMessage::Instance(InstanceMessage::OpenSource) => self.open_current_source(),
            // The Mods tab's Add opens Discover on the instance's mods.
            LodestoneAppMessage::Instance(InstanceMessage::AddMods) => self.add_mods_from_instance(),
            LodestoneAppMessage::Instance(InstanceMessage::Menu(action)) => self.current_instance_menu(action),
            LodestoneAppMessage::SourceOpened(result) => self.source_opened(result),
            LodestoneAppMessage::Instance(InstanceMessage::Settings(InstanceSettingsMessage::RevealFolder)) => self.reveal_current_instance(),
            LodestoneAppMessage::Instance(InstanceMessage::Settings(InstanceSettingsMessage::DeleteInstance)) => self.ask_current_instance_delete(),
            // The Settings tab's edits change the instance, which the root applies, saves and hands back to the page.
            LodestoneAppMessage::Instance(InstanceMessage::Settings(InstanceSettingsMessage::Edit(edit))) => self.edit_instance(edit),
            LodestoneAppMessage::Instance(InstanceMessage::Settings(InstanceSettingsMessage::ResetSettings)) => self.ask_instance_reset(),
            LodestoneAppMessage::Instance(InstanceMessage::Settings(InstanceSettingsMessage::Reinstall)) => self.reinstall_instance(),
            LodestoneAppMessage::InstanceSaved { instance, result } => self.instance_saved(instance, result),
            LodestoneAppMessage::InstanceResetAnswered { instance, confirmed } => self.instance_reset_answered(instance, confirmed),
            LodestoneAppMessage::SettingsSaved(result) => self.settings_saved(result),
            LodestoneAppMessage::VersionsPrefetched(result) => self.versions_prefetched(result),
            LodestoneAppMessage::AccountSwitched(index, result) => self.account_switched(index, result),
            LodestoneAppMessage::ProfileChangeConfirmed(confirmed) => self.profile_change_confirmed(confirmed),
            LodestoneAppMessage::LocalDataAnswered(confirmed) => self.local_data_answered(confirmed),
            LodestoneAppMessage::ProfileChanged(result) => self.profile_changed(result),
            LodestoneAppMessage::Skins(message) => self.update_skins(message),
            LodestoneAppMessage::SkinsLoaded(scan) => {
                self.skins.set_scan(scan);
                self.previews_task()
            }
            LodestoneAppMessage::PreviewsReady(drawn) => {
                self.skins.set_previews(drawn);
                // The frame on show may still need drawing, so the next drawing starts now rather than at the next tick.
                self.previews_task()
            }
            LodestoneAppMessage::CapesLoaded(faces) => {
                self.skins.set_cape_faces(faces);
                Task::none()
            }
            LodestoneAppMessage::ProfileRefreshed(result) => self.profile_refreshed(result),
            LodestoneAppMessage::SkinFileMoved(result) => self.skin_file_moved(result),
            LodestoneAppMessage::SpeedTick => {
                self.speed.tick(lodestone_net::bytes_received());
                Task::none()
            }
            LodestoneAppMessage::ReinstallFinished { job, result } => self.reinstall_finished(job, result),
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
            LodestoneAppMessage::Instance(InstanceMessage::Worlds(worlds::Message::Menu { path, action })) => {
                self.shown_world(|instance| WorldMessage::Menu { instance, path, action })
            }
            LodestoneAppMessage::Instance(InstanceMessage::Worlds(worlds::Message::Import)) => {
                self.shown_world(|instance| WorldMessage::Import { instance })
            }
            LodestoneAppMessage::Instance(InstanceMessage::Worlds(worlds::Message::NewWorld)) => {
                self.shown_world(|instance| WorldMessage::NewWorld { instance })
            }
            LodestoneAppMessage::World(message) => self.world_message(message),
            LodestoneAppMessage::WorldPinsLoaded(rows) => self.world_pins_loaded(rows),
            LodestoneAppMessage::Worlds(WorldsMessage::World(message)) => self.world_message(message),
            LodestoneAppMessage::Worlds(WorldsMessage::Pick { menu, instance }) => {
                self.worlds.update(WorldsMessage::DismissMenu);
                self.picked_instance(menu, instance)
            }
            LodestoneAppMessage::Worlds(message) => {
                self.worlds.update(message);
                Task::none()
            }
            // The Mods tab's folder reads are checked against the platforms here, and its actions are carried out here.
            LodestoneAppMessage::Instance(InstanceMessage::Scanned { generation, files }) if generation == self.instance_page.generation() => {
                let apply = self
                    .instance_page
                    .update(InstanceMessage::Scanned { generation, files })
                    .map(LodestoneAppMessage::Instance);
                Task::batch([apply, self.mods_scanned(generation)])
            }
            LodestoneAppMessage::Instance(InstanceMessage::TabSelected(tab)) => {
                let select = self
                    .instance_page
                    .update(InstanceMessage::TabSelected(tab))
                    .map(LodestoneAppMessage::Instance);
                Task::batch([select, self.mods_checks_due()])
            }
            LodestoneAppMessage::Instance(InstanceMessage::Mods(mods::Message::Open(file_name))) => self.mod_row_pressed(file_name),
            LodestoneAppMessage::Instance(InstanceMessage::Mods(mods::Message::Menu { file_name, action })) => {
                self.mod_menu_action(file_name, action)
            }
            LodestoneAppMessage::Instance(InstanceMessage::Mods(mods::Message::Dialog(message))) if !message.is_local() => {
                self.mod_dialog_action(message)
            }
            LodestoneAppMessage::ModActions(message) => self.mod_actions_message(message),
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
                Task::batch([
                    sweep,
                    reattach,
                    self.load_shared(),
                    self.load_mod_sources(),
                    self.load_world_pins(),
                    self.portable_database_ready(),
                ])
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
            LodestoneAppMessage::Settings(SettingsMessage::Save) => self.save_settings(),
            LodestoneAppMessage::Settings(SettingsMessage::OpenInstancesFolder) => super::instances::reveal_folder(self.paths.instances()),
            LodestoneAppMessage::Settings(SettingsMessage::OpenDataFolder) => super::instances::reveal_folder(self.paths.root()),
            LodestoneAppMessage::Settings(SettingsMessage::ClearLocalData) => self.ask_clear_local_data(),
            LodestoneAppMessage::Settings(SettingsMessage::Account(action)) => match action {
                AccountAction::MakePrimary(index) => self.switch_account(index),
                AccountAction::SignOut(index) => self.remove_account(index),
                AccountAction::Add => self.add_account(),
            },
            LodestoneAppMessage::Settings(message) => self.settings.update(message).map(LodestoneAppMessage::Settings),
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
            LodestoneAppMessage::SidebarNav {
                group: NavGroup::Bottom,
                index: 0,
            } => self.navigate(Page::Settings),
            LodestoneAppMessage::SidebarNav { group, index } => {
                debug!("Sidebar nav item {group:?}#{index} has no page yet");
                Task::none()
            }
            LodestoneAppMessage::AppearanceReady(Ok(appearance)) => self.apply_appearance(appearance),
            LodestoneAppMessage::AppearanceReady(Err(error)) => {
                warn!("Player appearance unavailable: {error}");
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
            if page == Page::Worlds {
                return self.open_worlds();
            }
            if page == Page::Skins {
                return self.open_skins();
            }
            if page == Page::Settings {
                return self.open_settings();
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

    /// Opens the Settings page: its lists load, and its draft starts from the saved values.
    pub(super) fn open_settings(&mut self) -> Task<LodestoneAppMessage> {
        self.settings.sync(&self.settings_values, &self.paths);
        self.settings.open().map(LodestoneAppMessage::Settings)
    }

    /// Commits the Settings page's draft. The values become the saved ones, the palette and the instance pages follow
    /// them, and the write runs in the background.
    pub(super) fn save_settings(&mut self) -> Task<LodestoneAppMessage> {
        let values = self.settings.commit();
        values.apply();
        self.settings_values = values.clone();
        self.refresh_instance_page();
        match self.settings_saves.request(SETTINGS_KEY, values) {
            Some(values) => write_settings(values),
            None => Task::none(),
        }
    }

    /// Finishes a write of the settings. The newest values that arrived during it are written next. A failed write is
    /// logged, and the values stay as the page shows them.
    pub(super) fn settings_saved(&mut self, result: Result<(), String>) -> Task<LodestoneAppMessage> {
        if let Err(error) = &result {
            warn!("Saving the settings failed: {error}");
        }
        self.settings_saves.complete(&SETTINGS_KEY).map_or_else(Task::none, write_settings)
    }
}

/// The key the settings are saved under. There is one set of values, so one write at a time.
const SETTINGS_KEY: &str = "settings";

/// Writes the settings into the preferences, keeping the language and the Discover platform as the file has them.
fn write_settings(values: SettingsValues) -> Task<LodestoneAppMessage> {
    Task::perform(
        async move {
            match tokio::task::spawn_blocking(move || {
                let mut preferences = Preferences::load();
                preferences.settings = values;
                preferences.store()
            })
            .await
            {
                Ok(saved) => saved,
                Err(error) => Err(error.to_string()),
            }
        },
        LodestoneAppMessage::SettingsSaved,
    )
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
