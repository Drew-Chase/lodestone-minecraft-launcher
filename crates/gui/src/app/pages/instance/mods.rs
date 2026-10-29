//! Mods tab — the mod jars in the instance's `mods/` folder as one divided
//! list, with a text filter and a switch per mod.
//!
//! The list is read from disk by [`super::files`]. `update` keeps the indices
//! of the visible rows current, so `view` only walks them.

use aurora_leaf::Icon;
use iced::widget::{Space, Text, column, container, row, text_input};
use iced::{Background, Border, Color, Element, Fill, Padding, alignment};
use iced_color_extension::rgb;

use super::files::ModFile;
use crate::app::components::badge::Badge;
use crate::app::components::button::Variant;
use crate::app::components::icon_button::SECONDARY_BUTTON_STYLE;
use crate::app::components::input;
use crate::app::components::panel::{divided_panel, panel};
use crate::app::components::{IconButton, TextButton, Toggle};
use crate::app::theme::{color, fonts, utility};

/// The active filter: a case-insensitive match on name or author.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModFilter {
    pub query: String,
}

impl ModFilter {
    /// Whether `entry` passes the filter.
    pub fn matches(&self, entry: &ModFile) -> bool {
        self.query.is_empty()
            || contains_ignore_case(&entry.name, &self.query)
            || entry.author.as_deref().is_some_and(|author| contains_ignore_case(author, &self.query))
    }
}

fn contains_ignore_case(haystack: &str, needle: &str) -> bool {
    haystack.to_lowercase().contains(&needle.to_lowercase())
}

/// The Mods tab's state.
#[derive(Debug, Default)]
pub struct State {
    entries: Vec<ModFile>,
    filter: ModFilter,
    /// Indices into `entries` that pass `filter`, kept current by `update`.
    visible: Vec<usize>,
}

/// Messages the Mods tab emits.
#[derive(Debug, Clone)]
pub enum Message {
    /// The filter text changed.
    Query(String),
    /// A mod's switch was flipped.
    Toggle { index: usize, enabled: bool },
    /// Add Mod was pressed — display-only until the instance manager lands.
    Add,
    /// A row's menu was pressed — display-only for now.
    More,
}

impl State {
    /// Replaces the mods with the ones the folder holds, unfiltered.
    pub fn set_mods(&mut self, mods: Vec<ModFile>) {
        self.entries = mods;
        self.refilter();
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::Query(query) => {
                self.filter.query = query;
                self.refilter();
            }
            Message::Toggle { index, enabled } => {
                if let Some(entry) = self.entries.get_mut(index) {
                    entry.enabled = enabled;
                }
            }
            // Display-only until the instance manager lands.
            Message::Add | Message::More => {}
        }
    }

    fn refilter(&mut self) {
        let visible = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| self.filter.matches(entry))
            .map(|(index, _)| index)
            .collect();
        self.visible = visible;
    }

    /// The filter bar above the list of mods.
    pub fn view(&self) -> Element<'_, Message> {
        let rows: Vec<Element<'_, Message>> = self.visible.iter().map(|&index| mod_row(index, &self.entries[index])).collect();
        let list = if rows.is_empty() {
            let message = if self.entries.is_empty() {
                "No mods in this instance yet."
            } else {
                "No mods match this filter."
            };
            panel(Text::new(message).size(12).font(fonts::Inter::REGULAR).color(color::Text::MUTED))
        } else {
            divided_panel(rows, Padding::new(0.0).left(16).right(16))
        };

        column![self.toolbar(), list].spacing(16).width(Fill).into()
    }

    fn toolbar(&self) -> Element<'_, Message> {
        row![
            text_input("Filter mods…", &self.filter.query)
                .on_input(Message::Query)
                .style(input::style)
                .padding(Padding::new(0.0).left(14).right(14).top(10).bottom(10))
                .size(13)
                .width(Fill),
            TextButton::labeled_icon(Icon::lucide().plus(), "Add Mod", Variant::Primary).on_press(Message::Add),
        ]
        .spacing(10)
        .align_y(alignment::Vertical::Center)
        .into()
    }
}

/// One mod row: a tile, the name, the author and version (or the file name
/// when the jar declares neither), the switch, and the menu.
fn mod_row(index: usize, entry: &ModFile) -> Element<'_, Message> {
    let (ink, fill) = if entry.enabled {
        (color::Primary::DEFAULT, color::Primary::TINT)
    } else {
        (color::Text::MUTED, rgb!(255, 255, 255, 6%))
    };
    let tile = container(utility::icon(Icon::lucide().package(), 15.0, Some((ink, ink))))
        .width(32)
        .height(32)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .style(move |_| tile_style(fill));

    let mut title = row![
        Text::new(entry.name.as_str())
            .size(13)
            .font(fonts::Inter::SEMI_BOLD)
            .color(color::Text::HEADLINE),
    ]
    .spacing(8)
    .align_y(alignment::Vertical::Center);
    if !entry.enabled {
        title = title.push(Badge::new("DISABLED", color::Text::MUTED, rgb!(255, 255, 255, 6%)).compact());
    }

    let details = column![
        title,
        Text::new(subtitle(entry))
            .size(11)
            .font(fonts::JetBrainsMono::REGULAR)
            .color(color::Text::PLACEHOLDER),
    ]
    .spacing(4);

    row![
        tile,
        details,
        Space::new().width(Fill),
        Toggle::new(entry.enabled).on_toggle_with(move |enabled| Message::Toggle { index, enabled }),
        IconButton::new(Icon::lucide().ellipsis_vertical())
            .size(32)
            .radius(10.0)
            .icon_size(14.0)
            .style(SECONDARY_BUTTON_STYLE)
            .on_press(Message::More),
    ]
    .spacing(14)
    .align_y(alignment::Vertical::Center)
    .padding(Padding::new(0.0).top(12).bottom(12))
    .into()
}

/// The line under a mod's name.
fn subtitle(entry: &ModFile) -> String {
    match (&entry.author, &entry.version) {
        (Some(author), Some(version)) => format!("by {author} · v{version}"),
        (Some(author), None) => format!("by {author}"),
        (None, Some(version)) => format!("v{version}"),
        (None, None) => entry.file_name.clone(),
    }
}

fn tile_style(fill: Color) -> container::Style {
    container::Style {
        background: Some(Background::Color(fill)),
        border: Border {
            radius: 8.0.into(),
            width: 1.0,
            color: color::Surface::LINE,
        },
        ..container::Style::default()
    }
}

#[cfg(test)]
#[path = "../../../../tests/app/pages/instance/mods.rs"]
mod tests;
