//! Logs tab — the live `latest.log` panel over a bounded line buffer.
//!
//! The buffer keeps the newest [`MAX_LINES`] lines. While following, the
//! scroller stays pinned to the bottom so new lines come into view; the LIVE
//! pill toggles that, and the page re-reads the file on a timer while it is on.
//! Export is display-only until the root owns file IO.

use std::collections::VecDeque;

use aurora_leaf::Icon;
use iced::widget::{Space, Text, column, container, row};
use iced::{Background, Color, Element, Fill, Padding, alignment};
use iced_color_extension::rgb;

use crate::app::components::TextButton;
use crate::app::components::badge::Badge;
use crate::app::components::button::Variant;
use crate::app::components::panel::{hairline, panel_with_padding};
use crate::app::components::scrollbar;
use crate::app::theme::{color, fonts, utility};

/// How many lines the buffer keeps; the oldest are dropped first.
pub const MAX_LINES: usize = 500;

/// Column widths of the timestamp and level tag, so messages line up.
const TIME_WIDTH: f32 = 66.0;
const TAG_WIDTH: f32 = 64.0;
/// Offset that puts the caret under the message column.
const CARET_OFFSET: f32 = TIME_WIDTH + TAG_WIDTH + 28.0;

/// The severity a log line is tagged with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    /// The bracketed tag drawn before the message.
    pub fn tag(self) -> &'static str {
        match self {
            LogLevel::Debug => "[DEBUG]",
            LogLevel::Info => "[INFO]",
            LogLevel::Warn => "[WARN]",
            LogLevel::Error => "[ERROR]",
        }
    }
}

/// One line of the log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogLine {
    pub time: String,
    pub level: LogLevel,
    pub message: String,
}

/// The newest lines, oldest first, capped at [`MAX_LINES`].
#[derive(Debug, Default)]
pub struct LogBuffer {
    lines: VecDeque<LogLine>,
}

impl LogBuffer {
    pub fn from_lines(lines: impl IntoIterator<Item = LogLine>) -> Self {
        let mut buffer = Self::default();
        for line in lines {
            buffer.push(line);
        }
        buffer
    }

    /// Appends a line, evicting the oldest once the cap is reached.
    pub fn push(&mut self, line: LogLine) {
        if self.lines.len() == MAX_LINES {
            self.lines.pop_front();
        }
        self.lines.push_back(line);
    }

    pub fn iter(&self) -> impl Iterator<Item = &LogLine> {
        self.lines.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }
}

/// Messages the Logs tab emits.
#[derive(Debug, Clone)]
pub enum Message {
    /// The LIVE pill was pressed.
    ToggleFollow,
    /// Export was pressed — display-only until the root owns file IO.
    Export,
}

/// The Logs tab's state.
#[derive(Debug)]
pub struct State {
    buffer: LogBuffer,
    follow: bool,
}

impl Default for State {
    /// Empty, and following new lines.
    fn default() -> Self {
        Self {
            buffer: LogBuffer::default(),
            follow: true,
        }
    }
}

impl State {
    /// Replaces the buffered lines with the latest read of the log.
    pub fn set_lines(&mut self, lines: Vec<LogLine>) {
        self.buffer = LogBuffer::from_lines(lines);
    }

    /// Whether new lines are followed. The page re-reads the log only while they are.
    pub fn following(&self) -> bool {
        self.follow
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::ToggleFollow => self.follow = !self.follow,
            // Display-only until the root owns file IO.
            Message::Export => {}
        }
    }

    /// The log panel, filling the height it is given and scrolling itself.
    pub fn view(&self) -> Element<'_, Message> {
        let header = row![
            utility::icon(Icon::lucide().terminal(), 14.0, Some((color::Primary::DEFAULT, color::Primary::DEFAULT))),
            Text::new("latest.log")
                .size(13)
                .font(fonts::JetBrainsMono::SEMI_BOLD)
                .color(color::Text::HEADLINE),
            Space::new().width(Fill),
            live_pill(self.follow),
            TextButton::labeled_icon(Icon::lucide().download(), "Export", Variant::Ghost).on_press(Message::Export),
        ]
        .spacing(10)
        .align_y(alignment::Vertical::Center);

        let body: Element<'_, Message> = if self.buffer.is_empty() {
            Text::new("No latest.log yet. Launch the game to create one.")
                .size(12)
                .font(fonts::Inter::REGULAR)
                .color(color::Text::MUTED)
                .into()
        } else {
            let lines: Vec<Element<'_, Message>> = self.buffer.iter().map(log_row).chain(std::iter::once(caret())).collect();
            column(lines).spacing(5).width(Fill).into()
        };
        let scroll = scrollbar::scroll(body).width(Fill).height(Fill);
        let scroll = if self.follow { scroll.anchor_bottom() } else { scroll };

        container(panel_with_padding(
            column![header, hairline(), scroll].spacing(14).height(Fill),
            Padding::new(18.0),
        ))
        .height(Fill)
        .into()
    }
}

/// The LIVE/PAUSED pill; pressing it toggles following.
fn live_pill<'a>(following: bool) -> Element<'a, Message> {
    let (label, ink, fill) = if following {
        ("LIVE", color::Primary::DEFAULT, color::Primary::TINT)
    } else {
        ("PAUSED", color::Text::MUTED, rgb!(255, 255, 255, 6%))
    };

    TextButton::content(Badge::new(label, ink, fill).with_pulse(following))
        .variant(Variant::Tab)
        .padding(Padding::new(0.0))
        .on_press(Message::ToggleFollow)
        .into()
}

/// One log line: timestamp, level tag, then the message.
fn log_row(line: &LogLine) -> Element<'_, Message> {
    row![
        container(
            Text::new(line.time.as_str())
                .size(12)
                .font(fonts::JetBrainsMono::REGULAR)
                .color(color::Text::PLACEHOLDER),
        )
        .width(TIME_WIDTH),
        container(
            Text::new(line.level.tag())
                .size(12)
                .font(fonts::JetBrainsMono::SEMI_BOLD)
                .color(level_ink(line.level)),
        )
        .width(TAG_WIDTH),
        Text::new(line.message.as_str())
            .size(12)
            .font(fonts::JetBrainsMono::REGULAR)
            .color(color::Text::DEFAULT),
    ]
    .spacing(14)
    .into()
}

/// The resting block of the blinking cursor, after the last line.
fn caret() -> Element<'static, Message> {
    row![
        Space::new().width(CARET_OFFSET),
        container(Space::new().width(7).height(14)).style(|_| container::Style {
            background: Some(Background::Color(color::Primary::DEFAULT)),
            ..container::Style::default()
        }),
    ]
    .into()
}

fn level_ink(level: LogLevel) -> Color {
    match level {
        LogLevel::Debug => color::Text::PLACEHOLDER,
        LogLevel::Info => color::Primary::DEFAULT,
        LogLevel::Warn => color::Accents::AMBER,
        LogLevel::Error => color::Accents::PINK,
    }
}

#[cfg(test)]
#[path = "../../../../tests/app/pages/instance/logs.rs"]
mod tests;
