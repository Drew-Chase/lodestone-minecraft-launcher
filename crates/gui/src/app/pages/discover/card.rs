//! The result cards: a grid card, a compact row and a table row. One project
//! is the same in each, laid out differently. Each has a fixed height, so a long
//! summary scrolls inside its body rather than stretching the card.

use std::hash::{DefaultHasher, Hash, Hasher};

use aurora_leaf::Icon;
use iced::widget::{Space, Text, column, container, image, row, stack, svg};
use iced::{Background, Border, Color, ContentFit, Element, Fill, Length, Padding, Size, alignment};
use lodestone_platforms::SearchResult;

use super::{Artwork, Message, Source};
use crate::app::components::platform_logo;
use crate::app::components::scene_art;
use crate::app::components::scrollbar;
use crate::app::lang;
use crate::app::pages::library::instance::Biome;
use crate::app::theme::{color, fonts};

/// The height of a grid card's art.
pub const GRID_ART_HEIGHT: f32 = 150.0;
/// A grid card's height. Every card is this tall, and a summary longer than its body scrolls.
const GRID_CARD_HEIGHT: f32 = 370.0;
/// A compact row's height. Every row is this tall, and a summary longer than its body scrolls.
const COMPACT_ROW_HEIGHT: f32 = 172.0;
/// A table row's height. Every row is this tall, and a summary longer than its cell scrolls.
const TABLE_ROW_HEIGHT: f32 = 88.0;
/// The height of a line of text as a multiple of its size: iced's default line height.
const LINE_SPACING: f32 = 1.3;
/// The side of a compact row's and a table row's thumbnail.
pub const THUMB_SIZE: f32 = 44.0;
/// The width of the table's source column, and of its numbers.
const TABLE_SOURCE_WIDTH: f32 = 130.0;
const TABLE_NUMBER_WIDTH: f32 = 110.0;
const TABLE_AUTHOR_WIDTH: f32 = 150.0;
const TABLE_ACTION_WIDTH: f32 = 96.0;

/// The biomes the scene art can show, in the order the hash picks from them.
const BIOMES: [Biome; 8] = [
    Biome::End,
    Biome::Forest,
    Biome::Cherry,
    Biome::Ocean,
    Biome::Desert,
    Biome::Snow,
    Biome::Nether,
    Biome::Mushroom,
];

/// The scene a project's art shows. It comes from the platform and the id, so a
/// project keeps its picture from one launch to the next.
pub fn scene_for(source: Source, id: &str) -> (Biome, u64) {
    let mut hasher = DefaultHasher::new();
    source.key().hash(&mut hasher);
    id.hash(&mut hasher);
    let hash = hasher.finish();
    (BIOMES[(hash % BIOMES.len() as u64) as usize], hash >> 8)
}

/// A count with a short suffix, the way the cards show it: `512`, `9.4K`, `18.2M`.
pub fn short_count(count: u64) -> String {
    match count {
        0..1_000 => count.to_string(),
        1_000..1_000_000 => format!("{:.1}K", count as f64 / 1_000.0),
        _ => format!("{:.1}M", count as f64 / 1_000_000.0),
    }
}

/// The date part of an ISO 8601 timestamp, or nothing when it has none.
fn date_only(updated: Option<&str>) -> &str {
    updated.and_then(|stamp| stamp.get(..10)).unwrap_or("")
}

/// The platform's mark and name in a small rounded chip.
pub fn chip<'a>(source: Source) -> Element<'a, Message> {
    container(
        row![
            platform_logo::logo(source.logo(), 12.0),
            Text::new(source.name().to_uppercase())
                .size(10)
                .font(fonts::JetBrainsMono::SEMI_BOLD)
                .color(color::Text::HEADLINE),
        ]
        .spacing(6)
        .align_y(alignment::Vertical::Center),
    )
    .padding(Padding::new(0.0).left(8).right(8).top(4).bottom(4))
    .style(|_| container::Style {
        background: Some(Background::Color(Color::from_rgba(0.03, 0.04, 0.05, 0.82))),
        border: Border {
            radius: 999.0.into(),
            width: 1.0,
            color: color::Surface::LINE_STRONG,
        },
        ..container::Style::default()
    })
    .into()
}

/// A line icon in `tint`.
pub fn glyph<'a>(markup: &'static str, size: f32, tint: Color) -> Element<'a, Message> {
    svg(svg::Handle::from_memory(markup.as_bytes()))
        .width(size)
        .height(size)
        .style(move |_, _| svg::Style { color: Some(tint) })
        .into()
}

/// The downloads figure, with its icon.
fn downloads<'a>(hit: &SearchResult) -> Element<'a, Message> {
    row![
        glyph(Icon::lucide().download(), 12.0, color::Text::MUTED),
        Text::new(short_count(hit.downloads))
            .size(12)
            .font(fonts::JetBrainsMono::REGULAR)
            .color(color::Text::MUTED),
    ]
    .spacing(6)
    .align_y(alignment::Vertical::Center)
    .into()
}

/// The author's name, or the platform's own name when the author is not known.
fn author_of(hit: &SearchResult) -> String {
    hit.author.clone().unwrap_or_default()
}

/// The surface a picture shows while it loads: the platform's mark on the panel colour.
fn loading_surface<'a>(source: Source, width: f32, height: f32) -> Element<'a, Message> {
    container(platform_logo::logo(source.logo(), width.min(height) * 0.5))
        .width(width)
        .height(height)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .style(|_| container::Style {
            background: Some(Background::Color(color::Surface::BG3)),
            ..container::Style::default()
        })
        .into()
}

/// The picture a thumbnail shows: the platform's picture, or the scene when the platform gave none.
fn thumb<'a>(source: Source, hit: &SearchResult, artwork: Artwork<'_>, size: f32) -> Element<'a, Message> {
    match artwork {
        Artwork::Loaded(picture) => image(picture.icon.clone()).width(size).height(size).into(),
        Artwork::Loading => loading_surface(source, size, size),
        Artwork::Scene => {
            let (biome, seed) = scene_for(source, &hit.id);
            svg(scene_art::scene_handle(biome, seed)).width(size).height(size).into()
        }
    }
}

/// The text of a name or author in a box `lines` lines tall. Text past the box is cut off, so a card keeps its
/// height whatever its text is.
fn lines_box<'a>(text: Text<'a>, size: f32, lines: f32) -> Element<'a, Message> {
    container(text)
        .width(Fill)
        .height(Length::Fixed((size * LINE_SPACING).ceil() * lines))
        .clip(true)
        .into()
}

/// A summary that fills the body it sits in, and scrolls when it runs longer than that body.
fn summary<'a>(text: &'a str, tint: Color) -> Element<'a, Message> {
    scrollbar::scroll(Text::new(text).size(12).font(fonts::Inter::REGULAR).color(tint).width(Fill))
        .width(Fill)
        .height(Fill)
        .into()
}

/// A project's art across a card or the hero: the platform's banner, a placeholder while it loads,
/// or the generated scene when the platform gave no picture.
pub fn cover_art<'a>(source: Source, hit: &SearchResult, artwork: Artwork<'_>, width: f32, height: f32) -> Element<'a, Message> {
    match artwork {
        Artwork::Loaded(picture) => image(picture.banner.clone())
            .width(width)
            .height(height)
            .content_fit(ContentFit::Cover)
            .into(),
        Artwork::Loading => loading_surface(source, width, height),
        Artwork::Scene => {
            let (biome, seed) = scene_for(source, &hit.id);
            svg(scene_art::banner_handle(biome, seed, Size::new(width, height)))
                .width(width)
                .height(height)
                .into()
        }
    }
}

/// A card's surface: the dark panel with a hairline border.
pub fn panel(_: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(color::Surface::BG2)),
        border: Border {
            radius: 16.0.into(),
            width: 1.0,
            color: color::Surface::LINE,
        },
        ..container::Style::default()
    }
}

/// A project as a grid card: its scene art with the platform's chip, then the name, author, summary, and a footer
/// with the downloads and `action`. The card is `GRID_CARD_HEIGHT` tall, and the summary takes what is left.
pub fn grid_card<'a>(source: Source, hit: &'a SearchResult, width: f32, artwork: Artwork<'_>, action: Element<'a, Message>) -> Element<'a, Message> {
    let cover = stack![
        cover_art(source, hit, artwork, width, GRID_ART_HEIGHT),
        container(chip(source))
            .width(Fill)
            .height(Fill)
            .padding(Padding::new(10.0))
            .align_x(alignment::Horizontal::Left)
            .align_y(alignment::Vertical::Top),
    ];
    // Two lines for the name, so a long one wraps rather than pushing the author down.
    let body = column![
        lines_box(
            Text::new(hit.name.as_str())
                .size(15)
                .font(fonts::Inter::BOLD)
                .color(color::Text::HEADLINE),
            15.0,
            2.0,
        ),
        lines_box(
            Text::new(author_of(hit)).size(12).font(fonts::Inter::REGULAR).color(color::Text::MUTED),
            12.0,
            1.0,
        ),
        summary(hit.description.as_str(), color::Text::DEFAULT),
        row![downloads(hit), Space::new().width(Fill), action].align_y(alignment::Vertical::Center),
    ]
    .spacing(8)
    .padding(Padding::new(14.0))
    .height(Fill);
    container(column![cover, body].width(width).height(Fill))
        .width(width)
        .height(Length::Fixed(GRID_CARD_HEIGHT))
        .style(panel)
        .into()
}

/// A project as a compact row: a thumbnail, the name with the platform, the author, the summary, and the downloads
/// and `action` below. The row is `COMPACT_ROW_HEIGHT` tall, and the summary takes what is left.
pub fn compact_row<'a>(source: Source, hit: &'a SearchResult, artwork: Artwork<'_>, action: Element<'a, Message>) -> Element<'a, Message> {
    let text = column![
        row![
            lines_box(
                Text::new(hit.name.as_str())
                    .size(14)
                    .font(fonts::Inter::BOLD)
                    .color(color::Text::HEADLINE),
                14.0,
                1.0,
            ),
            chip(source),
        ]
        .spacing(8)
        .align_y(alignment::Vertical::Center),
        lines_box(
            Text::new(author_of(hit)).size(12).font(fonts::Inter::REGULAR).color(color::Text::MUTED),
            12.0,
            1.0,
        ),
        summary(hit.description.as_str(), color::Text::DEFAULT),
        row![downloads(hit), Space::new().width(Fill), action].align_y(alignment::Vertical::Center),
    ]
    .spacing(4)
    .width(Fill)
    .height(Fill);
    container(
        row![thumb(source, hit, artwork, THUMB_SIZE), text]
            .spacing(14)
            .align_y(alignment::Vertical::Top)
            .height(Fill),
    )
    .padding(Padding::new(14.0))
    .width(Fill)
    .height(Length::Fixed(COMPACT_ROW_HEIGHT))
    .style(panel)
    .into()
}

/// The table's header row, labelled with the lang keys of its columns.
pub fn table_header<'a>() -> Element<'a, Message> {
    let label = |text: &'static str, width: Length| {
        Text::new(lang::t(text))
            .size(10)
            .font(fonts::JetBrainsMono::SEMI_BOLD)
            .color(color::Text::PLACEHOLDER)
            .width(width)
    };
    row![
        label("discover.table.name", Length::Fill),
        label("discover.table.author", Length::Fixed(TABLE_AUTHOR_WIDTH)),
        label("discover.table.source", Length::Fixed(TABLE_SOURCE_WIDTH)),
        label("discover.table.downloads", Length::Fixed(TABLE_NUMBER_WIDTH)),
        label("discover.table.updated", Length::Fixed(TABLE_NUMBER_WIDTH)),
        Space::new().width(Length::Fixed(TABLE_ACTION_WIDTH)),
    ]
    .spacing(12)
    .padding(Padding::new(0.0).left(16).right(16).top(10).bottom(10))
    .align_y(alignment::Vertical::Center)
    .into()
}

/// A project as a table row, with its thumbnail, name and summary, and its
/// author, platform, downloads, update date and `action` in columns.
pub fn table_row<'a>(source: Source, hit: &'a SearchResult, artwork: Artwork<'_>, action: Element<'a, Message>) -> Element<'a, Message> {
    let name = row![
        thumb(source, hit, artwork, THUMB_SIZE),
        column![
            lines_box(
                Text::new(hit.name.as_str())
                    .size(14)
                    .font(fonts::Inter::BOLD)
                    .color(color::Text::HEADLINE),
                14.0,
                1.0,
            ),
            summary(hit.description.as_str(), color::Text::MUTED),
        ]
        .spacing(2)
        .width(Fill)
        .height(Fill),
    ]
    .spacing(12)
    .align_y(alignment::Vertical::Center)
    .width(Fill)
    .height(Fill);
    let number = |text: String| {
        Text::new(text)
            .size(12)
            .font(fonts::JetBrainsMono::REGULAR)
            .color(color::Text::MUTED)
            .width(Length::Fixed(TABLE_NUMBER_WIDTH))
    };
    row![
        name,
        Text::new(author_of(hit))
            .size(12)
            .font(fonts::Inter::REGULAR)
            .color(color::Text::MUTED)
            .width(Length::Fixed(TABLE_AUTHOR_WIDTH)),
        container(chip(source)).width(Length::Fixed(TABLE_SOURCE_WIDTH)),
        number(short_count(hit.downloads)),
        number(date_only(hit.updated.as_deref()).to_string()),
        container(action)
            .width(Length::Fixed(TABLE_ACTION_WIDTH))
            .align_x(alignment::Horizontal::Right),
    ]
    .spacing(12)
    .padding(Padding::new(0.0).left(16).right(16).top(10).bottom(10))
    .align_y(alignment::Vertical::Center)
    .height(Length::Fixed(TABLE_ROW_HEIGHT))
    .into()
}

/// The label of an action button that opens a project's page.
pub fn view_label() -> &'static str {
    lang::t("discover.action.view")
}
