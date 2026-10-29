//! Worlds tab — the saved worlds as a two-column grid, with a search field, a
//! Singleplayer/Backups filter and per-world actions.
//!
//! The list is seeded by [`super::demo`]. `update` keeps the visible indices
//! current, so `view` only walks them.

use aurora_leaf::Icon;
use chrono::{DateTime, Local};
use iced::widget::{Space, Text, column, container, responsive, row, stack, svg, text_input};
use iced::{Background, Border, Color, ContentFit, Element, Fill, Length, Padding, Theme, alignment};
use iced_color_extension::rgb;

use crate::app::components::badge::Badge;
use crate::app::components::button::Variant;
use crate::app::components::grid::grid;
use crate::app::components::icon_button::SECONDARY_BUTTON_STYLE;
use crate::app::components::input;
use crate::app::components::panel::{panel, panel_with_padding};
use crate::app::components::scene_art::screenshot_handle;
use crate::app::components::segmented::segmented;
use crate::app::components::{IconButton, TextButton};
use crate::app::pages::library::instance::{Biome, Instance};
use crate::app::theme::{color, fonts, utility};

use super::demo;
use super::format::{format_bytes, format_clock};
use super::layer;

/// A card's height, per the reference.
const CARD_HEIGHT: f32 = 145.0;
/// The scene thumbnail's width, per the reference.
const THUMB_WIDTH: f32 = 128.0;

/// The game mode a world was created in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameMode {
    Survival,
    Creative,
    Adventure,
    Hardcore,
}

impl GameMode {
    pub fn label(self) -> &'static str {
        match self {
            GameMode::Survival => "SURVIVAL",
            GameMode::Creative => "CREATIVE",
            GameMode::Adventure => "ADVENTURE",
            GameMode::Hardcore => "HARDCORE",
        }
    }
}

/// The difficulty a world was set to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Difficulty {
    Peaceful,
    Normal,
    Hard,
}

impl Difficulty {
    pub fn label(self) -> &'static str {
        match self {
            Difficulty::Peaceful => "PEACEFUL",
            Difficulty::Normal => "NORMAL",
            Difficulty::Hard => "HARD",
        }
    }
}

/// The chips above the grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WorldFilter {
    #[default]
    All,
    Singleplayer,
    Backups,
}

impl WorldFilter {
    /// The chips, in order, with their labels.
    pub const OPTIONS: [(WorldFilter, &'static str); 3] = [
        (WorldFilter::All, "All"),
        (WorldFilter::Singleplayer, "Singleplayer"),
        (WorldFilter::Backups, "Backups"),
    ];

    /// Whether `world` belongs under this chip.
    pub fn admits(self, world: &WorldEntry) -> bool {
        match self {
            WorldFilter::All => true,
            WorldFilter::Singleplayer => !world.backup,
            WorldFilter::Backups => world.backup,
        }
    }
}

/// One saved world.
#[derive(Debug, Clone, PartialEq)]
pub struct WorldEntry {
    pub name: String,
    pub game_mode: GameMode,
    pub difficulty: Difficulty,
    pub seed: i64,
    pub play_seconds: u32,
    pub size_bytes: u64,
    pub saved: DateTime<Local>,
    pub version: String,
    pub pinned: bool,
    /// Synced to the cloud — draws the cloud glyph.
    pub synced: bool,
    /// Protected from edits — draws the shield glyph when not synced.
    pub protected: bool,
    /// A backup copy rather than a live save.
    pub backup: bool,
    pub biome: Biome,
    pub art_seed: u64,
}

/// The "Saved …" caption: relative to `now` for the last few weeks, and a
/// calendar date after that.
pub fn format_saved(saved: DateTime<Local>, now: DateTime<Local>) -> String {
    let days = now.date_naive().signed_duration_since(saved.date_naive()).num_days();
    match days {
        ..=0 => format!("Saved {} today", saved.format("%H:%M")),
        1 => "Saved Yesterday".to_string(),
        2..=6 => format!("Saved {days} days ago"),
        7..=13 => "Saved Last week".to_string(),
        14..=29 => format!("Saved {} weeks ago", days / 7),
        _ => format!("Saved {}", saved.format("%b %-d, %Y")),
    }
}

/// Messages the Worlds tab emits.
#[derive(Debug, Clone)]
pub enum Message {
    Search(String),
    Filter(WorldFilter),
    /// Import — display-only until the instance manager lands.
    Import,
    /// New world — display-only until the instance manager lands.
    NewWorld,
    /// A world's Play, folder and menu buttons — display-only for now.
    Play,
    Reveal,
    More,
}

/// The Worlds tab's state.
#[derive(Debug)]
pub struct State {
    worlds: Vec<WorldEntry>,
    query: String,
    filter: WorldFilter,
    /// Indices into `worlds` that pass the search and the filter.
    visible: Vec<usize>,
    /// The moment the tab was opened, the reference for "Saved … today".
    now: DateTime<Local>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            worlds: Vec::new(),
            query: String::new(),
            filter: WorldFilter::default(),
            visible: Vec::new(),
            now: Local::now(),
        }
    }
}

impl State {
    /// The worlds an instance starts with, relative to `now`.
    pub fn for_instance(instance: &Instance, now: DateTime<Local>) -> Self {
        let mut state = Self {
            worlds: demo::worlds(instance, now),
            now,
            ..Self::default()
        };
        state.refilter();
        state
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::Search(query) => {
                self.query = query;
                self.refilter();
            }
            Message::Filter(filter) => {
                self.filter = filter;
                self.refilter();
            }
            // Display-only until the launch pipeline and instance manager land.
            Message::Import | Message::NewWorld | Message::Play | Message::Reveal | Message::More => {}
        }
    }

    fn refilter(&mut self) {
        let query = self.query.to_lowercase();
        let visible = self
            .worlds
            .iter()
            .enumerate()
            .filter(|(_, world)| self.filter.admits(world) && (query.is_empty() || world.name.to_lowercase().contains(&query)))
            .map(|(index, _)| index)
            .collect();
        self.visible = visible;
    }

    /// The toolbar above the two-column grid of worlds.
    pub fn view(&self) -> Element<'_, Message> {
        let cards: Vec<Element<'_, Message>> = self.visible.iter().map(|&index| self.world_card(&self.worlds[index])).collect();
        let body = if cards.is_empty() {
            panel(
                Text::new("No worlds match your search.")
                    .size(12)
                    .font(fonts::Inter::REGULAR)
                    .color(color::Text::MUTED),
            )
        } else {
            grid(cards, 2, 14.0)
        };

        column![self.toolbar(), body].spacing(16).width(Fill).into()
    }

    fn toolbar(&self) -> Element<'_, Message> {
        let search = container(
            row![
                utility::icon(Icon::lucide().search(), 14.0, Some((color::Text::PLACEHOLDER, color::Text::PLACEHOLDER))),
                text_input("Search worlds…", &self.query)
                    .on_input(Message::Search)
                    .style(bare_input)
                    .padding(Padding::new(0.0).top(9.0).bottom(9.0).right(12.0))
                    .size(13)
                    .width(Fill),
            ]
            .spacing(10)
            .align_y(alignment::Vertical::Center),
        )
        .width(Length::FillPortion(2))
        .height(40)
        .padding(Padding::new(0.0).left(14.0))
        .align_y(alignment::Vertical::Center)
        .style(|_| container::Style {
            background: Some(Background::Color(rgb!(255, 255, 255, 4%))),
            border: Border {
                radius: 12.0.into(),
                width: 1.0,
                color: color::Surface::LINE,
            },
            ..container::Style::default()
        });

        row![
            search,
            segmented(&WorldFilter::OPTIONS, self.filter, Message::Filter),
            Space::new().width(Fill),
            TextButton::labeled_icon(Icon::lucide().upload(), "Import", Variant::Ghost).on_press(Message::Import),
            TextButton::labeled_icon(Icon::lucide().plus(), "New world", Variant::Ghost).on_press(Message::NewWorld),
        ]
        .spacing(10)
        .align_y(alignment::Vertical::Center)
        .into()
    }

    /// One world: the thumbnail beside its name, chips, playtime and actions.
    fn world_card<'a>(&'a self, world: &'a WorldEntry) -> Element<'a, Message> {
        let body = column![
            row![
                Text::new(world.name.as_str())
                    .size(14)
                    .font(fonts::Inter::SEMI_BOLD)
                    .color(color::Text::HEADLINE),
                Space::new().width(Fill),
                status_glyph(world),
            ]
            .align_y(alignment::Vertical::Center),
            row![
                mode_badge(world.game_mode),
                Badge::new(world.difficulty.label(), color::Text::MUTED, rgb!(255, 255, 255, 6%)).compact(),
            ]
            .spacing(6),
            Text::new(format!("# {}", world.seed))
                .size(11)
                .font(fonts::JetBrainsMono::REGULAR)
                .color(color::Text::PLACEHOLDER),
            Text::new(format!(
                "play {} · size {}",
                format_clock(world.play_seconds),
                format_bytes(world.size_bytes)
            ))
            .size(11)
            .font(fonts::JetBrainsMono::REGULAR)
            .color(color::Text::PLACEHOLDER),
            Space::new().height(Fill),
            row![
                Text::new(format_saved(world.saved, self.now))
                    .size(11)
                    .font(fonts::Inter::REGULAR)
                    .color(color::Text::MUTED),
                Space::new().width(Fill),
                IconButton::new(Icon::lucide().folder())
                    .size(30)
                    .radius(9.0)
                    .icon_size(13.0)
                    .style(SECONDARY_BUTTON_STYLE)
                    .on_press(Message::Reveal),
                IconButton::new(Icon::lucide().ellipsis_vertical())
                    .size(30)
                    .radius(9.0)
                    .icon_size(13.0)
                    .style(SECONDARY_BUTTON_STYLE)
                    .on_press(Message::More),
                TextButton::primary("Play")
                    .padding(Padding::new(0.0).left(14).right(14).top(7).bottom(7))
                    .on_press(Message::Play),
            ]
            .spacing(6)
            .align_y(alignment::Vertical::Center),
        ]
        .spacing(6)
        .padding(Padding::new(0.0).top(14).bottom(12).left(16).right(14))
        .width(Fill)
        .height(Fill);

        panel_with_padding(row![thumbnail(world), body].height(CARD_HEIGHT), Padding::new(0.0))
    }
}

/// The field inside the search box. It draws no fill or border of its own, so
/// the box provides the surface.
fn bare_input(_theme: &Theme, _status: text_input::Status) -> text_input::Style {
    text_input::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border::default(),
        icon: color::Text::PLACEHOLDER,
        placeholder: color::Text::PLACEHOLDER,
        value: color::Text::DEFAULT,
        selection: input::selection_color(),
    }
}

/// The scene thumbnail, with the pin (top-left) and the version (bottom-right).
fn thumbnail(world: &WorldEntry) -> Element<'static, Message> {
    let (biome, seed) = (world.biome, world.art_seed);
    let art = responsive(move |size| {
        svg(screenshot_handle(biome, seed, size))
            .content_fit(ContentFit::Fill)
            .width(Fill)
            .height(Fill)
            .into()
    });

    let mut layers = stack![art];
    if world.pinned {
        layers = layers.push(layer(
            pin_chip(),
            alignment::Horizontal::Left,
            alignment::Vertical::Top,
            Padding::new(0.0).left(8.0).top(8.0),
        ));
    }
    layers = layers.push(layer(
        Badge::new(world.version.as_str(), color::Text::DEFAULT, rgb!(0, 0, 0, 45%))
            .compact()
            .into(),
        alignment::Horizontal::Right,
        alignment::Vertical::Bottom,
        Padding::new(0.0).right(8.0).bottom(8.0),
    ));

    container(layers).width(THUMB_WIDTH).height(Fill).into()
}

fn pin_chip() -> Element<'static, Message> {
    container(utility::icon(
        Icon::lucide().pin(),
        11.0,
        Some((color::Text::HEADLINE, color::Text::HEADLINE)),
    ))
    .padding(5)
    .style(|_| container::Style {
        background: Some(Background::Color(rgb!(0, 0, 0, 40%))),
        border: Border {
            radius: 8.0.into(),
            width: 1.0,
            color: color::Surface::LINE,
        },
        ..container::Style::default()
    })
    .into()
}

/// The cloud for synced worlds, the shield for protected ones, nothing otherwise.
fn status_glyph(world: &WorldEntry) -> Element<'static, Message> {
    if world.synced {
        utility::icon(Icon::lucide().cloud(), 13.0, Some((color::Accents::CYAN, color::Accents::CYAN)))
    } else if world.protected {
        utility::icon(Icon::lucide().shield(), 13.0, Some((color::Text::PLACEHOLDER, color::Text::PLACEHOLDER)))
    } else {
        Space::new().into()
    }
}

fn mode_badge(mode: GameMode) -> Badge {
    let (ink, fill) = match mode {
        GameMode::Survival => (color::Primary::DEFAULT, color::Primary::TINT),
        GameMode::Creative => (color::Accents::CYAN, color::Accents::CYAN_TINT),
        GameMode::Adventure => (color::Accents::VIOLET_LIGHT, color::Accents::VIOLET_TINT),
        GameMode::Hardcore => (color::Accents::AMBER, color::Accents::AMBER_TINT),
    };
    Badge::new(mode.label(), ink, fill).compact()
}

#[cfg(test)]
#[path = "../../../../tests/app/pages/instance/worlds.rs"]
mod tests;
