//! The changelog: each version's notes in the order the platform lists them, a page at a time.

use aurora_leaf::Icon;
use iced::widget::{Space, Text, column, container, markdown, row, text};
use iced::{Element, Fill, alignment};
use lodestone_platforms::{Channel, ContentVersion};

use super::parts::{pager, row_action, section, stat};
use super::readme::{ReadmeViewer, settings};
use super::{Message, PER_PAGE, ResourcePage, pages_of};
use crate::app::components::badge::Badge;
use crate::app::lang;
use crate::app::pages::discover::{date_only, short_count};
use crate::app::theme::{color, fonts};

/// The changelog: the page's versions, each with its notes, and the pager under them.
pub(super) fn view<'a>(page: &'a ResourcePage) -> Element<'a, Message> {
    let versions = page.loaded_versions();
    let pages = pages_of(versions.len());
    let current = page.changelog_page.min(pages - 1);
    let entries: Vec<Element<'a, Message>> = versions
        .iter()
        .enumerate()
        .skip(current * PER_PAGE)
        .take(PER_PAGE)
        .map(|(index, version)| entry(page, index, version))
        .collect();
    column![
        column(entries).spacing(16).width(Fill),
        container_center(pager(current, pages, Message::ChangelogPage)),
    ]
    .spacing(24)
    .width(Fill)
    .into()
}

/// One version: its name, channel, date and counts, its actions, and its notes.
fn entry<'a>(page: &'a ResourcePage, index: usize, version: &'a ContentVersion) -> Element<'a, Message> {
    let name = version.name.as_deref().unwrap_or(version.version_number.as_str());
    let mut head = vec![Text::new(name).size(15).font(fonts::Inter::BOLD).color(color::Text::HEADLINE).into()];
    if let Some(channel) = version.channel {
        head.push(channel_badge(channel));
    }
    head.push(Space::new().width(Fill).into());
    if let Some(date) = version.date.as_deref() {
        head.push(stat(Icon::lucide().clock(), date_only(Some(date)).to_string()));
    }
    head.push(stat(Icon::lucide().download(), short_count(version.downloads)));
    head.push(match page.listing() {
        Some(listing) => row_action(listing.content, &version.id),
        None => Space::new().into(),
    });
    let notes = match page.changelogs.get(&index) {
        Some(content) => markdown::view_with(content.items(), settings(), &ReadmeViewer { pictures: &page.pictures }),
        None => text(lang::t("resource.changelog.none"))
            .size(13)
            .font(fonts::Inter::REGULAR)
            .color(color::Text::MUTED)
            .into(),
    };
    section(column![row(head).spacing(12).align_y(alignment::Vertical::Center), notes,].spacing(12))
}

/// The channel's badge. A release is lit, and the others are dimmed.
fn channel_badge<'a>(channel: Channel) -> Element<'a, Message> {
    let (label, ink, fill) = match channel {
        Channel::Release => (lang::t("resource.channel.release"), color::Primary::DEFAULT, color::Primary::TINT),
        Channel::Beta => (lang::t("resource.channel.beta"), color::Text::MUTED, color::Surface::BG3),
        Channel::Alpha => (lang::t("resource.channel.alpha"), color::Text::MUTED, color::Surface::BG3),
    };
    Badge::new(label, ink, fill).compact().into()
}

/// Centres a row of controls under the list.
fn container_center<'a>(content: Element<'a, Message>) -> Element<'a, Message> {
    container(content).width(Fill).align_x(alignment::Horizontal::Center).into()
}
