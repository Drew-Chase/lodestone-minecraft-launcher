//! The hero: the first hit of a browse, large, with its chip, its downloads and a
//! button to its page. It is shown while the page browses, and hidden while a
//! search is typed.

use aurora_leaf::Icon;
use iced::widget::{Space, Text, column, container, responsive, row, stack};
use iced::{Background, Color, Element, Fill, Length, Padding, alignment};
use lodestone_platforms::SearchResult;

use super::card::{Corners, chip, cover_art, glyph, short_count};
use super::{Discover, Message};
use crate::app::components::TextButton;
use crate::app::components::button::Variant;
use crate::app::components::scene_art;
use crate::app::lang;
use crate::app::theme::{color, fonts};

/// The hero's height.
pub(super) const HEIGHT: f32 = 240.0;

/// The hero for the page's first hit. `None` while there is no hit, or while the page is searched rather than browsed.
/// The hero takes the width its column gives it, so it runs the whole width of the content, as the filter row does.
pub(super) fn view(page: &Discover) -> Option<Element<'_, Message>> {
    if !page.text.trim().is_empty() {
        return None;
    }
    let hit = page.results.hits.first()?;
    // The height is the hero's own, so the responsive wrapper takes no more than its content asks for.
    Some(responsive(move |size| banner(page, hit, size.width)).height(Length::Shrink).into())
}

/// The hero for `hit`, at `width`.
fn banner<'a>(page: &'a Discover, hit: &'a SearchResult, width: f32) -> Element<'a, Message> {
    let width = width.max(320.0);
    // The picture sits a pixel inside the frame, and rounds its corners concentric with the frame's.
    let art = cover_art(
        page.source,
        hit,
        page.artwork(hit.banner_url.as_deref()),
        width - 2.0,
        HEIGHT - 2.0,
        Corners::Hero,
    );
    let shade = container(Space::new().width(Fill).height(Fill)).style(|_| container::Style {
        background: Some(Background::Color(Color::from_rgba(0.03, 0.04, 0.05, 0.55))),
        border: iced::Border {
            radius: scene_art::DISCOVER_HERO_ART_RADIUS.into(),
            ..iced::Border::default()
        },
        ..container::Style::default()
    });

    let open = TextButton::content(
        row![
            glyph(Icon::lucide().book_open(), 14.0, color::primary_on()),
            Text::new(super::card::view_label())
                .size(13)
                .font(fonts::Inter::BOLD)
                .color(color::primary_on()),
        ]
        .spacing(8)
        .align_y(alignment::Vertical::Center),
    )
    .variant(Variant::Primary)
    .padding(Padding::new(0.0).left(18).right(18).top(10).bottom(10))
    .on_press(Message::OpenResource(hit.id.clone()));

    let text = column![
        row![
            chip(page.source),
            Text::new(lang::t("discover.hero.most_downloaded"))
                .size(11)
                .font(fonts::JetBrainsMono::SEMI_BOLD)
                .color(color::primary()),
        ]
        .spacing(12)
        .align_y(alignment::Vertical::Center),
        Text::new(hit.name.as_str())
            .size(30)
            .font(fonts::Inter::EXTRA_BOLD)
            .color(color::text_headline()),
        Text::new(hit.description.as_str())
            .size(14)
            .font(fonts::Inter::REGULAR)
            .color(color::text_default())
            .width(Length::Fixed(560.0)),
        row![
            open,
            Text::new(lang::t_with("discover.hero.downloads", &[("count", short_count(hit.downloads).as_str())]))
                .size(12)
                .font(fonts::JetBrainsMono::REGULAR)
                .color(color::text_muted()),
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

    container(stack![art, shade, body])
        .padding(Padding::new(1.0))
        .width(width)
        .height(HEIGHT)
        .style(|_| container::Style {
            border: iced::Border {
                radius: 18.0.into(),
                width: 1.0,
                color: color::surface_line(),
            },
            ..container::Style::default()
        })
        .into()
}
