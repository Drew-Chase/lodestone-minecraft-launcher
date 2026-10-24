use crate::app::LodestoneAppMessage;
#[cfg(windows)]
use crate::app::LodestoneLauncher;
#[cfg(windows)]
use crate::app::components::icon_button;
#[cfg(windows)]
use crate::app::components::icon_button::IconButton;
#[cfg(windows)]
use crate::app::theme::color;
#[cfg(windows)]
use aurora_leaf::Icon;
use iced::widget::row;
#[cfg(windows)]
use iced::widget::{Space, Text, container, mouse_area};
#[cfg(windows)]
use iced::{Background, Fill, Padding};
use iced::{Element, Task, window};
#[cfg(windows)]
use tracing::{debug, info};

#[derive(Debug, Clone, Default)]
pub struct Titlebar;
#[cfg(windows)]
#[derive(Debug, Clone)]
pub enum TitlebarMessage {
    /// Step the router's history back (mouse chrome or future hotkey).
    NavBack,
    /// Step the router's history forward.
    NavForward,
    Close,
    ToggleMaximized,
    ToggleMinimize,
    StartDragWindow,
}

#[cfg(not(windows))]
#[derive(Debug, Clone)]
pub enum TitlebarMessage {}

impl Titlebar {
    #[cfg(windows)]
    pub fn update(&mut self, message: TitlebarMessage, window_id: window::Id) -> Task<LodestoneAppMessage> {
        match message {
            TitlebarMessage::NavBack => Task::done(LodestoneAppMessage::NavBack),
            TitlebarMessage::NavForward => Task::done(LodestoneAppMessage::NavForward),
            TitlebarMessage::ToggleMaximized => {
                info!("Toggling maximize on window {window_id}");
                window::toggle_maximize(window_id)
            }
            TitlebarMessage::ToggleMinimize => {
                info!("Minimizing window {window_id}");
                window::minimize(window_id, true)
            }
            TitlebarMessage::Close => {
                info!("Closing window {window_id}");
                window::close(window_id)
            }
            TitlebarMessage::StartDragWindow => {
                debug!("Starting drag on window {window_id}");
                window::drag(window_id)
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    pub fn update(&mut self, _message: TitlebarMessage, _window_id: window::Id) -> Task<LodestoneAppMessage> {
        Task::none()
    }

    #[cfg(windows)]
    pub fn view(&self, can_go_back: bool, can_go_forward: bool, transparent: bool) -> Element<'_, TitlebarMessage> {
        container(row![
            if transparent {
                // Chrome floats over the login backdrop: a bare drag surface
                // with the window buttons on the right (the login page owns
                // the centered brand line).
                row![
                    mouse_area(Space::new().width(Fill).height(Fill))
                        .on_double_click(TitlebarMessage::ToggleMaximized)
                        .on_press(TitlebarMessage::StartDragWindow),
                ]
                .width(Fill)
                .height(Fill)
            } else {
                row![
                    IconButton::new(Icon::lucide().chevron_left())
                        .width(34)
                        .height(Fill)
                        .icon_size(13.0)
                        .style(icon_button::SECONDARY_BUTTON_STYLE)
                        .disabled(!can_go_back)
                        .on_press(TitlebarMessage::NavBack),
                    IconButton::new(Icon::lucide().chevron_right())
                        .width(34)
                        .height(Fill)
                        .icon_size(13.0)
                        .style(icon_button::SECONDARY_BUTTON_STYLE)
                        .disabled(!can_go_forward)
                        .on_press(TitlebarMessage::NavForward),
                    mouse_area(row![
                        container(
                            Text::new(LodestoneLauncher::TITLE)
                                .size(18)
                                .font(crate::app::theme::fonts::Inter::EXTRA_BOLD)
                        )
                        .padding(Padding::new(0.0).left(8).right(8).top(4))
                        .width(Fill)
                        .height(Fill),
                    ])
                    .on_double_click(TitlebarMessage::ToggleMaximized)
                    .on_press(TitlebarMessage::StartDragWindow),
                ]
                .width(Fill)
                .height(Fill)
            },
            IconButton::new(Icon::material_symbols().minimize_rounded())
                .width(47)
                .height(Fill)
                .style(icon_button::SECONDARY_BUTTON_STYLE)
                .on_press(TitlebarMessage::ToggleMinimize),
            IconButton::new(Icon::material_symbols().square_outline_rounded())
                .width(47)
                .icon_size(11.5)
                .height(Fill)
                .style(icon_button::SECONDARY_BUTTON_STYLE)
                .on_press(TitlebarMessage::ToggleMaximized),
            IconButton::new(Icon::material_symbols().close_rounded())
                .width(47)
                .height(Fill)
                .style(icon_button::DANGER_BUTTON_STYLE)
                .on_press(TitlebarMessage::Close),
        ])
        .width(Fill)
        .height(32)
        .style(move |_| {
            if transparent {
                // The login backdrop shows through the chrome.
                container::Style::default()
            } else {
                container::Style {
                    background: Some(Background::from(color::Surface::TITLEBAR)),
                    ..Default::default()
                }
            }
        })
        .into()
    }
    #[cfg(not(target_os = "windows"))]
    pub fn view(&self, _can_go_back: bool, _can_go_forward: bool, _transparent: bool) -> Element<'_, TitlebarMessage> {
        row![].into()
    }
}
