//! Screenshot tiles — generated scene art with a caption.
//!
//! [`Caption::Overlay`] writes the file name over the art (the Overview strip)
//! with no frame. [`Caption::Footer`] adds an opaque bar with the name and size
//! inside a hairline frame (the Screenshots tab). The art is generated at the
//! tile's exact pixel size, so its rounded corners stay circular.

use aurora_leaf::Icon;
use iced::border::Radius;
use iced::widget::{Space, Text, container, responsive, row, stack, svg};
use iced::{Background, Border, ContentFit, Element, Fill, Padding, alignment};
use iced_color_extension::rgb;

use crate::app::components::scene_art::screenshot_handle;
use crate::app::pages::library::instance::Biome;
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

/// A screenshot tile `height` pixels tall, filling the width it is given.
pub fn screenshot_tile<'a, M: 'a>(biome: Biome, seed: u64, caption: Caption<'a>, height: f32) -> Element<'a, M> {
    let art = responsive(move |bounds| {
        let art = svg(screenshot_handle(biome, seed, bounds))
            .content_fit(ContentFit::Fill)
            .width(Fill)
            .height(Fill);
        stack![art, caption_layer(caption)].width(Fill).height(Fill).into()
    });

    let framed = matches!(caption, Caption::Footer { .. });
    container(art)
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
