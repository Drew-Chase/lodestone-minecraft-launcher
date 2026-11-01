//! The Versions tab: the platform's releases in a table, filtered by channel and game version and a page at a time.
//! A press on a row opens that release's page.

use iced::mouse;
use iced::widget::{Space, Text, column, container, mouse_area, row};
use iced::{Element, Fill, Length, Padding, alignment};
use lodestone_platforms::ContentVersion;

use super::parts::{chip, loader_label, pager, row_action, section};
use super::{CHANNELS, Message, PER_PAGE, ResourcePage, pages_of};
use crate::app::components::Select;
use crate::app::lang;
use crate::app::pages::discover::{date_only, short_count};
use crate::app::theme::{color, fonts};

/// The width of each filter's selector.
const FILTER_WIDTH: f32 = 176.0;
/// The width of the table's game version, platform, date and downloads columns, and of its action.
const GAME_WIDTH: f32 = 150.0;
const PLATFORM_WIDTH: f32 = 150.0;
const DATE_WIDTH: f32 = 100.0;
const COUNT_WIDTH: f32 = 90.0;
const ACTION_WIDTH: f32 = 40.0;
/// The most game versions a row lists before it says how many more there are.
const GAME_VERSIONS_PER_ROW: usize = 2;

/// The Versions tab: the filters, the page of releases as a table, and the pager under it.
pub(super) fn view<'a>(page: &'a ResourcePage) -> Element<'a, Message> {
    let matched = page.filtered_versions();
    let pages = pages_of(matched.len());
    let current = page.versions_page.min(pages - 1);
    let versions = page.loaded_versions();
    let mut rows: Vec<Element<'a, Message>> = matched
        .iter()
        .skip(current * PER_PAGE)
        .take(PER_PAGE)
        .map(|&index| table_row(page, index, &versions[index]))
        .collect();
    if rows.is_empty() {
        rows.push(
            Text::new(lang::t("resource.versions.none"))
                .size(13)
                .font(fonts::Inter::REGULAR)
                .color(color::Text::MUTED)
                .into(),
        );
    }
    column![
        filters(page, matched.len()),
        section(column![table_header(), column(rows).width(Fill)].spacing(4)),
        container(pager(current, pages, Message::VersionsPage))
            .width(Fill)
            .align_x(alignment::Horizontal::Center),
    ]
    .spacing(20)
    .width(Fill)
    .into()
}

/// The channel and game-version selectors, each shown when the releases carry what it filters by, and the count.
fn filters<'a>(page: &'a ResourcePage, count: usize) -> Element<'a, Message> {
    let mut controls: Vec<Element<'a, Message>> = Vec::new();
    if page.loaded_versions().iter().any(|version| version.channel.is_some()) {
        let options: Vec<String> = [
            lang::t("resource.filter.any_channel"),
            lang::t("resource.channel.release"),
            lang::t("resource.channel.beta"),
            lang::t("resource.channel.alpha"),
        ]
        .into_iter()
        .map(str::to_string)
        .collect();
        let selected = CHANNELS.iter().position(|channel| *channel == page.channel).unwrap_or(0);
        controls.push(
            Select::new(options, Some(selected))
                .on_select(Message::ChannelSelected)
                .width(Length::Fixed(FILTER_WIDTH))
                .into(),
        );
    }
    if !page.game_versions.is_empty() {
        let mut options = vec![lang::t("resource.filter.any_version").to_string()];
        options.extend(page.game_versions.iter().cloned());
        let selected = page
            .game_version
            .as_ref()
            .and_then(|wanted| page.game_versions.iter().position(|version| version == wanted))
            .map_or(0, |index| index + 1);
        controls.push(
            Select::new(options, Some(selected))
                .searchable()
                .on_select(Message::GameVersionSelected)
                .width(Length::Fixed(FILTER_WIDTH))
                .into(),
        );
    }
    controls.push(Space::new().width(Fill).into());
    controls.push(
        Text::new(lang::t_with("resource.versions.count", &[("count", count.to_string().as_str())]))
            .size(12)
            .font(fonts::JetBrainsMono::REGULAR)
            .color(color::Text::MUTED)
            .into(),
    );
    row(controls).spacing(8).align_y(alignment::Vertical::Center).width(Fill).into()
}

/// The table's heading row, labelled with the keys of its columns.
fn table_header<'a>() -> Element<'a, Message> {
    let label = |text: &'static str, width: Length| {
        Text::new(lang::t(text))
            .size(10)
            .font(fonts::JetBrainsMono::SEMI_BOLD)
            .color(color::Text::PLACEHOLDER)
            .width(width)
    };
    row![
        label("resource.table.version", Length::Fill),
        label("resource.table.game_version", Length::Fixed(GAME_WIDTH)),
        label("resource.table.platform", Length::Fixed(PLATFORM_WIDTH)),
        label("resource.table.published", Length::Fixed(DATE_WIDTH)),
        label("resource.table.downloads", Length::Fixed(COUNT_WIDTH)),
        Space::new().width(Length::Fixed(ACTION_WIDTH)),
    ]
    .spacing(12)
    .padding(Padding::new(0.0).left(16).right(16).top(8).bottom(8))
    .align_y(alignment::Vertical::Center)
    .into()
}

/// One release: its name and number, its game versions, platforms, date and downloads, and the action that takes it.
/// A press on the row opens the release's page.
fn table_row<'a>(page: &'a ResourcePage, index: usize, version: &'a ContentVersion) -> Element<'a, Message> {
    let name = version.name.as_deref().unwrap_or(version.version_number.as_str());
    let mut games: Vec<Element<'a, Message>> = version
        .minecraft_versions
        .iter()
        .take(GAME_VERSIONS_PER_ROW)
        .map(|game| chip(game.as_str()))
        .collect();
    let more = version.minecraft_versions.len().saturating_sub(GAME_VERSIONS_PER_ROW);
    if more > 0 {
        games.push(chip(lang::t_with("resource.more", &[("count", more.to_string().as_str())])));
    }
    let platforms: Vec<Element<'a, Message>> = version.loaders.iter().map(|loader| chip(loader_label(loader))).collect();
    let action = match page.listing() {
        Some(listing) => row_action(listing.content, &version.id),
        None => Space::new().into(),
    };
    let line = row![
        column![
            Text::new(name).size(14).font(fonts::Inter::SEMI_BOLD).color(color::Text::HEADLINE),
            Text::new(version.version_number.as_str())
                .size(11)
                .font(fonts::JetBrainsMono::REGULAR)
                .color(color::Text::PLACEHOLDER),
        ]
        .spacing(2)
        .width(Fill),
        row(games).spacing(4).width(Length::Fixed(GAME_WIDTH)),
        row(platforms).spacing(4).width(Length::Fixed(PLATFORM_WIDTH)),
        Text::new(date_only(version.date.as_deref()))
            .size(12)
            .font(fonts::JetBrainsMono::REGULAR)
            .color(color::Text::MUTED)
            .width(Length::Fixed(DATE_WIDTH)),
        Text::new(short_count(version.downloads))
            .size(12)
            .font(fonts::JetBrainsMono::REGULAR)
            .color(color::Text::MUTED)
            .width(Length::Fixed(COUNT_WIDTH)),
        container(action).width(Length::Fixed(ACTION_WIDTH)).align_x(alignment::Horizontal::Right),
    ]
    .spacing(12)
    .padding(Padding::new(0.0).left(16).right(16).top(12).bottom(12))
    .align_y(alignment::Vertical::Center);
    mouse_area(line)
        .on_press(Message::OpenVersion(index))
        .interaction(mouse::Interaction::Pointer)
        .into()
}
