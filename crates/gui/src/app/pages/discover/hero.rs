//! The hero: the first hit of a browse, large, with its chip, its downloads and a
//! button to its page. It is shown while the page browses, and hidden while a
//! search is typed.

use aurora_leaf::Icon;
use iced::widget::{Space, Text, column, container, row, stack};
use iced::{Background, Color, Element, Fill, Length, Padding, Size, alignment};

use super::card::{chip, cover_art, glyph, short_count};
use super::{Discover, Message};
use crate::app::components::TextButton;
use crate::app::components::button::Variant;
use crate::app::lang;
use crate::app::theme::{color, fonts};

/// The hero's height.
const HEIGHT: f32 = 240.0;

/// The hero for the page's first hit, sized to the content column. `None` while
/// there is no hit, or while the page is searched rather than browsed.
pub(super) fn view(page: &Discover, window: Size) -> Option<Element<'_, Message>> {
    if !page.text.trim().is_empty() {
        return None;
    }
    let hit = page.results.hits.first()?;
    let width = (window.width - 68.0 - 56.0).max(320.0);
    let art = cover_art(page.source, hit, page.artwork(hit.icon_url.as_deref()), width, HEIGHT);
    let shade = container(Space::new().width(Fill).height(Fill)).style(|_| container::Style {
        background: Some(Background::Color(Color::from_rgba(0.03, 0.04, 0.05, 0.55))),
        ..container::Style::default()
    });

    let mut open = TextButton::content(
        row![
            glyph(Icon::lucide().external_link(), 14.0, color::Primary::ON),
            Text::new(super::card::view_label())
                .size(13)
                .font(fonts::Inter::BOLD)
                .color(color::Primary::ON),
        ]
        .spacing(8)
        .align_y(alignment::Vertical::Center),
    )
    .variant(Variant::Primary)
    .padding(Padding::new(0.0).left(18).right(18).top(10).bottom(10));
    if let Some(url) = &hit.web_url {
        open = open.on_press(Message::Open(url.clone()));
    } else {
        open = open.disabled(true);
    }

    let text = column![
        row![
            chip(page.source),
            Text::new(lang::t("discover.hero.most_downloaded"))
                .size(11)
                .font(fonts::JetBrainsMono::SEMI_BOLD)
                .color(color::Primary::DEFAULT),
        ]
        .spacing(12)
        .align_y(alignment::Vertical::Center),
        Text::new(hit.name.as_str())
            .size(30)
            .font(fonts::Inter::EXTRA_BOLD)
            .color(color::Text::HEADLINE),
        Text::new(hit.description.as_str())
            .size(14)
            .font(fonts::Inter::REGULAR)
            .color(color::Text::DEFAULT)
            .width(Length::Fixed(560.0)),
        row![
            open,
            Text::new(lang::t_with("discover.hero.downloads", &[("count", short_count(hit.downloads).as_str())]))
                .size(12)
                .font(fonts::JetBrainsMono::REGULAR)
                .color(color::Text::MUTED),
        ]
        .spacing(16)
        .align_y(alignment::Vertical::Center),
    ]
    .spacing(10);
    let body = container(text)
        .padding(Padding::new(28.0))
        .width(Fill)
        .height(Fill)
        .align_y(alignment::Vertical::Bottom);

    Some(
        container(stack![art, shade, body])
            .width(width)
            .height(HEIGHT)
            .style(|_| container::Style {
                border: iced::Border {
                    radius: 18.0.into(),
                    width: 1.0,
                    color: color::Surface::LINE,
                },
                ..container::Style::default()
            })
            .into(),
    )
}
