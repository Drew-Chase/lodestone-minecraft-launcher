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
#[cfg(windows)]
use iced::widget::{Text, container, mouse_area};
use iced::widget::{row, text};
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
    pub fn view(&self) -> Element<'_, TitlebarMessage> {
        container(row![
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
        .style(|_| container::Style {
            background: Some(Background::from(color::Surface::TITLEBAR)),
            ..Default::default()
        })
        .into()
    }
    #[cfg(not(target_os = "windows"))]
    pub fn view(&self) -> Element<'_, TitlebarMessage> {
        row![].into()
    }
}
