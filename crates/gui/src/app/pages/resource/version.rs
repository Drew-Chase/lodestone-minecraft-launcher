//! A release's page: its name and channel, the action that takes it, its compatibility, its changes, what a modpack
//! bundles, and its files.

use aurora_leaf::Icon;
use iced::widget::{Space, Text, column, markdown, row, text};
use iced::{Element, Fill, Length, alignment};
use lodestone_platforms::{Channel, ContentType, ContentVersion};

use super::header::primary_action;
use super::parts::{chip, chip_row, heading, loader_label, picture, section, stat};
use super::readme::{ReadmeViewer, settings};
use super::{Load, Message, ResourcePage};
use crate::app::components::TextButton;
use crate::app::components::badge::Badge;
use crate::app::lang;
use crate::app::pages::discover::{ProjectStatus, Source, date_only, short_count};
use crate::app::theme::{color, fonts};

/// The width of the file size column.
const SIZE_WIDTH: f32 = 96.0;

/// The release's page: its header, then its facts and sections, which scroll with the page.
pub(super) fn view<'a>(page: &'a ResourcePage, index: usize, version: &'a ContentVersion, status: &ProjectStatus) -> Element<'a, Message> {
    let Some(listing) = page.listing() else {
        return Space::new().into();
    };
    let name = version.name.as_deref().unwrap_or(version.version_number.as_str());
    let mut title = vec![
        Text::new(name)
            .size(24)
            .font(fonts::Inter::EXTRA_BOLD)
            .color(color::text_headline())
            .into(),
    ];
    let mut numbers = vec![
        Text::new(version.version_number.as_str())
            .size(12)
            .font(fonts::JetBrainsMono::REGULAR)
            .color(color::text_muted())
            .into(),
    ];
    if let Some(channel) = version.channel {
        numbers.push(channel_badge(channel));
    }
    title.push(row(numbers).spacing(10).align_y(alignment::Vertical::Center).into());
    let back = TextButton::ghost(lang::t("resource.versions.back")).on_press(Message::CloseVersion);
    let header = row![
        column![back, column(title).spacing(6)].spacing(12).width(Fill),
        primary_action(listing.content, status, Some(version.id.as_str())),
    ]
    .spacing(20)
    .align_y(alignment::Vertical::Top);

    let mut facts = vec![stat(Icon::lucide().download(), short_count(version.downloads))];
    if let Some(date) = version.date.as_deref() {
        facts.push(stat(
            Icon::lucide().clock(),
            lang::t_with("resource.published", &[("date", date_only(Some(date)))]),
        ));
    }
    let mut sections: Vec<Element<'a, Message>> = vec![row(facts).spacing(16).align_y(alignment::Vertical::Center).into()];
    sections.push(compatibility(version));
    sections.push(section(column![heading(lang::t("resource.changes")), changes(page, index)].spacing(12)));
    if listing.source == Source::Modrinth && listing.content == ContentType::Modpack {
        sections.push(section(column![heading(lang::t("resource.contents")), contents(page)].spacing(12)));
    }
    if !version.files.is_empty() {
        sections.push(section(column![heading(lang::t("resource.files")), files(version)].spacing(12)));
    }
    column![header, column(sections).spacing(16).width(Fill)].spacing(24).width(Fill).into()
}

/// The release's game versions and loaders, as chips.
fn compatibility<'a>(version: &ContentVersion) -> Element<'a, Message> {
    let games: Vec<Element<'a, Message>> = version.minecraft_versions.iter().map(|game| chip(game.as_str())).collect();
    let loaders: Vec<Element<'a, Message>> = version.loaders.iter().map(|loader| chip(loader_label(loader))).collect();
    let mut body = vec![heading(lang::t("resource.minecraft")), chip_row(games)];
    if !loaders.is_empty() {
        body.push(heading(lang::t("resource.platforms")));
        body.push(chip_row(loaders));
    }
    section(column(body).spacing(10))
}

/// The release's changelog, as the page parsed it when the release was opened. A release with none says so.
fn changes<'a>(page: &'a ResourcePage, index: usize) -> Element<'a, Message> {
    match page.changelogs.get(&index) {
        Some(content) => markdown::view_with(content.items(), settings(), &ReadmeViewer { pictures: &page.pictures }),
        None => text(lang::t("resource.changes_none"))
            .size(13)
            .font(fonts::Inter::REGULAR)
            .color(color::text_muted())
            .into(),
    }
}

/// The projects a modpack release bundles: each one's icon, name and release, or what stands in for them while they
/// load or when there are none.
fn contents<'a>(page: &'a ResourcePage) -> Element<'a, Message> {
    match &page.contents {
        Load::Loading => text(lang::t("resource.loading")).size(13).color(color::text_muted()).into(),
        Load::Failed(error) => text(lang::t_with("resource.error", &[("error", error.as_str())]))
            .size(13)
            .color(color::text_muted())
            .into(),
        Load::Loaded(projects) if projects.is_empty() => text(lang::t("resource.contents_none")).size(13).color(color::text_muted()).into(),
        Load::Loaded(projects) => column(projects.iter().map(|project| {
            let number = project.version_number.clone().unwrap_or_default();
            row![
                picture(project.icon_url.as_deref().and_then(|url| page.icon(url, super::SMALL_ICON)), 32.0),
                column![
                    Text::new(project.name.as_str())
                        .size(13)
                        .font(fonts::Inter::SEMI_BOLD)
                        .color(color::text_headline()),
                    Text::new(number).size(11).font(fonts::JetBrainsMono::REGULAR).color(color::text_muted()),
                ]
                .spacing(2),
            ]
            .spacing(12)
            .align_y(alignment::Vertical::Center)
            .into()
        }))
        .spacing(10)
        .into(),
    }
}

/// The release's files: each one's name, size and whether it is the primary one.
fn files<'a>(version: &'a ContentVersion) -> Element<'a, Message> {
    column(version.files.iter().map(|file| {
        let mut line = vec![
            Text::new(file.name.as_str())
                .size(12)
                .font(fonts::JetBrainsMono::REGULAR)
                .color(color::text_default())
                .width(Fill)
                .into(),
        ];
        line.push(
            Text::new(file.size.map(size_label).unwrap_or_default())
                .size(12)
                .font(fonts::JetBrainsMono::REGULAR)
                .color(color::text_muted())
                .width(Length::Fixed(SIZE_WIDTH))
                .into(),
        );
        if file.primary {
            line.push(
                Badge::new(lang::t("resource.files_primary"), color::primary(), color::primary_tint())
                    .compact()
                    .into(),
            );
        }
        row(line).spacing(12).align_y(alignment::Vertical::Center).into()
    }))
    .spacing(8)
    .into()
}

/// A channel's badge. A release is lit, and the others are dimmed.
fn channel_badge<'a>(channel: Channel) -> Element<'a, Message> {
    let (label, ink, fill) = match channel {
        Channel::Release => (lang::t("resource.channel.release"), color::primary(), color::primary_tint()),
        Channel::Beta => (lang::t("resource.channel.beta"), color::text_muted(), color::surface_bg3()),
        Channel::Alpha => (lang::t("resource.channel.alpha"), color::text_muted(), color::surface_bg3()),
    };
    Badge::new(label, ink, fill).compact().into()
}

/// A file's size, in KiB or MiB.
fn size_label(bytes: u64) -> String {
    const MIB: f64 = 1024.0 * 1024.0;
    let size = bytes as f64;
    if size >= MIB {
        format!("{:.2} MiB", size / MIB)
    } else {
        format!("{:.1} KiB", size / 1024.0)
    }
}
