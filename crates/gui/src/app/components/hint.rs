//! Hints — the bubble that names an icon-only control while the pointer rests on it. Wrap a control whose
//! icon does not say what it does, so its purpose shows on hover.

use iced::widget::tooltip::Position;
use iced::widget::{Text, container, tooltip};
use iced::{Background, Border, Element, Padding};

use crate::app::theme::{color, fonts};

/// Wraps `control` in a hint bubble that shows `text` above it while the pointer rests on it.
pub fn hint<'a, Message: 'a>(control: impl Into<Element<'a, Message>>, text: &'static str) -> Element<'a, Message> {
    tooltip(
        control,
        container(Text::new(text).size(12).font(fonts::Inter::MEDIUM).color(color::Text::HEADLINE))
            .padding(Padding::new(6.0).left(10.0).right(10.0))
            .style(|_| container::Style {
                background: Some(Background::Color(color::Surface::BG3)),
                border: Border {
                    radius: 8.0.into(),
                    width: 1.0,
                    color: color::Surface::LINE_STRONG,
                },
                ..container::Style::default()
            }),
        Position::Top,
    )
    .gap(6.0)
    .into()
}
