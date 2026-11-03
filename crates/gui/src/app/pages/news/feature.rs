//! The featured post: the newest post the filter lets through, drawn wide above the grid.

use std::collections::HashMap;

use iced::widget::{Text, column, container, image, row};
use iced::{Element, Fill, Padding, alignment};

use super::Message;
use super::grid::{category_badge, clickable, cover, date_label};
use crate::app::components::panel::panel_with_padding;
use crate::app::news::feed::{Feed, absolute};
use crate::app::theme::{color, fonts};

/// The height of the featured post's wide picture.
const HEIGHT: f32 = 300.0;

/// The post at `place` in the feed, drawn wide: its picture, its category and date, its title and summary. A press
/// opens the post on minecraft.net.
pub fn view<'a>(feed: &'a Feed, place: usize, pictures: &'a HashMap<String, image::Handle>) -> Element<'a, Message> {
    let post = &feed.posts[place];
    let meta = row![
        category_badge(&post.category),
        Text::new(date_label(&post.date))
            .size(11)
            .font(fonts::JetBrainsMono::REGULAR)
            .color(color::text_placeholder()),
    ]
    .spacing(10)
    .align_y(alignment::Vertical::Center);
    let text = column![
        meta,
        Text::new(post.title.clone())
            .size(22)
            .font(fonts::Inter::EXTRA_BOLD)
            .color(color::text_headline()),
        Text::new(post.text.clone())
            .size(14)
            .font(fonts::Inter::REGULAR)
            .color(color::text_muted()),
    ]
    .spacing(8)
    .width(Fill);
    let card = column![
        cover(pictures, &absolute(&post.news_page_image.url), HEIGHT),
        container(text).padding(Padding::new(20.0)).width(Fill),
    ];
    clickable(panel_with_padding(card, 0.0), Message::OpenPost(place))
}
