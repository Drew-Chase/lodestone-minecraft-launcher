//! Hints — the bubble that names an icon-only control while the pointer rests on it. Wrap a control whose
//! icon does not say what it does, so its purpose shows on hover.

use iced::widget::tooltip::Position;
use iced::widget::{Text, container, tooltip};
use iced::{Background, Border, Element, Padding};

use crate::app::theme::{color, fonts};

/// Wraps `control` in a hint bubble that shows `text` above it while the pointer rests on it.
pub fn hint<'a, Message: 'a>(control: impl Into<Element<'a, Message>>, text: &'static str) -> Element<'a, Message> {
    at(control, text, Position::Top)
}

/// Wraps `control` in a hint bubble that shows `text` on the side `position` of it while the pointer rests on it.
pub fn at<'a, Message: 'a>(control: impl Into<Element<'a, Message>>, text: &'static str, position: Position) -> Element<'a, Message> {
    tooltip(control, bubble(text), position).gap(6.0).into()
}

/// The bubble itself: `text` on the surface every hint shares. The sidebar's rail draws it as an overlay.
pub fn bubble<'a, Message: 'a>(text: &'static str) -> Element<'a, Message> {
    container(Text::new(text).size(12).font(fonts::Inter::MEDIUM).color(color::text_headline()))
        .padding(Padding::new(6.0).left(10.0).right(10.0))
        .style(|_| container::Style {
            background: Some(Background::Color(color::surface_bg3())),
            border: Border {
                radius: 8.0.into(),
                width: 1.0,
                color: color::surface_line_strong(),
            },
            ..container::Style::default()
        })
        .into()
}
