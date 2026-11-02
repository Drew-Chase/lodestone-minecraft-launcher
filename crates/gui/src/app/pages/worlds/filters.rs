//! The Worlds page's toolbar: the filter chips, the sort menu, and the switch between the grid and the table.

use std::cmp::Ordering;

use aurora_leaf::Icon;
use iced::widget::{Space, Text, column, container, row};
use iced::{Background, Border, Element, Fill, Padding, alignment};
use iced_color_extension::rgb;

use super::{Menu, Message, State, WorldEntry};
use crate::app::components::TextButton;
use crate::app::components::panel::panel_with_padding;
use crate::app::components::popover::Popover;
use crate::app::lang;
use crate::app::pages::instance::files::GameMode;
use crate::app::theme::{color, fonts, utility};

/// The chips that narrow the list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WorldFilter {
    /// Every world.
    #[default]
    All,
    /// Only the pinned worlds.
    Pinned,
    Survival,
    Creative,
    Hardcore,
}

impl WorldFilter {
    /// Every chip, in the order the row shows them.
    pub const ALL: [WorldFilter; 5] = [
        WorldFilter::All,
        WorldFilter::Pinned,
        WorldFilter::Survival,
        WorldFilter::Creative,
        WorldFilter::Hardcore,
    ];

    fn label(self) -> &'static str {
        match self {
            WorldFilter::All => lang::t("worlds.filter.all"),
            WorldFilter::Pinned => lang::t("worlds.filter.pinned"),
            WorldFilter::Survival => lang::t("worlds.filter.survival"),
            WorldFilter::Creative => lang::t("worlds.filter.creative"),
            WorldFilter::Hardcore => lang::t("worlds.filter.hardcore"),
        }
    }

    /// Whether `entry` passes this chip. `pinned` says whether the entry's world is pinned.
    pub fn matches(self, entry: &WorldEntry, pinned: bool) -> bool {
        let mode = entry.world.game_mode;
        match self {
            WorldFilter::All => true,
            WorldFilter::Pinned => pinned,
            WorldFilter::Survival => mode == Some(GameMode::Survival),
            WorldFilter::Creative => mode == Some(GameMode::Creative),
            WorldFilter::Hardcore => mode == Some(GameMode::Hardcore),
        }
    }
}

/// The orders the list sorts by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WorldSort {
    /// The most recently played first.
    #[default]
    LastPlayed,
    /// Names from A to Z.
    Name,
    /// The largest first.
    Size,
    /// The longest played first.
    PlayTime,
}

impl WorldSort {
    /// Every order, in the order the menu lists them.
    pub const ALL: [WorldSort; 4] = [WorldSort::LastPlayed, WorldSort::Name, WorldSort::Size, WorldSort::PlayTime];

    fn label(self) -> &'static str {
        match self {
            WorldSort::LastPlayed => lang::t("worlds.sort.last_played"),
            WorldSort::Name => lang::t("worlds.sort.name"),
            WorldSort::Size => lang::t("worlds.sort.size"),
            WorldSort::PlayTime => lang::t("worlds.sort.play_time"),
        }
    }

    /// How `a` orders against `b` under this sort.
    pub fn compare(self, a: &WorldEntry, b: &WorldEntry) -> Ordering {
        match self {
            WorldSort::LastPlayed => b.world.saved.cmp(&a.world.saved),
            WorldSort::Name => a.world.name.to_lowercase().cmp(&b.world.name.to_lowercase()),
            WorldSort::Size => b.world.size_bytes.cmp(&a.world.size_bytes),
            WorldSort::PlayTime => b.world.play_time.cmp(&a.world.play_time),
        }
    }
}

/// How the list lays out its worlds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WorldView {
    /// Cards, two to four to a row.
    #[default]
    Grid,
    /// One row a world, under a header.
    Table,
}

/// The toolbar: the filter chips on the left, and on the right the sort menu and the switch between the grid and
/// the table.
pub fn view(state: &State) -> Element<'_, Message> {
    let chips: Vec<Element<'_, Message>> = WorldFilter::ALL
        .iter()
        .enumerate()
        .map(|(index, &filter)| chip(filter, state.counts[index], state.filter == filter))
        .collect();
    row![
        row(chips).spacing(4).align_y(alignment::Vertical::Center),
        Space::new().width(Fill),
        sort_menu(state),
        view_switch(state),
    ]
    .spacing(12)
    .align_y(alignment::Vertical::Center)
    .width(Fill)
    .into()
}

/// One filter chip: its name and its count, on a faint fill with a hairline when it is the active one.
fn chip(filter: WorldFilter, count: usize, active: bool) -> Element<'static, Message> {
    let (ink, weight) = if active {
        (color::Text::HEADLINE, fonts::Inter::SEMI_BOLD)
    } else {
        (color::Text::MUTED, fonts::Inter::MEDIUM)
    };
    let content = row![
        Text::new(filter.label()).size(12).font(weight).color(ink),
        Text::new(count.to_string())
            .size(11)
            .font(fonts::JetBrainsMono::REGULAR)
            .color(color::Text::PLACEHOLDER),
    ]
    .spacing(8)
    .align_y(alignment::Vertical::Center);
    container(
        TextButton::content(content)
            .padding(Padding::new(0.0).left(12.0).right(12.0).top(7.0).bottom(7.0))
            .on_press(Message::Filter(filter)),
    )
    .style(move |_| container::Style {
        background: active.then_some(Background::Color(rgb!(255, 255, 255, 10%))),
        border: Border {
            radius: 999.0.into(),
            width: if active { 1.0 } else { 0.0 },
            color: color::Surface::LINE_STRONG,
        },
        ..container::Style::default()
    })
    .into()
}

/// The sort trigger, which opens the list of orders beside it. The trigger is plain, so the popover receives its press.
fn sort_menu(state: &State) -> Element<'_, Message> {
    let open = state.menu == Some(Menu::Sort);
    let trigger = row![
        Text::new(state.sort.label().to_uppercase())
            .size(11)
            .font(fonts::JetBrainsMono::REGULAR)
            .color(color::Text::MUTED),
        utility::icon(Icon::lucide().arrow_down(), 12.0, Some((color::Text::MUTED, color::Text::MUTED))),
    ]
    .spacing(6)
    .align_y(alignment::Vertical::Center);
    let content: Element<'_, Message> = if open {
        let options: Vec<Element<'_, Message>> = WorldSort::ALL
            .iter()
            .map(|&sort| TextButton::ghost(sort.label()).on_press(Message::Sort(sort)).into())
            .collect();
        panel_with_padding(column(options).spacing(2.0).width(180.0), Padding::new(6.0))
    } else {
        Space::new().into()
    };
    Popover::new(container(trigger).padding(Padding::new(8.0)), content)
        .open(open)
        .on_toggle(Message::ToggleMenu(Menu::Sort))
        .on_dismiss(Message::DismissMenu)
        .into()
}

/// The switch between the grid and the table: two icon cells in a hairline group, the active one lifted.
fn view_switch(state: &State) -> Element<'static, Message> {
    row![
        view_cell(WorldView::Grid, Icon::lucide().layout_grid(), state.view == WorldView::Grid),
        view_cell(WorldView::Table, Icon::lucide().table(), state.view == WorldView::Table),
    ]
    .spacing(2)
    .into()
}

/// One cell of the switch.
fn view_cell(view: WorldView, glyph: &'static str, active: bool) -> Element<'static, Message> {
    let ink = if active { color::Primary::DEFAULT } else { color::Text::MUTED };
    container(
        TextButton::content(utility::icon(glyph, 14.0, Some((ink, ink))))
            .padding(Padding::new(0.0).left(10.0).right(10.0).top(7.0).bottom(7.0))
            .on_press(Message::View(view)),
    )
    .style(move |_| container::Style {
        background: active.then_some(Background::Color(color::Primary::TINT)),
        border: Border {
            radius: 8.0.into(),
            ..Border::default()
        },
        ..container::Style::default()
    })
    .into()
}
