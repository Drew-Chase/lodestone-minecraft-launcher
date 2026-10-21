pub mod components;
pub mod pages;
pub mod router;
pub mod theme;
mod titlebar;

use crate::app::components::{NavItem, NavRail, TextButton};
use crate::app::pages::Page;
use crate::app::pages::home::{Home, Message as HomeMessage};
use crate::app::pages::settings::{Message as SettingsMessage, Settings};
use crate::app::pages::showcase::{Message as ShowcaseMessage, Showcase};
use crate::app::router::Router;
use crate::app::theme::color::Surface;
use crate::app::titlebar::{Titlebar, TitlebarMessage};
use iced::widget::{column, container, row, stack};
use iced::window::{Direction, Id, Level, Position};
use iced::{Element, Fill, Padding, Size, Subscription, Task, event, mouse, window};
use iced_borderless_window_resize_handle_widget::{ResizeHandles, drag_resize};
use tracing::{debug, info};

#[derive(Default)]
pub struct LodestoneLauncher {
    id: Option<Id>,
    titlebar: Titlebar,
    maximized: bool,
    router: Router,
    home: Home,
    settings: Settings,
    showcase: Showcase,
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
    Home(HomeMessage),
    Settings(SettingsMessage),
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
            // Only `Navigate` exists today; when the page grows more
            // variants, add the delegation arm per the `pages` contract.
            LodestoneAppMessage::Home(HomeMessage::Navigate(page)) => self.navigate(page),
            // Only `Navigate` exists today; when the page grows more
            // variants, add the delegation arm per the `pages` contract.
            LodestoneAppMessage::Settings(SettingsMessage::Navigate(page)) => self.navigate(page),
            LodestoneAppMessage::Showcase(ShowcaseMessage::Navigate(page)) => self.navigate(page),
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

    pub fn view(&self) -> Element<'_, LodestoneAppMessage> {
        stack![
            column![
                self.titlebar.view().map(LodestoneAppMessage::TitlebarMessage),
                row![self.nav_rail(), self.body()].width(Fill).height(Fill),
            ]
            .width(Fill)
            .height(Fill),
            ResizeHandles::new(LodestoneAppMessage::WindowStartResize)
                .gutter(4.0)
                .enabled(!self.maximized)
        ]
        .width(Fill)
        .height(Fill)
        .into()
    }

    /// Left edge chrome — one rail slot per [`Page::RAIL_PAGES`] entry.
    fn nav_rail(&self) -> Element<'_, LodestoneAppMessage> {
        let active = Page::RAIL_PAGES.iter().position(|page| *page == self.router.current());
        container(
            NavRail::new(Page::RAIL_PAGES.map(|page| NavItem::new(page.icon())))
                .active(active.unwrap_or(0))
                .on_select(|index| LodestoneAppMessage::Navigate(Page::RAIL_PAGES[index])),
        )
        .height(Fill)
        .align_y(iced::alignment::Vertical::Top)
        .padding(Padding::new(0.0).left(12).top(12))
        .into()
    }

    /// History chrome — back/forward over the router's stacks.
    fn history_controls(&self) -> Element<'_, LodestoneAppMessage> {
        row![
            TextButton::ghost("Back")
                .disabled(!self.router.can_go_back())
                .on_press(LodestoneAppMessage::NavBack),
            TextButton::ghost("Forward")
                .disabled(!self.router.can_go_forward())
                .on_press(LodestoneAppMessage::NavForward),
        ]
        .spacing(8)
        .padding(Padding::new(0.0).left(16).top(12))
        .into()
    }

    /// The page area: history controls above the router's current page.
    fn body(&self) -> Element<'_, LodestoneAppMessage> {
        column![
            self.history_controls(),
            container(self.outlet()).width(Fill).height(Fill).style(|_| container::Style {
                background: Some(iced::Background::Color(Surface::APPLICATION)),
                ..Default::default()
            }),
        ]
        .width(Fill)
        .height(Fill)
        .into()
    }

    /// The `<Outlet/>` — whichever page the router points at.
    fn outlet(&self) -> Element<'_, LodestoneAppMessage> {
        match self.router.current() {
            Page::Home => self.home.view().map(LodestoneAppMessage::Home),
            Page::Settings => self.settings.view().map(LodestoneAppMessage::Settings),
            Page::Showcase => self.showcase.view().map(LodestoneAppMessage::Showcase),
        }
    }

    pub fn subscription(&self) -> Subscription<LodestoneAppMessage> {
        Subscription::batch([
            window::resize_events().map(|(id, size)| LodestoneAppMessage::WindowSizeChanged(id, size)),
            event::listen_with(Self::mouse_navigation),
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
