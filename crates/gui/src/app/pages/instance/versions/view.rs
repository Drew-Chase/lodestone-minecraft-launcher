//! The Versions tab's view: the release the instance is on, then the releases of its platform as a table, a page at a time. The rows
//! draw their actions from how each release stands against the installed one.

use aurora_leaf::Icon;
use iced::widget::{Space, Text, column, container, row};
use iced::{Color, Element, Fill, Length, Padding, alignment};
use lodestone_platforms::ContentVersion;

use super::{Listing, Message, PAGE, Standing, State, standing};
use crate::app::components::chip::{chip, loader_label};
use crate::app::components::hint::hint;
use crate::app::components::icon_button::{IconButtonStyle, secondary_button_style};
use crate::app::components::{IconButton, TextButton, panel};
use crate::app::lang;
use crate::app::pages::discover::date_only;
use crate::app::theme::{color, fonts};

/// The widths of the table's columns after the name, and of its action. The game versions column fits a Minecraft version and the
/// "Changes Minecraft" chip beside it on one line.
const GAME_WIDTH: f32 = 200.0;
const LOADER_WIDTH: f32 = 120.0;
const DATE_WIDTH: f32 = 100.0;
const STATUS_WIDTH: f32 = 130.0;
const ACTION_WIDTH: f32 = 250.0;
/// How many Minecraft versions a row lists before it says how many more there are.
const GAME_VERSIONS_PER_ROW: usize = 2;

impl State {
    /// The tab. `busy` is the reason the instance cannot be updated now, when there is one: its actions are then shown but not
    /// pressable.
    pub fn view<'a>(&'a self, busy: Option<&'static str>) -> Element<'a, Message> {
        let mut page = column![self.installed_card()].spacing(18).width(Fill);
        if let Some(reason) = busy {
            page = page.push(note(reason));
        }
        page = page.push(match &self.listing {
            Listing::Idle => note(lang::t("versions.not_platform")),
            Listing::Loading => note(lang::t("versions.loading")),
            Listing::Failed(error) => self.failed(error),
            Listing::Loaded(versions) => self.releases(versions, busy),
        });
        page.into()
    }

    /// The release the instance is on, with the Minecraft version and loader it runs.
    fn installed_card<'a>(&'a self) -> Element<'a, Message> {
        let line = match (&self.installed_number, &self.installed_id) {
            (Some(number), Some(_)) => lang::t_with("versions.installed", &[("version", number.as_str())]),
            _ => lang::t("versions.unknown").to_string(),
        };
        let facts: Element<'a, Message> = row![chip(self.minecraft.as_str()), chip(loader_label(&self.loader))].spacing(6).into();
        panel::panel_with_padding(
            column![
                heading(lang::t("versions.heading")),
                Text::new(line).size(15).font(fonts::Inter::SEMI_BOLD).color(color::text_headline()),
                facts,
            ]
            .spacing(8),
            Padding::new(18.0),
        )
    }

    /// The listing's failure, with a press to try again.
    fn failed<'a>(&'a self, error: &'a str) -> Element<'a, Message> {
        column![
            note(&lang::t_with("versions.failed", &[("error", error)])),
            TextButton::ghost(lang::t("versions.retry")).on_press(Message::Retry),
        ]
        .spacing(10)
        .into()
    }

    /// The releases as a table, the first page of them, with a button for the rest.
    fn releases<'a>(&'a self, versions: &'a [ContentVersion], busy: Option<&'static str>) -> Element<'a, Message> {
        let shown = PAGE.min(versions.len());
        let mut rows: Vec<Element<'a, Message>> = versions
            .iter()
            .take(shown)
            .map(|version| self.release_row(versions, version, busy))
            .collect();
        if rows.is_empty() {
            rows.push(note(lang::t("versions.none")));
        }
        let mut table = column![table_header(), column(rows).spacing(4).width(Fill)].spacing(4).width(Fill);
        if versions.len() > shown {
            let more = lang::t_with("versions.more", &[("count", (versions.len() - shown).to_string().as_str())]);
            table = table.push(
                container(TextButton::ghost(more).on_press(Message::More))
                    .width(Fill)
                    .align_x(alignment::Horizontal::Center),
            );
        }
        panel::panel_with_padding(table, Padding::new(18.0))
    }

    /// One release: its name and number, its Minecraft versions and loaders, its date, how it stands, and what its row does.
    fn release_row<'a>(&'a self, versions: &[ContentVersion], version: &'a ContentVersion, busy: Option<&'static str>) -> Element<'a, Message> {
        let standing = standing(versions, self.installed_id.as_deref(), &version.id);
        let enabled = busy.is_none() && self.preparing().is_none();
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
        if !version.minecraft_versions.is_empty() && !version.minecraft_versions.contains(&self.minecraft) {
            games.push(chip(lang::t("versions.changes_game")));
        }
        let mut loaders: Vec<Element<'a, Message>> = version.loaders.iter().map(|loader| chip(loader_label(loader))).collect();
        if !version.loaders.is_empty() && !version.loaders.contains(&self.loader) {
            loaders.push(chip(lang::t("versions.changes_loader")));
        }
        let status = if self.preparing() == Some(version.id.as_str()) {
            lang::t("versions.preparing")
        } else {
            standing_label(standing)
        };
        row![
            column![
                Text::new(name).size(14).font(fonts::Inter::SEMI_BOLD).color(color::text_headline()),
                Text::new(version.version_number.as_str())
                    .size(11)
                    .font(fonts::JetBrainsMono::REGULAR)
                    .color(color::text_placeholder()),
            ]
            .spacing(2)
            .width(Fill),
            row(games).spacing(4).width(Length::Fixed(GAME_WIDTH)),
            row(loaders).spacing(4).width(Length::Fixed(LOADER_WIDTH)),
            Text::new(date_only(version.date.as_deref()))
                .size(12)
                .font(fonts::JetBrainsMono::REGULAR)
                .color(color::text_muted())
                .width(Length::Fixed(DATE_WIDTH)),
            container(chip(status)).width(Length::Fixed(STATUS_WIDTH)),
            container(actions(&version.id, standing, enabled))
                .width(Length::Fixed(ACTION_WIDTH))
                .align_x(alignment::Horizontal::Right),
        ]
        .spacing(12)
        .padding(Padding::new(0.0).left(16).right(16).top(12).bottom(12))
        .align_y(alignment::Vertical::Center)
        .into()
    }
}

/// The actions a release's row offers, each an icon with a tooltip: the installed release is reinstalled (cyan), a newer one
/// updated to (green), an older one switched to (amber), and a release on an unknown installed one is either recorded as the one
/// the instance is on (neutral), or installed over it (green).
fn actions<'a>(id: &str, standing: Standing, enabled: bool) -> Element<'a, Message> {
    let move_to =
        |markup: &'static str, label: &'static str, style: IconButtonStyle| icon_button(markup, label, style, Message::Move(id.to_string()), enabled);
    match standing {
        Standing::Installed => move_to(
            Icon::lucide().refresh_cw(),
            lang::t("versions.reinstall"),
            tinted(color::cyan(), color::cyan_tint()),
        ),
        Standing::Newer => move_to(
            Icon::lucide().arrow_up(),
            lang::t("versions.update"),
            tinted(color::primary_on(), color::primary()),
        ),
        Standing::Older | Standing::Other => move_to(
            Icon::lucide().repeat(),
            lang::t("versions.switch"),
            tinted(color::amber(), color::amber_tint()),
        ),
        Standing::Unknown => row![
            icon_button(
                Icon::lucide().check(),
                lang::t("versions.mark"),
                secondary_button_style(),
                Message::Record(id.to_string()),
                enabled,
            ),
            move_to(
                Icon::lucide().download(),
                lang::t("versions.install"),
                tinted(color::primary_on(), color::primary()),
            ),
        ]
        .spacing(6)
        .into(),
    }
}

/// An icon button with a tooltip naming it, that presses `message` only while `enabled`.
fn icon_button<'a>(markup: &'static str, label: &'static str, style: IconButtonStyle, message: Message, enabled: bool) -> Element<'a, Message> {
    let button = IconButton::new(markup).size(32.0).radius(10.0).icon_size(14.0).style(style);
    let button = if enabled { button.on_press(message) } else { button.disabled(true) };
    hint(button, label)
}

/// A flat button whose icon and fill are `foreground` and `background`, so each action reads as its own colour.
fn tinted(foreground: Color, background: Color) -> IconButtonStyle {
    IconButtonStyle {
        foreground,
        foreground_hover: foreground,
        background,
        border: Color::TRANSPARENT,
        background_hover: background,
        border_hover: Color::TRANSPARENT,
        foreground_disabled: color::text_disabled(),
        background_disabled: Color::TRANSPARENT,
        foreground_pressed: foreground,
        background_pressed: background,
    }
}

/// The table's heading row, labelled with the keys of its columns.
fn table_header<'a>() -> Element<'a, Message> {
    let label = |text: &'static str, width: Length| {
        Text::new(lang::t(text))
            .size(10)
            .font(fonts::JetBrainsMono::SEMI_BOLD)
            .color(color::text_placeholder())
            .width(width)
    };
    row![
        label("resource.table.version", Length::Fill),
        label("resource.table.game_version", Length::Fixed(GAME_WIDTH)),
        label("versions.column.loader", Length::Fixed(LOADER_WIDTH)),
        label("resource.table.published", Length::Fixed(DATE_WIDTH)),
        label("versions.column.status", Length::Fixed(STATUS_WIDTH)),
        Space::new().width(Length::Fixed(ACTION_WIDTH)),
    ]
    .spacing(12)
    .padding(Padding::new(0.0).left(16).right(16).top(8).bottom(8))
    .align_y(alignment::Vertical::Center)
    .into()
}

/// The label a standing's status chip shows.
fn standing_label(standing: Standing) -> &'static str {
    match standing {
        Standing::Installed => lang::t("versions.standing.installed"),
        Standing::Newer => lang::t("versions.standing.newer"),
        Standing::Older => lang::t("versions.standing.older"),
        Standing::Other => lang::t("versions.standing.other"),
        Standing::Unknown => lang::t("versions.standing.unknown"),
    }
}

/// A section's heading: a small label above its content.
fn heading<'a>(label: &'static str) -> Element<'a, Message> {
    Text::new(label)
        .size(10)
        .font(fonts::JetBrainsMono::SEMI_BOLD)
        .color(color::text_placeholder())
        .into()
}

/// A line of muted text, for the notes the tab shows in place of a list.
fn note<'a>(text: &str) -> Element<'a, Message> {
    Text::new(text.to_string())
        .size(13)
        .font(fonts::Inter::REGULAR)
        .color(color::text_muted())
        .into()
}
