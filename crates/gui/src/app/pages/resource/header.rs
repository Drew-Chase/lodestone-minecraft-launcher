//! The resource page's header: the project's icon, name, author, summary and facts, with the action it offers.

use aurora_leaf::Icon;
use iced::widget::{Space, Text, column, row};
use iced::{Element, Length, alignment};

use super::parts::{chip, dimmed, picture, primary, stat};
use super::{Action, Listing, Message, ResourcePage};
use crate::app::components::hint::hint;
use crate::app::components::icon_button::GHOST_BUTTON_STYLE;
use crate::app::components::{IconButton, Select};
use crate::app::lang;
use crate::app::pages::discover::{InstallState, ProjectStatus, date_only, short_count};
use crate::app::theme::{color, fonts};
use lodestone_platforms::ContentType;

/// The side of the project's picture in the header.
const ICON_SIZE: f32 = 96.0;
/// The width of the header's instance selector.
const SELECT_WIDTH: f32 = 220.0;
/// The most tags the header lists.
const TAG_LIMIT: usize = 6;

/// The header: the project beside its name, author, summary and facts, and the action it offers on the right.
pub(super) fn view<'a>(page: &'a ResourcePage, status: &ProjectStatus) -> Element<'a, Message> {
    let Some(listing) = page.listing() else {
        return Space::new().into();
    };
    let details = page.details();
    let summary = details.map_or(listing.summary.as_str(), |details| details.summary.as_str());
    let author = details.and_then(|details| details.author.clone()).or_else(|| listing.author.clone());
    let downloads = details.map_or(listing.downloads, |details| details.downloads);
    let updated = details
        .and_then(|details| details.info.updated.clone())
        .or_else(|| listing.updated.clone());
    let followers = details.and_then(|details| details.info.followers);

    let mut info: Vec<Element<'a, Message>> = vec![
        Text::new(listing.name.as_str())
            .size(24)
            .font(fonts::Inter::EXTRA_BOLD)
            .color(color::Text::HEADLINE)
            .into(),
    ];
    if let Some(author) = author {
        info.push(
            Text::new(lang::t_with("resource.by", &[("author", author.as_str())]))
                .size(12)
                .font(fonts::Inter::REGULAR)
                .color(color::Text::MUTED)
                .into(),
        );
    }
    info.push(
        Text::new(summary)
            .size(14)
            .font(fonts::Inter::REGULAR)
            .color(color::Text::DEFAULT)
            .width(Length::Fixed(560.0))
            .into(),
    );
    let mut stats = vec![stat(Icon::lucide().download(), short_count(downloads))];
    if let Some(followers) = followers {
        stats.push(stat(Icon::lucide().heart(), short_count(followers)));
    }
    if let Some(updated) = updated.as_deref() {
        stats.push(stat(
            Icon::lucide().clock(),
            lang::t_with("resource.updated", &[("date", date_only(Some(updated)))]),
        ));
    }
    info.push(row(stats).spacing(16).align_y(alignment::Vertical::Center).into());
    if let Some(details) = details {
        let tags: Vec<Element<'a, Message>> = details.info.categories.iter().take(TAG_LIMIT).map(|tag| chip(tag.as_str())).collect();
        if !tags.is_empty() {
            info.push(row(tags).spacing(6).into());
        }
    }

    row![
        picture(listing.icon_url.as_deref().and_then(|url| page.picture(url)), ICON_SIZE),
        column(info).spacing(8).width(Length::Fill),
        actions(listing, status),
    ]
    .spacing(20)
    .align_y(alignment::Vertical::Top)
    .into()
}

/// The header's actions: the instance selector for a mod, the action the project offers, and the link to its page.
fn actions<'a>(listing: &'a Listing, status: &ProjectStatus) -> Element<'a, Message> {
    let mut rows: Vec<Element<'a, Message>> = Vec::new();
    if listing.content == ContentType::Mod {
        rows.push(instance_select(status));
    }
    let web = IconButton::new(Icon::lucide().external_link())
        .size(36.0)
        .radius(10.0)
        .icon_size(14.0)
        .style(GHOST_BUTTON_STYLE);
    let web = match listing.web_url {
        Some(_) => web.on_press(Message::Act(Action::OpenWeb)),
        None => web.disabled(true),
    };
    rows.push(
        row![
            primary_action(listing.content, status, None),
            hint(web, lang::t("resource.action.open_web")),
        ]
        .spacing(10)
        .align_y(alignment::Vertical::Center)
        .into(),
    );
    column(rows).spacing(10).align_x(alignment::Horizontal::Right).into()
}

/// The action a project offers, as its kind and its state say: install a modpack, add a mod to the instance picked,
/// or download anything else into the shared folder. `version` is the release the action takes, or `None` for the
/// newest that fits.
pub(super) fn primary_action<'a>(content: ContentType, status: &ProjectStatus, version: Option<&str>) -> Element<'a, Message> {
    let version = version.map(str::to_string);
    match content {
        ContentType::Modpack => match &status.install {
            Some(InstallState::Installed(instance)) => primary(
                lang::t("resource.action.open_instance"),
                Icon::lucide().external_link(),
                Action::OpenInstance(instance.clone()),
            ),
            Some(InstallState::Queued) => dimmed(lang::t("resource.action.queued")),
            Some(InstallState::Preparing) => dimmed(lang::t("resource.action.preparing")),
            None => primary(lang::t("resource.action.install"), Icon::lucide().download(), Action::Install(version)),
        },
        ContentType::Mod => {
            if status.added {
                dimmed(lang::t("resource.action.added"))
            } else if status.adding {
                dimmed(lang::t("resource.action.adding"))
            } else if status.target.as_ref().is_some_and(|target| target.takes_mods()) {
                primary(lang::t("resource.action.add"), Icon::lucide().plus(), Action::Add(version))
            } else {
                dimmed(lang::t("resource.action.add"))
            }
        }
        ContentType::ResourcePack | ContentType::Shader | ContentType::DataPack | ContentType::World => {
            if status.in_library {
                dimmed(lang::t("resource.action.in_library"))
            } else if status.downloading {
                dimmed(lang::t("resource.action.downloading"))
            } else {
                primary(lang::t("resource.action.download"), Icon::lucide().download(), Action::Download(version))
            }
        }
    }
}

/// The selector for the instance a mod goes to. Dimmed while the library holds none.
fn instance_select<'a>(status: &ProjectStatus) -> Element<'a, Message> {
    let picker = Select::new(status.instances.clone(), status.target_index)
        .leading_icon(Icon::lucide().package())
        .on_select(|index| Message::Act(Action::PickInstance(index)))
        .width(Length::Fixed(SELECT_WIDTH));
    if status.instances.is_empty() {
        picker.placeholder(lang::t("discover.mods.no_instances")).disabled(true).into()
    } else {
        picker.placeholder(lang::t("discover.mods.choose_instance")).into()
    }
}
