//! Screenshot tiles — a capture's thumbnail with a caption.
//!
//! [`Caption::Overlay`] writes the file name over the bottom-left corner with no
//! frame (the Overview strip). [`Caption::Footer`] adds an opaque bar with the
//! name and size inside a hairline frame (the Screenshots tab). A tile whose
//! thumbnail has not decoded yet shows [`placeholder_art`].

use aurora_leaf::Icon;
use iced::border::Radius;
use iced::widget::image::Handle;
use iced::widget::{Space, Text, container, image, row, stack};
use iced::{Background, Border, ContentFit, Element, Fill, Padding, alignment};
use iced_color_extension::rgb;

use crate::app::theme::{color, fonts, utility};

/// The footer bar's height, per the reference (34px).
pub const FOOTER_HEIGHT: f32 = 34.0;
/// The tile's corner radius.
const RADIUS: f32 = 14.0;

/// What the tile prints over or beneath its art.
#[derive(Debug, Clone, Copy)]
pub enum Caption<'a> {
    /// The name in small mono over the bottom-left corner.
    Overlay(&'a str),
    /// A bar beneath the art with the name and the size, inside a hairline frame.
    Footer { name: &'a str, size: &'a str },
}

/// A screenshot tile `height` pixels tall, filling the width it is given. The
/// thumbnail covers the art area; `None` shows the placeholder instead.
pub fn photo_tile<'a, M: 'a>(thumbnail: Option<&Handle>, caption: Caption<'a>, height: f32) -> Element<'a, M> {
    let art: Element<'a, M> = match thumbnail {
        Some(handle) => image(handle.clone())
            .content_fit(ContentFit::Cover)
            .border_radius(RADIUS)
            .width(Fill)
            .height(Fill)
            .into(),
        None => placeholder_art(RADIUS),
    };

    let framed = matches!(caption, Caption::Footer { .. });
    container(stack![art, caption_layer(caption)].width(Fill).height(Fill))
        .width(Fill)
        .height(height)
        .style(move |_| container::Style {
            border: Border {
                radius: RADIUS.into(),
                width: if framed { 1.0 } else { 0.0 },
                color: color::Surface::LINE,
            },
            ..container::Style::default()
        })
        .into()
}

/// The neutral stand-in for a thumbnail that is not ready, or cannot be read,
/// with corners rounded to `radius` like the thumbnail it stands in for.
pub fn placeholder_art<'a, M: 'a>(radius: f32) -> Element<'a, M> {
    container(utility::icon(
        Icon::lucide().image(),
        18.0,
        Some((color::Text::PLACEHOLDER, color::Text::PLACEHOLDER)),
    ))
    .width(Fill)
    .height(Fill)
    .align_x(alignment::Horizontal::Center)
    .align_y(alignment::Vertical::Center)
    .style(move |_| container::Style {
        background: Some(Background::Color(rgb!(255, 255, 255, 4%))),
        border: Border {
            radius: radius.into(),
            ..Border::default()
        },
        ..container::Style::default()
    })
    .into()
}

/// The caption, pinned to the tile's bottom edge.
fn caption_layer<'a, M: 'a>(caption: Caption<'a>) -> Element<'a, M> {
    match caption {
        Caption::Overlay(name) => container(
            Text::new(name)
                .size(9)
                .font(fonts::JetBrainsMono::REGULAR)
                .color(rgb!(255, 255, 255, 70%)),
        )
        .width(Fill)
        .height(Fill)
        .align_x(alignment::Horizontal::Left)
        .align_y(alignment::Vertical::Bottom)
        .padding(Padding::new(0.0).left(8).bottom(6))
        .into(),
        Caption::Footer { name, size } => container(footer_bar(name, size))
            .width(Fill)
            .height(Fill)
            .align_y(alignment::Vertical::Bottom)
            .into(),
    }
}

/// The opaque footer: an image glyph, the file name, and the size on the right.
/// Its bottom corners follow the tile's radius.
fn footer_bar<'a, M: 'a>(name: &'a str, size: &'a str) -> Element<'a, M> {
    container(
        row![
            utility::icon(Icon::lucide().image(), 11.0, Some((color::Text::PLACEHOLDER, color::Text::PLACEHOLDER))),
            Text::new(name).size(11).font(fonts::JetBrainsMono::REGULAR).color(color::Text::MUTED),
            Space::new().width(Fill),
            Text::new(size)
                .size(11)
                .font(fonts::JetBrainsMono::REGULAR)
                .color(color::Text::PLACEHOLDER),
        ]
        .spacing(8)
        .align_y(alignment::Vertical::Center),
    )
    .width(Fill)
    .height(FOOTER_HEIGHT)
    .padding(Padding::new(0.0).left(12).right(12))
    .align_y(alignment::Vertical::Center)
    .style(|_| container::Style {
        background: Some(Background::Color(iced::color!(0x0D0F12))),
        border: Border {
            radius: Radius {
                top_left: 0.0,
                top_right: 0.0,
                bottom_right: RADIUS,
                bottom_left: RADIUS,
            },
            ..Border::default()
        },
        ..container::Style::default()
    })
    .into()
}
