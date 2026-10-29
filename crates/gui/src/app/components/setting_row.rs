//! Setting rows — label, description and control, the layout of the settings tab.
//!
//! [`setting`] and [`described_setting`] lay out one row, and
//! [`section_header`] heads a section with its icon, title and subtitle.
//! Stack rows with `panel::divided_panel` so hairlines separate them.

use crate::app::theme::{color, fonts, utility};
use iced::widget::{Space, Text, column, container, row};
use iced::{Background, Border, Color, Element, Fill, Padding, alignment};
use iced_color_extension::rgb;

/// A row label in the default ink.
pub fn label<'a>(text: impl Into<String>) -> Text<'a> {
    Text::new(text.into()).size(13).font(fonts::Inter::MEDIUM).color(color::Text::DEFAULT)
}

/// A row with only a label on the left and its control on the right.
pub fn setting<'a, M: 'a>(name: impl Into<String>, control: impl Into<Element<'a, M>>) -> Element<'a, M> {
    row![label(name), Space::new().width(Fill), control.into()]
        .align_y(alignment::Vertical::Center)
        .padding(Padding::new(0.0).top(14).bottom(14))
        .into()
}

/// A row with a label and a muted description on the left.
pub fn described_setting<'a, M: 'a>(name: impl Into<String>, description: impl Into<String>, control: impl Into<Element<'a, M>>) -> Element<'a, M> {
    row![
        column![
            label(name),
            Text::new(description.into())
                .size(11)
                .font(fonts::Inter::REGULAR)
                .color(color::Text::MUTED),
        ]
        .spacing(3),
        Space::new().width(Fill),
        control.into(),
    ]
    .spacing(16)
    .align_y(alignment::Vertical::Center)
    .padding(Padding::new(0.0).top(14).bottom(14))
    .into()
}

/// A section heading: a 22px icon tile drawn in `tint`, then the title and subtitle.
pub fn section_header<'a, M: 'a>(icon: &'static str, tint: Color, title: &'static str, subtitle: &'static str) -> Element<'a, M> {
    row![
        container(utility::icon(icon, 14.0, Some((tint, tint))))
            .width(22)
            .height(22)
            .align_x(alignment::Horizontal::Center)
            .align_y(alignment::Vertical::Center)
            .style(|_| container::Style {
                background: Some(Background::Color(rgb!(255, 255, 255, 4%))),
                border: Border {
                    radius: 6.0.into(),
                    width: 1.0,
                    color: color::Surface::LINE,
                },
                ..container::Style::default()
            }),
        column![
            Text::new(title).size(13).font(fonts::Inter::SEMI_BOLD).color(color::Text::HEADLINE),
            Text::new(subtitle).size(11).font(fonts::Inter::REGULAR).color(color::Text::MUTED),
        ]
        .spacing(2),
    ]
    .spacing(12)
    .align_y(alignment::Vertical::Center)
    .into()
}
