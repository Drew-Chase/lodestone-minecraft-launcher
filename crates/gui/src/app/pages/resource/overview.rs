//! The overview: the project's readme, or its HTML description, beside the sidebar of its facts.

use iced::widget::{column, container, markdown, row, text};
use iced::{Element, Fill, Length, Size};
use lodestone_platforms::DescriptionFormat;

use super::parts::section;
use super::readme::{ReadmeViewer, settings};
use super::{Action, Load, Message, ResourcePage, sidebar};
use crate::app::components::{TextButton, html};
use crate::app::lang;
use crate::app::theme::{color, fonts};

/// The sidebar's width.
const SIDEBAR_WIDTH: f32 = 280.0;
/// The gap between the readme and the sidebar.
const GAP: f32 = 24.0;

/// The readme in its panel, with the sidebar beside it. The sidebar drops below the readme on a narrow window.
pub(super) fn view<'a>(page: &'a ResourcePage, window: Size) -> Element<'a, Message> {
    let main = container(section(body(page))).width(Fill);
    if window.width < 1000.0 {
        column![main, sidebar::view(page)].spacing(GAP).width(Fill).into()
    } else {
        row![main, container_width(sidebar::view(page), SIDEBAR_WIDTH),]
            .spacing(GAP)
            .width(Fill)
            .into()
    }
}

/// The readme as markdown, or the description drawn from its HTML, or what stands in for them: the summary and a way
/// to the platform's page, or a note that the project has nothing to read.
fn body<'a>(page: &'a ResourcePage) -> Element<'a, Message> {
    if let Some(content) = &page.readme {
        return markdown::view_with(content.items(), settings(), &ReadmeViewer { pictures: &page.pictures });
    }
    if let Some(description) = page.description.as_ref().filter(|description| !description.is_empty()) {
        let viewer = html::Viewer {
            pictures: &page.pictures,
            open: &page.spoilers,
            on_link: Message::Link,
            on_toggle: Message::Spoiler,
        };
        return html::view(description, &viewer);
    }
    match &page.details {
        Load::Loading => text_line(lang::t("resource.loading")),
        Load::Failed(error) => text_line(lang::t_with("resource.error", &[("error", error.as_str())])),
        Load::Loaded(details) => {
            let mut lines = column![];
            let summary = details.summary.as_str();
            if !summary.is_empty() {
                lines = lines.push(text_line(summary));
            }
            // An HTML description that did not come, or has nothing drawable, is still on the platform's page.
            if details.description_format == DescriptionFormat::Html {
                let platform = page.listing().map_or("", |listing| listing.source.name());
                lines = lines
                    .push(TextButton::ghost(lang::t_with("resource.read_on", &[("platform", platform)])).on_press(Message::Act(Action::OpenWeb)));
            } else if summary.is_empty() {
                lines = lines.push(text_line(lang::t("resource.no_description")));
            }
            lines.spacing(14).into()
        }
    }
}

/// A line of body text.
fn text_line<'a>(line: impl Into<String>) -> Element<'a, Message> {
    text(line.into())
        .size(14)
        .font(fonts::Inter::REGULAR)
        .color(color::text_default())
        .width(Fill)
        .into()
}

/// The content at a fixed width.
fn container_width<'a>(content: Element<'a, Message>, width: f32) -> Element<'a, Message> {
    container(content).width(Length::Fixed(width)).into()
}
