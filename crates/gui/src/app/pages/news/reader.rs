//! The patch note reader: a way back to the list, then a Java Edition patch note's version and kind, its title, its picture,
//! and its body, drawn as markdown. Links in the body open in the browser.

use std::collections::HashMap;

use iced::widget::{Space, Text, column, container, image, markdown, rich_text, row};
use iced::{Element, Fill, Padding, Theme};

use super::grid::{cover, kind_label};
use super::{Message, Reading};
use crate::app::components::TextButton;
use crate::app::components::badge::Badge;
use crate::app::lang;
use crate::app::news::feed::{Feed, absolute};
use crate::app::theme::{color, fonts};

/// The height of the reader's picture.
const HEIGHT: f32 = 260.0;

/// The space under the body, so the last line clears the bottom of the scroll area.
const BOTTOM: f32 = 32.0;

/// The reader: the patch note at `reading`, or nothing when the feed no longer has it.
pub fn view<'a>(reading: &'a Reading, feed: &'a Feed, pictures: &'a HashMap<String, image::Handle>) -> Element<'a, Message> {
    let Some(patch) = feed.patches.get(reading.index) else {
        return Space::new().into();
    };
    let back = TextButton::ghost(lang::t("news.reader.back")).on_press(Message::CloseReader);
    let kind = kind_label(patch.patch_type.as_deref());
    let mut badges = row![Badge::new(patch.version.clone(), color::primary(), color::primary_tint()).compact()].spacing(6);
    if !kind.is_empty() {
        badges = badges.push(Badge::new(kind, color::text_muted(), color::surface_bg3()).compact());
    }
    let article = markdown::view_with(reading.article.items(), settings(), &PatchViewer { pictures });
    column![
        back,
        badges,
        Text::new(patch.title.clone())
            .size(26)
            .font(fonts::Inter::EXTRA_BOLD)
            .color(color::text_headline()),
        cover(pictures, &absolute(&patch.image.url), HEIGHT),
        container(article).width(Fill),
    ]
    .spacing(16)
    .width(Fill)
    .padding(Padding::new(0.0).bottom(BOTTOM))
    .into()
}

/// The markdown settings the reader draws with: body text at 14 px, and the platform's link colour.
fn settings() -> markdown::Settings {
    let style = markdown::Style {
        link_color: color::primary(),
        ..markdown::Style::from(&Theme::Dark)
    };
    markdown::Settings::with_text_size(14, style)
}

/// Draws a patch note's pictures from the ones the page has loaded, and its links as page messages. A picture still
/// loading, or one that failed, shows its alt text.
struct PatchViewer<'a> {
    pictures: &'a HashMap<String, image::Handle>,
}

impl<'a> markdown::Viewer<'a, Message> for PatchViewer<'a> {
    fn on_link_click(url: markdown::Uri) -> Message {
        Message::Link(url)
    }

    fn image(&self, settings: markdown::Settings, url: &'a markdown::Uri, _title: &'a str, alt: &markdown::Text) -> Element<'a, Message> {
        match self.pictures.get(url) {
            Some(handle) => image(handle.clone()).width(Fill).into(),
            None => container(rich_text(alt.spans(settings.style)).on_link_click(Self::on_link_click))
                .padding(settings.spacing.0)
                .into(),
        }
    }
}
