//! Startup: the initial state and boot tasks, and the main window settings.

use crate::app::database::Database;
use crate::app::install::catalog;
use crate::app::lang;
use crate::app::pages::library::LibraryView;
use crate::app::pages::login::Message as LoginMessage;
use crate::app::paths::{self, LauncherPaths};
use crate::app::preferences::Preferences;
use iced::window::{Level, Position};
use iced::{Size, Task, window};
use std::path::PathBuf;
use tracing::{info, warn};

use super::{LodestoneAppMessage, LodestoneLauncher, probe_enabled, updater};

impl LodestoneLauncher {
    pub fn boot() -> (Self, Task<LodestoneAppMessage>) {
        // The saved language goes live before the first view, so the first frame is already translated.
        let preferences = Preferences::load();
        if let Some(code) = preferences.language.as_deref()
            && !lang::set_active(code)
        {
            info!("Saved language {code} is not bundled; using the default");
        }
        preferences.settings.apply();
        let id = window::latest();
        // The data folder is read before the database opens, so every path the launcher builds starts in the right place. A
        // portable launcher or a development run keeps its fixed folders, so the folders saved for it do not apply.
        let fixed = paths::data_folder_fixed();
        let data = LauncherPaths::new(paths::resolve_data_root(preferences.data_folder.as_deref()))
            .with_instances(saved_folder(fixed, &preferences.instances_folder))
            .with_logs(saved_folder(fixed, &preferences.logs_folder))
            .with_cache(saved_folder(fixed, &preferences.cache_folder));
        let mut app = Self {
            settings_values: preferences.settings,
            paths: data,
            setup_finished: preferences.setup_finished,
            ..Self::default()
        };
        app.library.set_hero_rule(app.settings_values.hero);
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
        // Open the launcher database — migrations run on connect.
        tasks.push(Task::perform(Database::connect(), LodestoneAppMessage::DatabaseReady));
        // The update check runs at launch, in the background, and then every `updater::CHECK_EVERY`. `updater::enabled` says when
        // it is skipped.
        if updater::enabled(app.portable.is_some()) {
            tasks.push(updater::check(app.settings_values.beta_channel, None));
        }
        // The Minecraft version list loads at launch when Auto-update game versions is on, so New Instance opens with it.
        if app.settings_values.auto_update_game_versions {
            tasks.push(Task::perform(catalog::releases(), |result| {
                LodestoneAppMessage::VersionsPrefetched(result.map(std::sync::Arc::new))
            }));
        }
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
        // The account that plays by default enters the launcher at once. A Microsoft sign-in is restored in the background.
        tasks.push(app.restore_at_boot());
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
                // Files dragged from Explorer reach the Import Instance dialog. iced's default turns this on too.
                drag_and_drop: true,
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
            icon: app_icon(),
            // A close is answered by `CloseRequested`, which asks before a running game is left unwatched.
            exit_on_close_request: false,
        }
    }
}

/// The folder the preferences saved, unless the launcher keeps fixed folders, which a saved choice does not change.
fn saved_folder(fixed: bool, folder: &Option<PathBuf>) -> Option<PathBuf> {
    if fixed { None } else { folder.clone() }
}

/// The 512px application mark, embedded in the binary.
const APP_ICON: &[u8] = include_bytes!("../../../../res/icon/icon.png");

/// The application mark as RGBA pixels, with its width and height.
fn icon_pixels() -> Result<(Vec<u8>, u32, u32), image::ImageError> {
    let icon = image::load_from_memory_with_format(APP_ICON, image::ImageFormat::Png)?.into_rgba8();
    let (width, height) = icon.dimensions();
    Ok((icon.into_raw(), width, height))
}

/// The window's icon. A decode failure logs a warning and leaves the window without one.
fn app_icon() -> Option<window::icon::Icon> {
    let (rgba, width, height) = icon_pixels()
        .inspect_err(|error| warn!("could not decode the application icon: {error}"))
        .ok()?;
    window::icon::from_rgba(rgba, width, height).ok()
}

#[cfg(test)]
#[path = "../../tests/app/boot.rs"]
mod tests;
