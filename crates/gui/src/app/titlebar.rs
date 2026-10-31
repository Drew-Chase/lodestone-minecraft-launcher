use crate::app::LodestoneAppMessage;
#[cfg(windows)]
use crate::app::LodestoneLauncher;
#[cfg(windows)]
use crate::app::components::brand_tag::brand_tag;
#[cfg(windows)]
use crate::app::components::game_controls;
#[cfg(windows)]
use crate::app::components::icon_button;
#[cfg(windows)]
use crate::app::components::icon_button::IconButton;
#[cfg(windows)]
use crate::app::launch::GameMessage;
use crate::app::launch::games::GameBar;
#[cfg(windows)]
use crate::app::launch::games::RunsPlace;
#[cfg(windows)]
use crate::app::theme::color;
#[cfg(windows)]
use aurora_leaf::Icon;
use iced::widget::row;
#[cfg(windows)]
use iced::widget::{Space, Text, container, mouse_area, stack};
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
    /// Opens or shuts the panel of running games under the title bar.
    ToggleRuns,
    /// Shuts the panel of running games.
    DismissRuns,
    /// Stops the run with this serial.
    KillRun(u64),
    /// Opens the console of the run with this serial.
    ConsoleRun(u64),
    /// Starts another run of the instance `id`.
    LaunchNew(String),
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
            TitlebarMessage::ToggleRuns => Task::done(LodestoneAppMessage::Game(GameMessage::ToggleRuns(RunsPlace::Titlebar))),
            TitlebarMessage::DismissRuns => Task::done(LodestoneAppMessage::Game(GameMessage::DismissRuns(RunsPlace::Titlebar))),
            TitlebarMessage::KillRun(serial) => Task::done(LodestoneAppMessage::Game(GameMessage::KillRun(serial))),
            TitlebarMessage::ConsoleRun(serial) => Task::done(LodestoneAppMessage::Game(GameMessage::ConsoleRun(serial))),
            TitlebarMessage::LaunchNew(id) => Task::done(LodestoneAppMessage::Game(GameMessage::LaunchNew(id))),
        }
    }
    #[cfg(not(target_os = "windows"))]
    pub fn update(&mut self, _message: TitlebarMessage, _window_id: window::Id) -> Task<LodestoneAppMessage> {
        Task::none()
    }

    /// The bar. While a game runs, the game controls sit before the window controls: the indicator, whose
    /// panel lists every running game with its new-process, console and kill buttons.
    #[cfg(windows)]
    pub fn view(&self, can_go_back: bool, can_go_forward: bool, transparent: bool, game: &GameBar, runs_open: bool) -> Element<'_, TitlebarMessage> {
        let window_buttons = row![
            IconButton::new(Icon::material_symbols().minimize_rounded())
                .width(47)
                .height(Fill)
                .style(icon_button::SECONDARY_BUTTON_STYLE)
                .radius(0.0)
                .on_press(TitlebarMessage::ToggleMinimize),
            IconButton::new(Icon::material_symbols().square_outline_rounded())
                .width(47)
                .icon_size(11.5)
                .height(Fill)
                .style(icon_button::SECONDARY_BUTTON_STYLE)
                .radius(0.0)
                .on_press(TitlebarMessage::ToggleMaximized),
            IconButton::new(Icon::material_symbols().close_rounded())
                .width(47)
                .height(Fill)
                .style(icon_button::DANGER_BUTTON_STYLE)
                .radius(0.0)
                .on_press(TitlebarMessage::Close),
        ]
        .height(Fill);
        let window_buttons = row![Self::game_buttons(game, runs_open), window_buttons].height(Fill);

        let bar: Element<'_, TitlebarMessage> = if transparent {
            // Chrome floats over the login backdrop. The drag surface spans the whole bar,
            // and the tag is centred over it. The window buttons sit on top, so their clicks
            // never start a drag.
            stack![
                mouse_area(Space::new().width(Fill).height(Fill))
                    .on_double_click(TitlebarMessage::ToggleMaximized)
                    .on_press(TitlebarMessage::StartDragWindow),
                container(brand_tag())
                    .width(Fill)
                    .height(Fill)
                    .align_x(iced::alignment::Horizontal::Center)
                    .align_y(iced::alignment::Vertical::Center),
                row![Space::new().width(Fill), window_buttons].width(Fill).height(Fill),
            ]
            .width(Fill)
            .height(Fill)
            .into()
        } else {
            row![
                IconButton::new(Icon::lucide().chevron_left())
                    .width(34)
                    .height(Fill)
                    .icon_size(13.0)
                    .style(icon_button::SECONDARY_BUTTON_STYLE)
                    .disabled(!can_go_back)
                    .radius(0.0)
                    .on_press(TitlebarMessage::NavBack),
                IconButton::new(Icon::lucide().chevron_right())
                    .width(34)
                    .height(Fill)
                    .icon_size(13.0)
                    .style(icon_button::SECONDARY_BUTTON_STYLE)
                    .disabled(!can_go_forward)
                    .radius(0.0)
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
                window_buttons,
            ]
            .width(Fill)
            .height(Fill)
            .into()
        };

        container(bar)
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
    pub fn view(
        &self,
        _can_go_back: bool,
        _can_go_forward: bool,
        _transparent: bool,
        _game: &GameBar,
        _runs_open: bool,
    ) -> Element<'_, TitlebarMessage> {
        row![].into()
    }

    /// The game controls on the bar, or an empty row while no game runs: the indicator, whose panel lists
    /// the running games with their new-process, console and kill buttons.
    #[cfg(windows)]
    fn game_buttons(bar: &GameBar, runs_open: bool) -> Element<'static, TitlebarMessage> {
        if bar.runs.is_empty() {
            return row![].height(Fill).into();
        }
        let actions = game_controls::Actions {
            toggle: TitlebarMessage::ToggleRuns,
            dismiss: TitlebarMessage::DismissRuns,
            kill: TitlebarMessage::KillRun,
            console: TitlebarMessage::ConsoleRun,
            launch: TitlebarMessage::LaunchNew,
        };
        Self::slot(game_controls::runs_popover(bar, runs_open, &actions))
    }

    /// Centres one game control vertically in the bar.
    #[cfg(windows)]
    fn slot(content: Element<'static, TitlebarMessage>) -> Element<'static, TitlebarMessage> {
        container(content).height(Fill).center_y(Fill).into()
    }
}
