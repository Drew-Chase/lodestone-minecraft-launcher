//! Logs tab — the live `latest.log` panel over a bounded line buffer.
//!
//! The buffer keeps the newest [`MAX_LINES`] lines. While following, the
//! scroller stays pinned to the bottom so new lines come into view; the LIVE
//! pill toggles that, and the page re-reads the file on a timer while it is on.

use std::collections::{HashSet, VecDeque};

use aurora_leaf::Icon;
use iced::widget::{Space, Text, column, container, row};
use iced::{Background, Color, Element, Fill, Padding, alignment};
use iced_color_extension::rgb;

use crate::app::components::TextButton;
use crate::app::components::badge::Badge;
use crate::app::components::button::Variant;
use crate::app::components::panel::{hairline, panel_with_padding};
use crate::app::components::scrollbar;
use crate::app::lang;
use crate::app::theme::{color, fonts, utility};

/// How many lines the buffer keeps; the oldest are dropped first.
pub const MAX_LINES: usize = 500;

/// Column widths of the timestamp and level tag, so messages line up.
const TIME_WIDTH: f32 = 66.0;
const TAG_WIDTH: f32 = 64.0;
/// Offset that puts the caret under the message column.
const CARET_OFFSET: f32 = TIME_WIDTH + TAG_WIDTH + 28.0;

/// The severity a log line is tagged with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    /// Every level, from the quietest to the loudest, in the order the filter shows them.
    pub const ALL: [LogLevel; 4] = [LogLevel::Debug, LogLevel::Info, LogLevel::Warn, LogLevel::Error];

    /// The bracketed tag drawn before the message.
    pub fn tag(self) -> &'static str {
        match self {
            LogLevel::Debug => "[DEBUG]",
            LogLevel::Info => "[INFO]",
            LogLevel::Warn => "[WARN]",
            LogLevel::Error => "[ERROR]",
        }
    }

    /// The level's name, as the filter's chip shows it.
    pub fn name(self) -> &'static str {
        match self {
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        }
    }

    /// The level a log4j tag names: the `LEVEL` of a `[thread/LEVEL]` prefix, or the `level` of an XML event. Anything
    /// unrecognised is not a level.
    pub fn from_tag(tag: &str) -> Option<LogLevel> {
        match tag {
            "DEBUG" | "TRACE" => Some(LogLevel::Debug),
            "INFO" => Some(LogLevel::Info),
            "WARN" => Some(LogLevel::Warn),
            "ERROR" | "FATAL" => Some(LogLevel::Error),
            _ => None,
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

impl LogLine {
    /// Reads a `[HH:MM:SS] [thread/LEVEL]: message` line, the layout latest.log and the game's pattern output share. `None`
    /// for a line without that prefix, or whose tag names no level.
    pub fn from_prefixed(raw: &str) -> Option<LogLine> {
        let (time, rest) = raw.strip_prefix('[')?.split_once(']')?;
        if time.len() != 8 {
            return None;
        }
        let (tag, rest) = rest.trim_start().strip_prefix('[')?.split_once(']')?;
        let level = LogLevel::from_tag(tag.rsplit_once('/')?.1)?;
        let message = rest.strip_prefix(':').unwrap_or(rest).trim_start();
        Some(LogLine {
            time: time.to_string(),
            level,
            message: message.to_string(),
        })
    }
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
}

/// Messages the Logs tab emits.
#[derive(Debug, Clone)]
pub enum Message {
    /// The LIVE pill was pressed.
    ToggleFollow,
    /// A level's filter chip was pressed: its lines are shown, or hidden when they were.
    ToggleLevel(LogLevel),
    /// The button that opens `latest.log` in its app was pressed. The root opens it.
    OpenLatest,
    /// The button that shows `latest.log` in the file browser was pressed. The root shows it.
    RevealLatest,
}

/// The Logs tab's state.
#[derive(Debug)]
pub struct State {
    buffer: LogBuffer,
    follow: bool,
    /// The levels whose lines the filter hides. Every level shows until its chip is pressed.
    hidden: HashSet<LogLevel>,
}

impl Default for State {
    /// Empty, following new lines, with every level shown.
    fn default() -> Self {
        Self {
            buffer: LogBuffer::default(),
            follow: true,
            hidden: HashSet::new(),
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
            Message::ToggleLevel(level) => {
                if !self.hidden.remove(&level) {
                    self.hidden.insert(level);
                }
            }
            // The root opens and reveals the log.
            Message::OpenLatest | Message::RevealLatest => {}
        }
    }

    /// The log panel, filling the height it is given and scrolling itself. While a game of the
    /// instance has output, `console` is that output, which shows lines the moment they arrive,
    /// including start-up errors the log file never gets. Otherwise the file's lines show.
    pub fn view<'a>(&'a self, console: Option<&'a VecDeque<LogLine>>) -> Element<'a, Message> {
        let console = console.filter(|lines| !lines.is_empty());
        let title = if console.is_some() {
            lang::t("instance.logs.console")
        } else {
            "latest.log"
        };
        let header = row![
            utility::icon(Icon::lucide().terminal(), 14.0, Some((color::primary(), color::primary()))),
            Text::new(title)
                .size(13)
                .font(fonts::JetBrainsMono::SEMI_BOLD)
                .color(color::text_headline()),
            Space::new().width(Fill),
            row(LogLevel::ALL.map(|level| level_chip(level, !self.hidden.contains(&level))))
                .spacing(6)
                .align_y(alignment::Vertical::Center),
            live_pill(self.follow),
            TextButton::labeled_icon(Icon::lucide().file(), lang::t("instance.logs.open_latest"), Variant::Ghost).on_press(Message::OpenLatest),
            TextButton::labeled_icon(Icon::lucide().folder(), lang::t("instance.logs.reveal"), Variant::Ghost).on_press(Message::RevealLatest),
        ]
        .spacing(10)
        .align_y(alignment::Vertical::Center);

        let lines: Vec<&LogLine> = match console {
            Some(lines) => lines.iter().collect(),
            None => self.buffer.iter().collect(),
        };
        let rows: Vec<&LogLine> = lines.into_iter().filter(|line| !self.hidden.contains(&line.level)).collect();
        let body: Element<'_, Message> = if rows.is_empty() {
            Text::new(lang::t("instance.logs.empty"))
                .size(12)
                .font(fonts::Inter::REGULAR)
                .color(color::text_muted())
                .into()
        } else {
            let lines: Vec<Element<'_, Message>> = rows.into_iter().map(log_row).chain(std::iter::once(caret())).collect();
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

/// A level's filter chip: lit in the level's colour while its lines show, and dimmed while they are hidden.
fn level_chip<'a>(level: LogLevel, shown: bool) -> Element<'a, Message> {
    let (ink, fill) = if shown {
        (level_ink(level), level_fill(level))
    } else {
        (color::text_placeholder(), rgb!(255, 255, 255, 2%))
    };
    TextButton::content(Badge::new(level.name(), ink, fill))
        .variant(Variant::Tab)
        .padding(Padding::new(0.0))
        .on_press(Message::ToggleLevel(level))
        .into()
}

/// The LIVE/PAUSED pill; pressing it toggles following.
fn live_pill<'a>(following: bool) -> Element<'a, Message> {
    let (label, ink, fill) = if following {
        (lang::t("instance.logs.live"), color::primary(), color::primary_tint())
    } else {
        (lang::t("instance.logs.paused"), color::text_muted(), rgb!(255, 255, 255, 6%))
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
                .color(color::text_placeholder()),
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
            .color(color::text_default()),
    ]
    .spacing(14)
    .into()
}

/// The resting block of the blinking cursor, after the last line.
fn caret() -> Element<'static, Message> {
    row![
        Space::new().width(CARET_OFFSET),
        container(Space::new().width(7).height(14)).style(|_| container::Style {
            background: Some(Background::Color(color::primary())),
            ..container::Style::default()
        }),
    ]
    .into()
}

fn level_ink(level: LogLevel) -> Color {
    match level {
        LogLevel::Debug => color::cyan(),
        LogLevel::Info => color::primary(),
        LogLevel::Warn => color::amber(),
        LogLevel::Error => color::pink(),
    }
}

/// The fill of a lit level chip. DEBUG's is light blue, as its ink is, so a lit DEBUG chip reads apart from a dimmed one.
fn level_fill(level: LogLevel) -> Color {
    match level {
        LogLevel::Debug => color::cyan_tint(),
        LogLevel::Info | LogLevel::Warn | LogLevel::Error => rgb!(255, 255, 255, 6%),
    }
}

#[cfg(test)]
#[path = "../../../../tests/app/pages/instance/logs.rs"]
mod tests;
