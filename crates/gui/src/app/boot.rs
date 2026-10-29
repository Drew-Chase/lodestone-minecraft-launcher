//! Startup: the initial state and boot tasks, and the main window settings.

use crate::app::database::Database;
use crate::app::pages::library::LibraryView;
use crate::app::pages::login::Message as LoginMessage;
use iced::window::{Level, Position};
use iced::{Size, Task, window};
use tracing::info;

use super::{LodestoneAppMessage, LodestoneLauncher, probe_enabled};

impl LodestoneLauncher {
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
        // Open the launcher database — migrations run on connect.
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
