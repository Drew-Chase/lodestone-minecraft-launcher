//! The result cards: a grid card, a compact row and a table row. One project
//! is the same in each, laid out differently. Each has a fixed height, so a long
//! summary scrolls inside its body rather than stretching the card.

use std::hash::{DefaultHasher, Hash, Hasher};

use aurora_leaf::Icon;
use iced::border::Radius;
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
/// A grid card's height: the art, the name and author, the footer and about three lines of summary. A longer
/// summary scrolls in its body.
pub(super) const GRID_CARD_HEIGHT: f32 = 330.0;
/// A card's corner radius. Its border and its hover ring follow it, and its art sits a pixel inside it.
pub const CARD_RADIUS: f32 = 16.0;
/// A compact row's height. Every row is this tall, and a summary longer than its body scrolls.
pub(super) const COMPACT_ROW_HEIGHT: f32 = 172.0;
/// A table row's height. Every row is this tall, and a summary longer than its cell scrolls.
pub(super) const TABLE_ROW_HEIGHT: f32 = 88.0;
/// The height of a line of text as a multiple of its size: iced's default line height.
const LINE_SPACING: f32 = 1.3;
/// The side of a compact row's and a table row's thumbnail.
pub const THUMB_SIZE: f32 = 44.0;
/// The side of the icon beside a grid card's name, as the library card's icon is drawn.
pub const TITLE_ICON_SIZE: f32 = 36.0;
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
pub fn date_only(updated: Option<&str>) -> &str {
    updated.and_then(|stamp| stamp.get(..10)).unwrap_or("")
}

/// The platform's mark and name in a small rounded chip.
pub fn chip<'a>(source: Source) -> Element<'a, Message> {
    platform_logo::pill(
        row![
            platform_logo::logo(source.logo(), 12.0),
            Text::new(source.name().to_uppercase())
                .size(10)
                .font(fonts::JetBrainsMono::SEMI_BOLD)
                .color(color::text_headline()),
        ]
        .spacing(6)
        .align_y(alignment::Vertical::Center)
        .into(),
    )
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
        glyph(Icon::lucide().download(), 12.0, color::text_muted()),
        Text::new(short_count(hit.downloads))
            .size(12)
            .font(fonts::JetBrainsMono::REGULAR)
            .color(color::text_muted()),
    ]
    .spacing(6)
    .align_y(alignment::Vertical::Center)
    .into()
}

/// The author's name, or the platform's own name when the author is not known.
fn author_of(hit: &SearchResult) -> String {
    hit.author.clone().unwrap_or_default()
}

/// The surface a picture shows while it loads: the platform's mark on the panel colour, rounded as the picture is.
fn loading_surface<'a>(source: Source, width: f32, height: f32, corners: Radius) -> Element<'a, Message> {
    container(platform_logo::logo(source.logo(), width.min(height) * 0.5))
        .width(width)
        .height(height)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .style(move |_| container::Style {
            background: Some(Background::Color(color::surface_bg3())),
            border: Border {
                radius: corners,
                ..Border::default()
            },
            ..container::Style::default()
        })
        .into()
}

/// The picture a thumbnail shows: the platform's picture, or the scene when the platform gave none.
fn thumb<'a>(source: Source, hit: &SearchResult, artwork: Artwork<'_>, size: f32) -> Element<'a, Message> {
    match artwork {
        Artwork::Loaded(handle) => image(handle.clone()).width(size).height(size).into(),
        Artwork::Loading => loading_surface(source, size, size, Radius::default()),
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
    scrollbar::scroll_panel(Text::new(text).size(12).font(fonts::Inter::REGULAR).color(tint).width(Fill))
        .width(Fill)
        .height(Fill)
        .into()
}

/// Which corners a picture rounds. A grid card's art rounds its top two, since the card's information continues below
/// it. A hero's picture rounds all four.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Corners {
    /// A grid card's art: the top two corners.
    Card,
    /// A hero's picture: all four corners.
    Hero,
}

impl Corners {
    /// The corner radius of the picture's frame, all four corners alike.
    fn radius(self) -> f32 {
        match self {
            Corners::Card => scene_art::DISCOVER_CARD_ART_RADIUS,
            Corners::Hero => scene_art::DISCOVER_HERO_ART_RADIUS,
        }
    }

    /// How far a picture is drawn past the bottom of its box, which then cuts it off. iced rounds an image's corners
    /// in an order that does not fit a top-only radius, so a card's picture rounds all four corners and draws a radius
    /// lower, leaving its rounded bottom corners outside the box. A hero's picture fills its box as it is.
    fn bleed(self) -> f32 {
        match self {
            Corners::Card => self.radius(),
            Corners::Hero => 0.0,
        }
    }
}

/// A project's art across a card or the hero: the platform's banner, a placeholder while it loads, or the generated
/// scene when the platform gave no picture. The art fills `width`×`height` and rounds its `corners`.
pub fn cover_art<'a>(source: Source, hit: &SearchResult, artwork: Artwork<'_>, width: f32, height: f32, corners: Corners) -> Element<'a, Message> {
    let drawn = height + corners.bleed();
    match artwork {
        Artwork::Loaded(handle) => cut(
            image(handle.clone())
                .width(width)
                .height(drawn)
                .content_fit(ContentFit::Cover)
                .border_radius(corners.radius())
                .into(),
            width,
            height,
        ),
        Artwork::Loading => cut(loading_surface(source, width, drawn, Radius::from(corners.radius())), width, height),
        Artwork::Scene => {
            let (biome, seed) = scene_for(source, &hit.id);
            let size = Size::new(width, height);
            let handle = match corners {
                Corners::Card => scene_art::card_art_handle(biome, seed, size),
                Corners::Hero => scene_art::hero_art_handle(biome, seed, size),
            };
            svg(handle).width(width).height(height).into()
        }
    }
}

/// `content` cut to `width`×`height`, so the part of a picture drawn past its box does not show.
fn cut<'a>(content: Element<'a, Message>, width: f32, height: f32) -> Element<'a, Message> {
    container(content).width(width).height(height).clip(true).into()
}

/// A card's surface: the dark panel with a hairline border.
pub fn panel(_: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(color::surface_bg2())),
        border: Border {
            radius: CARD_RADIUS.into(),
            width: 1.0,
            color: color::surface_line(),
        },
        ..container::Style::default()
    }
}

/// A project as a grid card: its banner with the platform's chip, then the icon beside the name and author, the
/// summary, and a footer with the downloads and `action`. The card is `GRID_CARD_HEIGHT` tall, and the summary takes
/// what is left. `cover` is the banner, and `icon` the project's own picture.
pub fn grid_card<'a>(
    source: Source,
    hit: &'a SearchResult,
    width: f32,
    cover: Artwork<'_>,
    icon: Artwork<'_>,
    action: Element<'a, Message>,
) -> Element<'a, Message> {
    // The art sits a pixel inside the border, so the border shows above and beside it, and its top corners follow
    // the border's radius from inside.
    let art = container(stack![
        cover_art(source, hit, cover, width - 2.0, GRID_ART_HEIGHT - 1.0, Corners::Card),
        container(chip(source))
            .width(Fill)
            .height(Fill)
            .padding(Padding::new(10.0))
            .align_x(alignment::Horizontal::Left)
            .align_y(alignment::Vertical::Top),
    ])
    .padding(Padding::new(1.0).bottom(0.0))
    .width(width);
    // Two lines for the name, so a long one wraps rather than pushing the author down.
    let heading = row![
        thumb(source, hit, icon, TITLE_ICON_SIZE),
        column![
            lines_box(
                Text::new(hit.name.as_str())
                    .size(15)
                    .font(fonts::Inter::BOLD)
                    .color(color::text_headline()),
                15.0,
                2.0,
            ),
            lines_box(
                Text::new(author_of(hit)).size(12).font(fonts::Inter::REGULAR).color(color::text_muted()),
                12.0,
                1.0,
            ),
        ]
        .spacing(2)
        .width(Fill),
    ]
    .spacing(12)
    .align_y(alignment::Vertical::Center);
    let body = column![
        heading,
        summary(hit.description.as_str(), color::text_default()),
        row![downloads(hit), Space::new().width(Fill), action].align_y(alignment::Vertical::Center),
    ]
    .spacing(8)
    .padding(Padding::new(14.0))
    .height(Fill);
    container(column![art, body].width(width).height(Fill))
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
                    .color(color::text_headline()),
                14.0,
                1.0,
            ),
            chip(source),
        ]
        .spacing(8)
        .align_y(alignment::Vertical::Center),
        lines_box(
            Text::new(author_of(hit)).size(12).font(fonts::Inter::REGULAR).color(color::text_muted()),
            12.0,
            1.0,
        ),
        summary(hit.description.as_str(), color::text_default()),
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
            .color(color::text_placeholder())
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
        // The text block takes its own height, so the row centres it as it centres the thumbnail.
        column![
            lines_box(
                Text::new(hit.name.as_str())
                    .size(14)
                    .font(fonts::Inter::BOLD)
                    .color(color::text_headline()),
                14.0,
                1.0,
            ),
            summary(hit.description.as_str(), color::text_muted()),
        ]
        .spacing(2)
        .width(Fill),
    ]
    .spacing(12)
    .align_y(alignment::Vertical::Center)
    .width(Fill)
    .height(Fill);
    let number = |text: String| {
        Text::new(text)
            .size(12)
            .font(fonts::JetBrainsMono::REGULAR)
            .color(color::text_muted())
            .width(Length::Fixed(TABLE_NUMBER_WIDTH))
    };
    row![
        name,
        Text::new(author_of(hit))
            .size(12)
            .font(fonts::Inter::REGULAR)
            .color(color::text_muted())
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

/// A dimmed block standing in for a line of text, or a picture, that has not arrived yet.
fn placeholder<'a>(width: Length, height: f32) -> Element<'a, Message> {
    container(Space::new().width(width).height(height))
        .style(|_| container::Style {
            background: Some(Background::Color(color::surface_bg3())),
            border: Border {
                radius: 6.0.into(),
                ..Border::default()
            },
            ..container::Style::default()
        })
        .into()
}

/// A card the grid shows while its results load. It has the shape of a result card, so the grid keeps its places as the
/// results arrive.
pub fn skeleton_card<'a>(width: f32) -> Element<'a, Message> {
    let body = column![
        placeholder(Length::Fixed(width * 0.6), 14.0),
        placeholder(Length::Fixed(width * 0.35), 10.0),
        placeholder(Length::Fill, 10.0),
        placeholder(Length::Fixed(width * 0.5), 10.0),
    ]
    .spacing(10);
    container(column![
        placeholder(Length::Fill, GRID_ART_HEIGHT),
        container(body).padding(Padding::new(14.0))
    ])
    .width(width)
    .height(Length::Fixed(GRID_CARD_HEIGHT))
    .style(panel)
    .into()
}

/// A row the compact layout shows while its results load: a thumbnail and three lines, at the height a row takes.
pub fn skeleton_row<'a>() -> Element<'a, Message> {
    container(
        row![
            placeholder(Length::Fixed(THUMB_SIZE), THUMB_SIZE),
            column![
                placeholder(Length::Fixed(220.0), 14.0),
                placeholder(Length::Fill, 10.0),
                placeholder(Length::Fixed(140.0), 10.0),
            ]
            .spacing(10)
            .width(Fill),
        ]
        .spacing(14)
        .align_y(alignment::Vertical::Center),
    )
    .width(Fill)
    .height(Length::Fixed(COMPACT_ROW_HEIGHT))
    .padding(Padding::new(14.0))
    .style(panel)
    .into()
}

/// A row the table shows while its results load, with a block where each column's text will be.
pub fn skeleton_table_row<'a>() -> Element<'a, Message> {
    row![
        placeholder(Length::Fixed(THUMB_SIZE), THUMB_SIZE),
        column![placeholder(Length::Fixed(260.0), 14.0), placeholder(Length::Fixed(380.0), 10.0)]
            .spacing(8)
            .width(Fill),
        placeholder(Length::Fixed(TABLE_AUTHOR_WIDTH - 40.0), 10.0),
        placeholder(Length::Fixed(TABLE_SOURCE_WIDTH - 40.0), 10.0),
        placeholder(Length::Fixed(TABLE_NUMBER_WIDTH - 30.0), 10.0),
        placeholder(Length::Fixed(TABLE_NUMBER_WIDTH - 30.0), 10.0),
        Space::new().width(Length::Fixed(TABLE_ACTION_WIDTH)),
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
