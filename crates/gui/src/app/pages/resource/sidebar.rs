//! The sidebar beside the readme: the project's compatibility, links, tags, creators and details. A section shows only
//! when the platform has what it lists.

use iced::widget::{Space, Text, column, row};
use iced::{Element, Fill, alignment};
use lodestone_platforms::{ContentDetails, Creator, ProjectLinks, Side};

use super::parts::{chip, chip_row, heading, loader_label, picture, section};
use super::{Message, ResourcePage};
use crate::app::components::TextButton;
use crate::app::lang;
use crate::app::pages::discover::date_only;
use crate::app::theme::{color, fonts};

/// The most game versions the compatibility section lists before it says how many more there are.
const VERSION_LIMIT: usize = 16;

/// The sidebar: one section per kind of fact the project has.
pub(super) fn view<'a>(page: &'a ResourcePage) -> Element<'a, Message> {
    let details = page.details();
    let mut sections: Vec<Element<'a, Message>> = vec![compatibility(page, details)];
    if let Some(details) = details {
        if has_links(&details.info.links) {
            sections.push(section(
                column![heading(lang::t("resource.links")), links(&details.info.links)].spacing(10),
            ));
        }
        if !details.info.categories.is_empty() {
            let tags: Vec<Element<'a, Message>> = details.info.categories.iter().map(|tag| chip(tag.as_str())).collect();
            sections.push(section(column![heading(lang::t("resource.tags")), chip_row(tags)].spacing(10)));
        }
        if !details.info.creators.is_empty() {
            sections.push(section(
                column![heading(lang::t("resource.creators")), creators(page, &details.info.creators)].spacing(12),
            ));
        }
        sections.push(section(column![heading(lang::t("resource.details")), facts(details)].spacing(10)));
    }
    column(sections).spacing(16).width(Fill).into()
}

/// The Minecraft versions, the loaders and the sides the project runs on. The versions fall back to the releases'
/// when the platform gives the project none.
fn compatibility<'a>(page: &'a ResourcePage, details: Option<&'a ContentDetails>) -> Element<'a, Message> {
    let versions: Vec<&str> = match details {
        Some(details) if !details.info.game_versions.is_empty() => details.info.game_versions.iter().map(String::as_str).collect(),
        _ => page.game_versions.iter().map(String::as_str).collect(),
    };
    let mut shown: Vec<Element<'a, Message>> = versions.iter().take(VERSION_LIMIT).map(|version| chip(*version)).collect();
    if versions.len() > VERSION_LIMIT {
        shown.push(chip(lang::t_with(
            "resource.more",
            &[("count", (versions.len() - VERSION_LIMIT).to_string().as_str())],
        )));
    }
    let mut body = vec![heading(lang::t("resource.minecraft")), chip_row(shown)];
    if let Some(details) = details {
        if !details.info.loaders.is_empty() {
            let loaders: Vec<Element<'a, Message>> = details.info.loaders.iter().map(|loader| chip(loader_label(loader))).collect();
            body.push(heading(lang::t("resource.platforms")));
            body.push(chip_row(loaders));
        }
        if !details.info.environments.is_empty() {
            let sides: Vec<Element<'a, Message>> = details
                .info
                .environments
                .iter()
                .map(|side| {
                    chip(match side {
                        Side::Client => lang::t("resource.environment.client"),
                        Side::Server => lang::t("resource.environment.server"),
                    })
                })
                .collect();
            body.push(heading(lang::t("resource.environments")));
            body.push(chip_row(sides));
        }
    }
    section(column(body).spacing(10))
}

/// Whether the project lists any link.
fn has_links(links: &ProjectLinks) -> bool {
    links.issues.is_some() || links.source.is_some() || links.wiki.is_some() || links.discord.is_some() || !links.donate.is_empty()
}

/// The links the project lists, each a button that opens it.
fn links<'a>(links: &ProjectLinks) -> Element<'a, Message> {
    let named = [
        ("resource.link.issues", links.issues.as_deref()),
        ("resource.link.source", links.source.as_deref()),
        ("resource.link.wiki", links.wiki.as_deref()),
        ("resource.link.discord", links.discord.as_deref()),
    ];
    let mut rows: Vec<Element<'a, Message>> = Vec::new();
    for (label, url) in named {
        if let Some(url) = url {
            rows.push(TextButton::ghost(lang::t(label)).on_press(Message::Link(url.to_string())).into());
        }
    }
    for url in &links.donate {
        rows.push(
            TextButton::ghost(lang::t("resource.link.donate"))
                .on_press(Message::Link(url.clone()))
                .into(),
        );
    }
    column(rows).spacing(4).into()
}

/// The team: each member's avatar, name and role.
fn creators<'a>(page: &'a ResourcePage, creators: &'a [Creator]) -> Element<'a, Message> {
    column(creators.iter().map(|creator| {
        let mut words = vec![
            Text::new(creator.name.as_str())
                .size(13)
                .font(fonts::Inter::SEMI_BOLD)
                .color(color::text_headline())
                .into(),
        ];
        if let Some(role) = &creator.role {
            words.push(
                Text::new(role.as_str())
                    .size(11)
                    .font(fonts::Inter::REGULAR)
                    .color(color::text_muted())
                    .into(),
            );
        }
        row![
            picture(creator.avatar_url.as_deref().and_then(|url| page.picture(url)), 32.0),
            column(words).spacing(2),
        ]
        .spacing(10)
        .align_y(alignment::Vertical::Center)
        .into()
    }))
    .spacing(12)
    .into()
}

/// The licence and the dates the project was published and updated.
fn facts<'a>(details: &'a ContentDetails) -> Element<'a, Message> {
    let mut rows: Vec<Element<'a, Message>> = Vec::new();
    if let Some(license) = &details.info.license {
        rows.push(fact_row(lang::t("resource.detail.license"), license));
    }
    if let Some(published) = &details.info.published {
        rows.push(fact_row(lang::t("resource.detail.published"), date_only(Some(published.as_str()))));
    }
    if let Some(updated) = &details.info.updated {
        rows.push(fact_row(lang::t("resource.detail.updated"), date_only(Some(updated.as_str()))));
    }
    column(rows).spacing(8).into()
}

/// One fact: its label on the left, and its value on the right.
fn fact_row<'a>(label: &'static str, value: &str) -> Element<'a, Message> {
    row![
        Text::new(label).size(12).font(fonts::Inter::REGULAR).color(color::text_muted()),
        Space::new().width(Fill),
        Text::new(value.to_string())
            .size(12)
            .font(fonts::Inter::MEDIUM)
            .color(color::text_default()),
    ]
    .align_y(alignment::Vertical::Center)
    .into()
}
