//! The Versions tab: the platform's releases in a table, filtered by channel and game version and a page at a time.
//! A press on a row opens that release's page.

use iced::mouse;
use iced::widget::{Space, Text, column, container, mouse_area, row};
use iced::{Element, Fill, Length, Padding, Size, alignment};
use lodestone_platforms::{ContentType, ContentVersion};

use super::parts::{chip, loader_label, mod_row_action, mod_state, pager, row_action, section};
use super::{CHANNELS, Message, PER_PAGE, ResourcePage, pages_of};
use crate::app::components::Select;
use crate::app::components::page::content_width;
use crate::app::components::table::{self, Column, NAME_MIN};
use crate::app::lang;
use crate::app::pages::discover::{ProjectStatus, date_only, short_count};
use crate::app::theme::{color, fonts};

/// The width of each filter's selector.
const FILTER_WIDTH: f32 = 176.0;
/// The width of the table's game version, platform, date and downloads columns, and of its action.
const GAME_WIDTH: f32 = 150.0;
const PLATFORM_WIDTH: f32 = 150.0;
const DATE_WIDTH: f32 = 100.0;
const COUNT_WIDTH: f32 = 90.0;
const ACTION_WIDTH: f32 = 40.0;
/// The gap between the table's cells, and the padding of the section and of each row either side.
const CELL_GAP: f32 = 12.0;
const SECTION_PAD: f32 = 18.0;
const ROW_PAD: f32 = 16.0;
/// The most game versions a row lists before it says how many more there are.
const GAME_VERSIONS_PER_ROW: usize = 2;

/// Which of the table's columns show, in the order [game versions, platform, date, downloads, action]. The downloads hide
/// first, then the date, then the platform; the game versions and the action always show.
type Shown = [bool; 5];

/// The columns that show in a section `width` pixels wide.
fn shown_columns(section_width: f32) -> Shown {
    const COLUMNS: [Column; 5] = [
        Column {
            width: GAME_WIDTH,
            hide: None,
        },
        Column {
            width: PLATFORM_WIDTH,
            hide: Some(2),
        },
        Column {
            width: DATE_WIDTH,
            hide: Some(1),
        },
        Column {
            width: COUNT_WIDTH,
            hide: Some(0),
        },
        Column {
            width: ACTION_WIDTH,
            hide: None,
        },
    ];
    let available = section_width - 2.0 * SECTION_PAD - 2.0 * ROW_PAD;
    let shown = table::fit(&COLUMNS, available, NAME_MIN, CELL_GAP);
    [shown[0], shown[1], shown[2], shown[3], shown[4]]
}

/// The Versions tab: the filters, the page of releases as a table, and the pager under it.
pub(super) fn view<'a>(page: &'a ResourcePage, status: &ProjectStatus, window: Size) -> Element<'a, Message> {
    let matched = page.filtered_versions();
    let pages = pages_of(matched.len());
    let current = page.versions_page.min(pages - 1);
    let versions = page.loaded_versions();
    let shown = shown_columns(content_width(window.width));
    let mut rows: Vec<Element<'a, Message>> = matched
        .iter()
        .skip(current * PER_PAGE)
        .take(PER_PAGE)
        .map(|&index| table_row(page, status, index, &versions[index], shown))
        .collect();
    if rows.is_empty() {
        rows.push(
            Text::new(lang::t("resource.versions.none"))
                .size(13)
                .font(fonts::Inter::REGULAR)
                .color(color::text_muted())
                .into(),
        );
    }
    column![
        filters(page, matched.len()),
        section(column![table_header(shown), column(rows).width(Fill)].spacing(4)),
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
            .color(color::text_muted())
            .into(),
    );
    row(controls).spacing(8).align_y(alignment::Vertical::Center).width(Fill).into()
}

/// The table's heading row, labelled with the keys of its columns, with the columns `shown` gives.
fn table_header<'a>(shown: Shown) -> Element<'a, Message> {
    let label = |text: &'static str, width: Length| {
        Text::new(lang::t(text))
            .size(10)
            .font(fonts::JetBrainsMono::SEMI_BOLD)
            .color(color::text_placeholder())
            .wrapping(iced::widget::text::Wrapping::None)
            .width(width)
    };
    let mut cells: Vec<Element<'a, Message>> = vec![label("resource.table.version", Length::Fill).into()];
    if shown[0] {
        cells.push(label("resource.table.game_version", Length::Fixed(GAME_WIDTH)).into());
    }
    if shown[1] {
        cells.push(label("resource.table.platform", Length::Fixed(PLATFORM_WIDTH)).into());
    }
    if shown[2] {
        cells.push(label("resource.table.published", Length::Fixed(DATE_WIDTH)).into());
    }
    if shown[3] {
        cells.push(label("resource.table.downloads", Length::Fixed(COUNT_WIDTH)).into());
    }
    cells.push(Space::new().width(Length::Fixed(ACTION_WIDTH)).into());
    row(cells)
        .spacing(CELL_GAP)
        .padding(Padding::new(0.0).left(ROW_PAD).right(ROW_PAD).top(8).bottom(8))
        .align_y(alignment::Vertical::Center)
        .into()
}

/// The facts of a release whose columns are hidden, in one line under its name, so the row loses no information.
fn hidden_facts(version: &ContentVersion, shown: Shown) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    if !shown[0] {
        parts.push(
            version
                .minecraft_versions
                .iter()
                .take(GAME_VERSIONS_PER_ROW)
                .cloned()
                .collect::<Vec<_>>()
                .join(", "),
        );
    }
    if !shown[1] {
        parts.push(version.loaders.iter().map(|loader| loader_label(loader)).collect::<Vec<_>>().join(", "));
    }
    if !shown[2] {
        parts.push(date_only(version.date.as_deref()).to_string());
    }
    if !shown[3] {
        parts.push(short_count(version.downloads));
    }
    (!parts.is_empty()).then(|| parts.join("  ·  "))
}

/// One release: its name and number, its game versions, platforms, date and downloads, and the action that takes it.
/// A press on the row opens the release's page.
fn table_row<'a>(page: &'a ResourcePage, status: &ProjectStatus, index: usize, version: &'a ContentVersion, shown: Shown) -> Element<'a, Message> {
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
        Some(listing) if listing.content == ContentType::Mod => mod_row_action(mod_state(status, Some(&version.id)), &version.id),
        Some(listing) => row_action(listing.content, &version.id),
        None => Space::new().into(),
    };
    let mut name_lines = column![
        Text::new(name).size(14).font(fonts::Inter::SEMI_BOLD).color(color::text_headline()),
        Text::new(version.version_number.as_str())
            .size(11)
            .font(fonts::JetBrainsMono::REGULAR)
            .color(color::text_placeholder()),
    ]
    .spacing(2);
    if let Some(facts) = hidden_facts(version, shown) {
        name_lines = name_lines.push(Text::new(facts).size(11).font(fonts::Inter::REGULAR).color(color::text_muted()));
    }
    let mut cells: Vec<Element<'a, Message>> = vec![name_lines.width(Fill).into()];
    if shown[0] {
        cells.push(row(games).spacing(4).width(Length::Fixed(GAME_WIDTH)).into());
    }
    if shown[1] {
        cells.push(row(platforms).spacing(4).width(Length::Fixed(PLATFORM_WIDTH)).into());
    }
    if shown[2] {
        cells.push(
            Text::new(date_only(version.date.as_deref()))
                .size(12)
                .font(fonts::JetBrainsMono::REGULAR)
                .color(color::text_muted())
                .width(Length::Fixed(DATE_WIDTH))
                .into(),
        );
    }
    if shown[3] {
        cells.push(
            Text::new(short_count(version.downloads))
                .size(12)
                .font(fonts::JetBrainsMono::REGULAR)
                .color(color::text_muted())
                .width(Length::Fixed(COUNT_WIDTH))
                .into(),
        );
    }
    cells.push(
        container(action)
            .width(Length::Fixed(ACTION_WIDTH))
            .align_x(alignment::Horizontal::Right)
            .into(),
    );
    let line = row(cells)
        .spacing(CELL_GAP)
        .padding(Padding::new(0.0).left(ROW_PAD).right(ROW_PAD).top(12).bottom(12))
        .align_y(alignment::Vertical::Center);
    mouse_area(line)
        .on_press(Message::OpenVersion(index))
        .interaction(mouse::Interaction::Pointer)
        .into()
}
