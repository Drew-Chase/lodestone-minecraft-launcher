//! Free-standing pieces the login views share: the logo, mono text, icons,
//! rules, and the Microsoft tile mark.

use std::sync::OnceLock;

use iced::widget::svg::Handle;
use iced::widget::{Space, Text, column, container, row, svg};
use iced::{Background, Color, Element, Fill, color};
use iced_color_extension::rgb;

use crate::app::theme::{color as palette, fonts};

use super::Message;

/// The Lodestone mark from the app's icon asset.
const LOGO_SVG: &str = include_str!("../../../../../../res/icon/icon.svg");

/// The logo handle — built once from the embedded icon asset. Shared with
/// the sidebar's logo.
pub(crate) fn logo_handle() -> &'static Handle {
    static LOGO: OnceLock<Handle> = OnceLock::new();
    LOGO.get_or_init(|| Handle::from_memory(LOGO_SVG.as_bytes()))
}

/// Mono text in the status bar's style — `--ink-3`, no line-height games.
pub(super) fn mono(content: impl Into<String>, size: f32) -> Text<'static> {
    Text::new(content.into())
        .font(fonts::JetBrainsMono::REGULAR)
        .size(size)
        .color(palette::Text::PLACEHOLDER)
}

/// An SVG icon from a lucide markup string. The tint supplies the color
/// (its alpha is ignored by the svg texture pipeline) and `opacity`
/// controls visibility — 0.0 hides the icon while keeping its layout
/// space, so hover states never shift the layout.
pub(super) fn icon(markup: &'static str, size: f32, tint: Color, opacity: f32) -> Element<'static, Message> {
    svg(Handle::from_memory(markup.as_bytes()))
        .width(size)
        .height(size)
        .opacity(opacity)
        .style(move |_, _| svg::Style { color: Some(tint) })
        .into()
}

/// The card's hairline border color — `--line` (white 7%) bumped to 9% per
/// the artboard.
pub(super) fn rgb_line() -> Color {
    rgb!(255, 255, 255, 9%)
}

/// A 1px horizontal rule in `--line`.
pub(super) fn rule() -> Element<'static, Message> {
    container(Space::new().height(1).width(Fill))
        .width(Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(palette::Surface::LINE)),
            ..container::Style::default()
        })
        .into()
}

/// A 1px underline for the verification link, exactly the width of its
/// fixed mono text ("microsoft.com/link" = 18 chars). Always occupies
/// space; visible only on hover.
pub(super) fn rule_under_link(visible: bool) -> Element<'static, Message> {
    container(Space::new().height(1).width(119))
        .width(119)
        .style(move |_| container::Style {
            background: Some(Background::Color(if visible { palette::Text::HEADLINE } else { Color::TRANSPARENT })),
            ..container::Style::default()
        })
        .into()
}

/// The four Microsoft logo tiles, 2×2 with 2px gutters.
pub(super) fn ms_tiles() -> Element<'static, Message> {
    let tile = |c: Color| {
        container(Space::new().width(7).height(7)).style(move |_| container::Style {
            background: Some(Background::Color(c)),
            ..container::Style::default()
        })
    };
    column![
        row![tile(color!(0xF25022)), tile(color!(0x7FBA00))].spacing(2),
        row![tile(color!(0x00A4EF)), tile(color!(0xFFB900))].spacing(2),
    ]
    .spacing(2)
    .into()
}
