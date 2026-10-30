//! The game controls the sidebar and the Windows title bar share. The indicator counts the running
//! games, and its panel lists each one with a button that starts another run of its instance, plus its
//! console and kill buttons.

use aurora_leaf::Icon;
use iced::widget::{Column, Row, Text, container, row};
use iced::{Background, Border, Element, Fill, Shadow, Vector, alignment};
use iced_color_extension::rgb;

use crate::app::components::PulseDot;
use crate::app::components::hint::hint;
use crate::app::components::icon_button::{self, IconButton};
use crate::app::components::popover::Popover;
use crate::app::lang;
use crate::app::launch::games::{GameBar, Run};
use crate::app::theme::{color, fonts};

/// The indicator's width. It sits inside the sidebar's 68 px rail with a margin either side.
pub const CHIP_WIDTH: f32 = 52.0;
/// The indicator's height, the same as the sidebar's 28 px buttons.
pub const CHIP_HEIGHT: f32 = 28.0;
/// The width of the runs panel.
const PANEL_WIDTH: f32 = 280.0;

/// The messages the game controls send, for the surface that shows them.
pub struct Actions<Message> {
    /// Opens or shuts the runs panel.
    pub toggle: Message,
    /// Shuts the runs panel.
    pub dismiss: Message,
    /// Stops the run with this serial.
    pub kill: fn(u64) -> Message,
    /// Opens the console of the run with this serial.
    pub console: fn(u64) -> Message,
    /// Starts another run of the instance with this id.
    pub launch: fn(String) -> Message,
}

/// The indicator and its runs panel: pressing the indicator opens the panel, which lists every running
/// game with its console and kill buttons, and a button that starts another run of its instance. With no
/// game running, the panel says so.
pub fn runs_popover<Message: Clone + 'static>(bar: &GameBar, open: bool, actions: &Actions<Message>) -> Element<'static, Message> {
    Popover::new(indicator(bar.runs.len()), runs_panel(&bar.runs, actions))
        .open(open)
        .on_toggle(actions.toggle.clone())
        .on_dismiss(actions.dismiss.clone())
        .into()
}

/// The indicator: a dot and the number of running games. The dot pulses while a game runs and is grey
/// when none does.
fn indicator<Message: 'static>(count: usize) -> Element<'static, Message> {
    let running = count > 0;
    let dot = if running {
        PulseDot::with_color(color::Primary::DEFAULT)
    } else {
        PulseDot::with_color(color::Text::DISABLED).pulsing(false)
    };
    let number = Text::new(count.to_string())
        .size(12)
        .font(fonts::JetBrainsMono::SEMI_BOLD)
        .color(if running { color::Text::HEADLINE } else { color::Text::MUTED });
    container(row![dot, number].spacing(6).align_y(alignment::Vertical::Center))
        .width(CHIP_WIDTH)
        .height(CHIP_HEIGHT)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .style(|_| container::Style {
            background: Some(Background::Color(rgb!(255, 255, 255, 4%))),
            border: Border {
                radius: 12.0.into(),
                width: 1.0,
                color: color::Surface::LINE,
            },
            ..container::Style::default()
        })
        .into()
}

/// The panel's body: one row per running game, or the note that none runs.
fn runs_panel<Message: Clone + 'static>(runs: &[Run], actions: &Actions<Message>) -> Element<'static, Message> {
    let body: Element<'static, Message> = if runs.is_empty() {
        Text::new(lang::t("game.none_running"))
            .size(12)
            .font(fonts::Inter::REGULAR)
            .color(color::Text::MUTED)
            .into()
    } else {
        Column::with_children(runs.iter().map(|run| run_row(run, actions)))
            .spacing(4)
            .width(Fill)
            .into()
    };
    container(body)
        .padding(8)
        .width(PANEL_WIDTH)
        .style(|_| container::Style {
            background: Some(Background::Color(color::Surface::BG3)),
            border: Border {
                radius: 12.0.into(),
                width: 1.0,
                color: color::Surface::LINE_STRONG,
            },
            shadow: Shadow {
                color: rgb!(0, 0, 0, 45%),
                offset: Vector::new(0.0, 10.0),
                blur_radius: 28.0,
            },
            ..container::Style::default()
        })
        .into()
}

/// One running game: its name, then the button that starts another run of its instance, its console
/// button and its kill button.
fn run_row<Message: Clone + 'static>(run: &Run, actions: &Actions<Message>) -> Element<'static, Message> {
    let items: Vec<Element<'static, Message>> = vec![
        Text::new(run.name.clone())
            .size(13)
            .font(fonts::Inter::MEDIUM)
            .color(color::Text::HEADLINE)
            .width(Fill)
            .into(),
        hint(
            IconButton::new(Icon::lucide().copy())
                .width(28)
                .height(28)
                .icon_size(12.0)
                .style(icon_button::SECONDARY_BUTTON_STYLE)
                .on_press((actions.launch)(run.instance.clone())),
            lang::t("game.new_process"),
        ),
        IconButton::new(Icon::lucide().terminal())
            .width(28)
            .height(28)
            .icon_size(12.0)
            .style(icon_button::SECONDARY_BUTTON_STYLE)
            .on_press((actions.console)(run.serial))
            .into(),
        IconButton::new(Icon::lucide().square())
            .width(28)
            .height(28)
            .icon_size(11.0)
            .style(icon_button::DANGER_BUTTON_STYLE)
            .on_press((actions.kill)(run.serial))
            .into(),
    ];
    Row::with_children(items).spacing(6).align_y(alignment::Vertical::Center).into()
}
