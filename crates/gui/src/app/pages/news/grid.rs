//! The News page's cards: posts and patch notes in rows of equal cards, and the pieces every card shares.

use std::collections::HashMap;

use chrono::NaiveDate;
use iced::widget::{Column, Row, Space, Text, column, container, image, mouse_area};
use iced::{Background, ContentFit, Element, Fill, Length, Padding, mouse};
use piston_mc::news::Category;

use super::Message;
use crate::app::components::badge::Badge;
use crate::app::components::panel::panel_with_padding;
use crate::app::lang;
use crate::app::news::feed::{Feed, absolute};
use crate::app::theme::{color, fonts};

/// The height of a card's picture.
pub const COVER_HEIGHT: f32 = 160.0;

/// A picture the page has loaded, cropped to fill a box `height` tall and the width of its column. A picture still loading,
/// or one that did not load, shows as a plain panel of the same size.
pub fn cover<'a>(pictures: &'a HashMap<String, image::Handle>, url: &str, height: f32) -> Element<'a, Message> {
    match pictures.get(url) {
        Some(handle) => image(handle.clone())
            .width(Fill)
            .height(Length::Fixed(height))
            .content_fit(ContentFit::Cover)
            .into(),
        None => container(Space::new().width(Fill).height(Length::Fixed(height)))
            .style(|_| container::Style {
                background: Some(Background::Color(color::surface_bg3())),
                ..container::Style::default()
            })
            .into(),
    }
}

/// A post's date as the page shows it, such as "Jan 16, 2024". A date the feed wrote some other way is shown as it is.
pub fn date_label(date: &str) -> String {
    NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map(|day| day.format("%b %-d, %Y").to_string())
        .unwrap_or_else(|_| date.to_owned())
}

/// The name a patch note's kind shows as, or nothing for a kind the page does not name.
pub fn kind_label(kind: Option<&str>) -> &'static str {
    match kind {
        Some("release") => lang::t("news.patch.release"),
        Some("snapshot") => lang::t("news.patch.snapshot"),
        _ => "",
    }
}

/// The category's name and its colours: the badge a post shows.
pub fn category_badge<'a>(category: &Category) -> Element<'a, Message> {
    let (label, ink, fill) = match category {
        Category::MinecraftJavaEdition => (lang::t("news.filter.java"), color::primary(), color::primary_tint()),
        Category::MinecraftForWindows => (lang::t("news.filter.bedrock"), color::cyan(), color::cyan_tint()),
        Category::MinecraftDungeons => (lang::t("news.filter.dungeons"), color::amber(), color::amber_tint()),
        Category::MinecraftLegends => (lang::t("news.filter.legends"), color::violet_light(), color::violet_tint()),
    };
    Badge::new(label, ink, fill).compact().into()
}

/// The posts at `places` in the feed, in rows of `columns` cards.
pub fn posts<'a>(feed: &'a Feed, places: &[usize], pictures: &'a HashMap<String, image::Handle>, columns: usize) -> Element<'a, Message> {
    grid(places.iter().map(|&place| post_card(feed, place, pictures)).collect(), columns)
}

/// One post as a card: its picture, its date, title and summary. A press opens the post on minecraft.net.
fn post_card<'a>(feed: &'a Feed, place: usize, pictures: &'a HashMap<String, image::Handle>) -> Element<'a, Message> {
    let post = &feed.posts[place];
    let text = column![
        Text::new(date_label(&post.date))
            .size(11)
            .font(fonts::JetBrainsMono::REGULAR)
            .color(color::text_placeholder()),
        Text::new(post.title.clone())
            .size(15)
            .font(fonts::Inter::SEMI_BOLD)
            .color(color::text_headline()),
        Text::new(post.text.clone())
            .size(12)
            .font(fonts::Inter::REGULAR)
            .color(color::text_muted()),
    ]
    .spacing(6)
    .width(Fill);
    let card = column![
        cover(pictures, &absolute(&post.play_page_image.url), COVER_HEIGHT),
        container(text).padding(Padding::new(14.0)).width(Fill),
    ];
    clickable(panel_with_padding(card, 0.0), Message::OpenPost(place))
}

/// The patch notes at the first `limit` places in the feed, in rows of `columns` cards.
pub fn patches<'a>(feed: &'a Feed, limit: usize, pictures: &'a HashMap<String, image::Handle>, columns: usize) -> Element<'a, Message> {
    grid(
        feed.patches
            .iter()
            .enumerate()
            .take(limit)
            .map(|(place, patch)| patch_card(patch, place, pictures))
            .collect(),
        columns,
    )
}

/// One patch note as a card: its picture, its version and kind, and its title. A press reads it in the launcher.
fn patch_card<'a>(
    patch: &'a piston_mc::patch_notes::java::JavaPatchEntry,
    place: usize,
    pictures: &'a HashMap<String, image::Handle>,
) -> Element<'a, Message> {
    let kind = kind_label(patch.patch_type.as_deref());
    let mut badges = iced::widget::row![Badge::new(patch.version.clone(), color::primary(), color::primary_tint()).compact()].spacing(6);
    if !kind.is_empty() {
        badges = badges.push(Badge::new(kind, color::text_muted(), color::surface_bg3()).compact());
    }
    let text = column![
        badges,
        Text::new(patch.title.clone())
            .size(15)
            .font(fonts::Inter::SEMI_BOLD)
            .color(color::text_headline()),
    ]
    .spacing(8)
    .width(Fill);
    let card = column![
        cover(pictures, &absolute(&patch.image.url), COVER_HEIGHT),
        container(text).padding(Padding::new(14.0)).width(Fill),
    ];
    clickable(panel_with_padding(card, 0.0), Message::ReadPatch(place))
}

/// Makes a card react to a press, with the pointer shown over it.
pub fn clickable<'a>(card: impl Into<Element<'a, Message>>, message: Message) -> Element<'a, Message> {
    mouse_area(card).on_press(message).interaction(mouse::Interaction::Pointer).into()
}

/// The cards in rows of `columns`, each card as wide as its share of its row. A last row short of cards keeps its width.
fn grid<'a>(cards: Vec<Element<'a, Message>>, columns: usize) -> Element<'a, Message> {
    let columns = columns.max(1);
    let mut rows = Column::new().spacing(16).width(Fill);
    let mut cards = cards.into_iter();
    loop {
        let mut line = Row::new().spacing(16).width(Fill);
        let mut filled = 0;
        for _ in 0..columns {
            match cards.next() {
                Some(card) => {
                    line = line.push(container(card).width(Fill));
                    filled += 1;
                }
                None => line = line.push(Space::new().width(Fill)),
            }
        }
        if filled == 0 {
            break;
        }
        rows = rows.push(line);
    }
    rows.into()
}
