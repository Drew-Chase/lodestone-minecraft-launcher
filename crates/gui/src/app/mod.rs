pub mod components;
pub mod pages;
pub mod theme;
mod titlebar;

use crate::app::pages::showcase::{Message as ShowcaseMessage, Showcase};
use crate::app::theme::color::Surface;
use crate::app::titlebar::{Titlebar, TitlebarMessage};
use iced::widget::{column, container, stack};
use iced::window::{Direction, Id, Level, Position};
use iced::{Element, Fill, Size, Subscription, Task, window};
use iced_borderless_window_resize_handle_widget::{ResizeHandles, drag_resize};
use tracing::{debug, info};

#[derive(Default)]
pub struct LodestoneLauncher {
    id: Option<Id>,
    titlebar: Titlebar,
    maximized: bool,
    showcase: Showcase,
}

#[derive(Debug)]
pub enum LodestoneAppMessage {
    WindowCreation(Option<Id>),
    WindowStartResize(Direction),
    WindowSizeChanged(Id, Size),
    TitlebarMessage(TitlebarMessage),
    WindowMaximized(bool),
    Showcase(ShowcaseMessage),
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
        let app = Self { ..Self::default() };
        (app, Task::batch([id.map(LodestoneAppMessage::WindowCreation)]))
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
            LodestoneAppMessage::WindowSizeChanged(id, _size) => {
                debug!("Window {id} size changed");
                window::is_maximized(id).map(LodestoneAppMessage::WindowMaximized)
            }
            LodestoneAppMessage::WindowMaximized(maximized) => {
                if self.maximized != maximized {
                    info!("Window maximized state changed: {maximized}");
                }
                self.maximized = maximized;
                Task::none()
            }
            LodestoneAppMessage::Showcase(message) => {
                self.showcase.update(message);
                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<'_, LodestoneAppMessage> {
        stack![
            column![
                self.titlebar.view().map(LodestoneAppMessage::TitlebarMessage),
                container(
                    container(self.showcase.view().map(LodestoneAppMessage::Showcase))
                        .width(Fill)
                        .height(Fill)
                )
                .width(Fill)
                .height(Fill)
                .style(|_| container::Style {
                    background: Some(iced::Background::Color(Surface::APPLICATION)),
                    ..Default::default()
                })
            ],
            ResizeHandles::new(LodestoneAppMessage::WindowStartResize)
                .gutter(4.0)
                .enabled(!self.maximized)
        ]
        .width(Fill)
        .height(Fill)
        .into()
    }

    pub fn subscription(&self) -> Subscription<LodestoneAppMessage> {
        Subscription::batch([window::resize_events().map(|(id, size)| LodestoneAppMessage::WindowSizeChanged(id, size))])
    }

    pub fn window_settings() -> window::Settings {
        window::Settings {
            size: Size::new(600f32, 600f32),
            min_size: Some(Size::new(680f32, 440f32)),
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
