//! Mods tab — the installed mods as one divided list, with a text filter, a
//! category filter, and a switch per mod.
//!
//! The list is seeded per instance by [`super::demo`]. `update` keeps the
//! indices of the visible rows current, so `view` only walks them.

use aurora_leaf::Icon;
use iced::widget::{Space, Text, column, container, row, text_input};
use iced::{Background, Border, Color, Element, Fill, Padding, alignment};

use crate::app::components::badge::Badge;
use crate::app::components::button::Variant;
use crate::app::components::icon_button::SECONDARY_BUTTON_STYLE;
use crate::app::components::input;
use crate::app::components::panel::{divided_panel, panel};
use crate::app::components::{IconButton, Select, TextButton, Toggle};
use crate::app::pages::library::instance::Instance;
use crate::app::theme::{color, fonts, utility};

use super::demo;

/// The categories a mod is filed under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModCategory {
    Performance,
    Graphics,
    Library,
    Hud,
    Utility,
}

impl ModCategory {
    /// Every category, in the order the filter lists them.
    pub const ALL: [ModCategory; 5] = [
        ModCategory::Performance,
        ModCategory::Graphics,
        ModCategory::Library,
        ModCategory::Hud,
        ModCategory::Utility,
    ];

    /// The uppercase tag drawn beside each mod's name.
    pub fn label(self) -> &'static str {
        match self {
            ModCategory::Performance => "PERFORMANCE",
            ModCategory::Graphics => "GRAPHICS",
            ModCategory::Library => "LIBRARY",
            ModCategory::Hud => "HUD",
            ModCategory::Utility => "UTILITY",
        }
    }

    /// The title-case name the category filter lists.
    pub fn title(self) -> &'static str {
        match self {
            ModCategory::Performance => "Performance",
            ModCategory::Graphics => "Graphics",
            ModCategory::Library => "Library",
            ModCategory::Hud => "HUD",
            ModCategory::Utility => "Utility",
        }
    }
}

/// One installed mod.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModEntry {
    pub name: String,
    pub author: String,
    pub version: String,
    pub category: ModCategory,
    pub enabled: bool,
}

/// The active filter: a case-insensitive match on name or author, and an
/// optional category.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModFilter {
    pub query: String,
    pub category: Option<ModCategory>,
}

impl ModFilter {
    /// Whether `entry` passes the filter.
    pub fn matches(&self, entry: &ModEntry) -> bool {
        let in_category = self.category.is_none_or(|category| category == entry.category);
        let in_text = self.query.is_empty() || contains_ignore_case(&entry.name, &self.query) || contains_ignore_case(&entry.author, &self.query);
        in_category && in_text
    }
}

fn contains_ignore_case(haystack: &str, needle: &str) -> bool {
    haystack.to_lowercase().contains(&needle.to_lowercase())
}

/// The Mods tab's state.
#[derive(Debug, Default)]
pub struct State {
    entries: Vec<ModEntry>,
    filter: ModFilter,
    /// Indices into `entries` that pass `filter`, kept current by `update`.
    visible: Vec<usize>,
}

/// Messages the Mods tab emits.
#[derive(Debug, Clone)]
pub enum Message {
    /// The filter text changed.
    Query(String),
    /// A category was picked; `None` lists every category.
    Category(Option<ModCategory>),
    /// A mod's switch was flipped.
    Toggle { index: usize, enabled: bool },
    /// Add Mod was pressed — display-only until the instance manager lands.
    Add,
    /// A row's menu was pressed — display-only for now.
    More,
}

impl State {
    /// The mods an instance starts with, unfiltered.
    pub fn for_instance(instance: &Instance) -> Self {
        let mut state = Self {
            entries: demo::mods(instance),
            ..Self::default()
        };
        state.refilter();
        state
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::Query(query) => {
                self.filter.query = query;
                self.refilter();
            }
            Message::Category(category) => {
                self.filter.category = category;
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
            panel(
                Text::new("No mods match these filters.")
                    .size(12)
                    .font(fonts::Inter::REGULAR)
                    .color(color::Text::MUTED),
            )
        } else {
            divided_panel(rows, Padding::new(0.0).left(16).right(16))
        };

        column![self.toolbar(), list].spacing(16).width(Fill).into()
    }

    fn toolbar(&self) -> Element<'_, Message> {
        // Option 0 clears the category filter; the rest map onto `ModCategory::ALL`.
        let options = std::iter::once("All categories").chain(ModCategory::ALL.iter().map(|category| category.title()));
        let selected = self
            .filter
            .category
            .and_then(|category| ModCategory::ALL.iter().position(|item| *item == category))
            .map(|position| position + 1);

        row![
            text_input("Filter mods…", &self.filter.query)
                .on_input(Message::Query)
                .style(input::style)
                .padding(Padding::new(0.0).left(14).right(14).top(10).bottom(10))
                .size(13)
                .width(Fill),
            Select::new(options, selected)
                .placeholder("Category")
                .on_select(|index| Message::Category(index.checked_sub(1).map(|position| ModCategory::ALL[position])))
                .width(160),
            TextButton::labeled_icon(Icon::lucide().plus(), "Add Mod", Variant::Primary).on_press(Message::Add),
        ]
        .spacing(10)
        .align_y(alignment::Vertical::Center)
        .into()
    }
}

/// One mod row: category tile, name and tag, author and version, the switch,
/// and the menu.
fn mod_row(index: usize, entry: &ModEntry) -> Element<'_, Message> {
    let (ink, fill) = category_palette(entry.category);
    let tile = container(utility::icon(Icon::lucide().package(), 15.0, Some((ink, ink))))
        .width(32)
        .height(32)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .style(move |_| tile_style(fill));

    let details = column![
        row![
            Text::new(entry.name.as_str())
                .size(13)
                .font(fonts::Inter::SEMI_BOLD)
                .color(color::Text::HEADLINE),
            Badge::new(entry.category.label(), ink, fill).compact(),
        ]
        .spacing(8)
        .align_y(alignment::Vertical::Center),
        Text::new(format!("by {} · v{}", entry.author, entry.version))
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

/// The accent ink and tinted fill of a category.
fn category_palette(category: ModCategory) -> (Color, Color) {
    match category {
        ModCategory::Performance => (color::Accents::CYAN, color::Accents::CYAN_TINT),
        ModCategory::Graphics => (color::Accents::VIOLET_LIGHT, color::Accents::VIOLET_TINT),
        ModCategory::Library | ModCategory::Utility => (color::Accents::AMBER, color::Accents::AMBER_TINT),
        ModCategory::Hud => (color::Primary::DEFAULT, color::Primary::TINT),
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
