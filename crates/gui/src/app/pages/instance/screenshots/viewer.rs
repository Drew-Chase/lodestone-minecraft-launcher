//! The viewer: one capture at full size, a previous and a next button either side, its controls above
//! and a row of small thumbnails beneath for quick navigation. It opens over the whole window, so the titlebar and sidebar sit
//! under it. The captures it steps through are the ones the grid lists.

use std::cmp::Ordering;

use aurora_leaf::Icon;
use iced::event::Status;
use iced::keyboard::{self, Key as KeyboardKey, key::Named};
use iced::widget::{Space, Text, column, container, image, opaque, row};
use iced::{Background, Color, ContentFit, Element, Event, Fill, Size, alignment, window};
use iced_color_extension::rgb;

use super::strip;
use super::{Action, Key, Message, State, menu};
use crate::app::components::IconButton;
use crate::app::components::hint::hint;
use crate::app::components::icon_button::SECONDARY_BUTTON_STYLE;
use crate::app::components::screenshot_tile::placeholder_art;
use crate::app::lang;
use crate::app::theme::{color, fonts};

/// The viewer's margin to the window's edges.
pub const PADDING: f32 = 24.0;
/// The corner radius of the picture.
const RADIUS: f32 = 14.0;
/// The dimming over the window: near opaque, so the grid behind it only shows as a hint.
const SCRIM: Color = rgb!(9, 10, 13, 94%);

/// Where the viewer goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Move {
    Previous,
    Next,
    First,
    Last,
}

/// The capture `movement` lands on, from `current` among `len` captures.
pub fn moved(current: usize, len: usize, movement: Move) -> usize {
    match movement {
        Move::Previous => current.saturating_sub(1),
        Move::Next => (current + 1).min(len.saturating_sub(1)),
        Move::First => 0,
        Move::Last => len.saturating_sub(1),
    }
}

/// Whether `movement` lands on a different capture than `current`.
pub fn can_move(current: usize, len: usize, movement: Move) -> bool {
    moved(current, len, movement) != current
}

/// The captures the viewer needs at full size: `current` first, then the ones either side of it.
pub fn needed(current: usize, len: usize) -> Vec<usize> {
    let mut indices = Vec::with_capacity(3);
    if current < len {
        indices.push(current);
    }
    if current > 0 && current - 1 < len {
        indices.push(current - 1);
    }
    if current + 1 < len {
        indices.push(current + 1);
    }
    indices
}

/// The viewer's place once the capture at `removed` has left. It stays on the same index, which the
/// capture after it now takes, or steps back when the last capture went. A closed viewer stays closed,
/// and so does one with nothing left to show.
pub fn after_removal(current: Option<usize>, removed: usize, remaining: usize) -> Option<usize> {
    let current = current?;
    if remaining == 0 {
        return None;
    }
    Some(match removed.cmp(&current) {
        Ordering::Less => current - 1,
        Ordering::Equal => current.min(remaining - 1),
        Ordering::Greater => current,
    })
}

/// The capture's place among the captures, counted from one: "3 / 48".
pub fn counter(index: usize, total: usize) -> String {
    let current = (index + 1).to_string();
    let total = total.to_string();
    lang::t_with(
        "instance.screenshots.viewer.counter",
        &[("current", current.as_str()), ("total", total.as_str())],
    )
}

/// The viewer's keys: Escape closes, the arrows, Home and End move, and Delete and Ctrl+C act on the
/// capture. A key a widget already took is left alone.
pub fn key_event(event: Event, status: Status, _window: window::Id) -> Option<Message> {
    if status != Status::Ignored {
        return None;
    }
    let Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) = event else {
        return None;
    };
    let message = match key.as_ref() {
        KeyboardKey::Named(Named::Escape) => Message::Close,
        KeyboardKey::Named(Named::ArrowLeft) => Message::Move(Move::Previous),
        KeyboardKey::Named(Named::ArrowRight) => Message::Move(Move::Next),
        KeyboardKey::Named(Named::Home) => Message::Move(Move::First),
        KeyboardKey::Named(Named::End) => Message::Move(Move::Last),
        KeyboardKey::Named(Named::Delete) => Message::Key(Key::Delete),
        KeyboardKey::Character(letter) if modifiers.command() && letter.eq_ignore_ascii_case("c") => Message::Key(Key::Copy),
        _ => return None,
    };
    Some(message)
}

/// The viewer over the window, showing the capture at `index`.
pub fn view(state: &State, index: usize, size: Size) -> Element<'_, Message> {
    let total = state.shots().len();
    let body = column![
        top_bar(state, index),
        row![
            step(index, total, Move::Previous, Icon::lucide().chevron_left()),
            stage(state, index),
            step(index, total, Move::Next, Icon::lucide().chevron_right()),
        ]
        .spacing(16)
        .height(Fill)
        .align_y(alignment::Vertical::Center),
        strip::view(state, index, size),
    ]
    .spacing(18)
    .width(Fill)
    .height(Fill);

    opaque(container(body).width(Fill).height(Fill).padding(PADDING).style(|_| container::Style {
        background: Some(Background::Color(SCRIM)),
        ..container::Style::default()
    }))
}

/// The file name and size on the left; the counter and the capture's buttons on the right.
fn top_bar(state: &State, index: usize) -> Element<'_, Message> {
    let shot = &state.shots()[index];
    let path = shot.path.clone();
    let caption = format!("{} · {}", shot.file_name, shot.size_label);
    row![
        Text::new(caption).size(12).font(fonts::JetBrainsMono::REGULAR).color(color::Text::MUTED),
        Space::new().width(Fill),
        Text::new(counter(index, state.shots().len()))
            .size(12)
            .font(fonts::JetBrainsMono::REGULAR)
            .color(color::Text::PLACEHOLDER),
        tool(
            Icon::lucide().copy(),
            lang::t("instance.screenshots.viewer.copy"),
            Message::Menu {
                path: path.clone(),
                action: Some(Action::Copy),
            },
        ),
        tool(
            Icon::lucide().external_link(),
            lang::t("instance.screenshots.viewer.open"),
            Message::Menu {
                path: path.clone(),
                action: Some(Action::Open),
            },
        ),
        tool(
            Icon::lucide().folder(),
            lang::t("instance.screenshots.viewer.reveal"),
            Message::Menu {
                path: path.clone(),
                action: Some(Action::Reveal),
            },
        ),
        tool(
            Icon::lucide().trash(),
            lang::t("instance.screenshots.viewer.delete"),
            Message::Menu {
                path,
                action: Some(Action::Delete),
            },
        ),
        tool(Icon::lucide().x(), lang::t("common.close"), Message::Close),
    ]
    .spacing(8)
    .align_y(alignment::Vertical::Center)
    .into()
}

/// An icon button for the viewer's bar, named by a hint while the pointer rests on it.
fn tool(glyph: &'static str, label: &'static str, message: Message) -> Element<'static, Message> {
    hint(
        IconButton::new(glyph)
            .size(32.0)
            .icon_size(14.0)
            .radius(9.0)
            .style(SECONDARY_BUTTON_STYLE)
            .on_press(message),
        label,
    )
}

/// A previous or next button, dimmed where there is no other capture to move to.
fn step(index: usize, total: usize, movement: Move, glyph: &'static str) -> Element<'static, Message> {
    IconButton::new(glyph)
        .size(44.0)
        .icon_size(18.0)
        .radius(14.0)
        .style(SECONDARY_BUTTON_STYLE)
        .disabled(!can_move(index, total, movement))
        .on_press(Message::Move(movement))
        .into()
}

/// The capture at full size, or its thumbnail until the full image lands. A right-click on it offers
/// the capture's menu.
fn stage(state: &State, index: usize) -> Element<'_, Message> {
    let shot = &state.shots()[index];
    let picture: Element<'_, Message> = match state.picture(&shot.path) {
        Some(handle) => image(handle.clone()).content_fit(ContentFit::Contain).width(Fill).height(Fill).into(),
        None => placeholder_art(RADIUS),
    };
    menu::attach(container(picture).width(Fill).height(Fill), &shot.path)
}

#[cfg(test)]
#[path = "../../../../../tests/app/pages/instance/screenshots/viewer.rs"]
mod tests;
