//! The root `update`: window chrome, navigation, page delegation, the
//! database, and the probes. Sign-in and account messages are routed to
//! `auth.rs`.

use crate::app::face;
use crate::app::pages::Page;
use crate::app::pages::instance::Message as InstanceMessage;
use crate::app::pages::library::Message as LibraryMessage;
use crate::app::pages::library::instance::Instance;
use crate::app::pages::settings::Message as SettingsMessage;
use crate::app::pages::showcase::Message as ShowcaseMessage;
use chrono::Local;
use iced::widget::image;
use iced::{Size, Task, window};
use iced_borderless_window_resize_handle_widget::drag_resize;
use tracing::{debug, info, warn};

use super::{LodestoneAppMessage, LodestoneLauncher, NavGroup, probe_enabled};

impl LodestoneLauncher {
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
        }
    }

    /// Routes to `page` through the router's history.
    pub(super) fn navigate(&mut self, page: Page) -> Task<LodestoneAppMessage> {
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
}
