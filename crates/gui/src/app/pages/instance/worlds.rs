//! Worlds tab — the saved worlds as a two-column grid, with a search field and
//! per-world actions.
//!
//! The list is read from the instance's `saves/` folder by [`super::files`].
//! `update` keeps the visible indices current, so `view` only walks them.

use aurora_leaf::Icon;
use chrono::{DateTime, Local};
use iced::widget::{Space, Text, column, container, image, row, stack, text_input};
use iced::{Background, Border, Color, ContentFit, Element, Fill, Length, Padding, Theme, alignment};
use iced_color_extension::rgb;

use super::files::WorldFile;
use super::format::format_bytes;
use super::layer;
use crate::app::components::badge::Badge;
use crate::app::components::button::Variant;
use crate::app::components::grid::grid;
use crate::app::components::icon_button::SECONDARY_BUTTON_STYLE;
use crate::app::components::input;
use crate::app::components::panel::{panel, panel_with_padding};
use crate::app::components::screenshot_tile::placeholder_art;
use crate::app::components::{IconButton, TextButton};
use crate::app::lang;
use crate::app::theme::{color, fonts, utility};

/// A card's height, per the reference.
const CARD_HEIGHT: f32 = 145.0;
/// The world thumbnail's width, per the reference.
const THUMB_WIDTH: f32 = 128.0;
/// The thumbnail's corner radius, per the reference.
const THUMB_RADIUS: f32 = 12.0;

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
            GameMode::Survival => lang::t("instance.worlds.mode.survival"),
            GameMode::Creative => lang::t("instance.worlds.mode.creative"),
            GameMode::Adventure => lang::t("instance.worlds.mode.adventure"),
            GameMode::Hardcore => lang::t("instance.worlds.mode.hardcore"),
        }
    }
}

/// The difficulty a world was set to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Difficulty {
    Peaceful,
    Easy,
    Normal,
    Hard,
}

impl Difficulty {
    pub fn label(self) -> &'static str {
        match self {
            Difficulty::Peaceful => lang::t("instance.worlds.difficulty.peaceful"),
            Difficulty::Easy => lang::t("instance.worlds.difficulty.easy"),
            Difficulty::Normal => lang::t("instance.worlds.difficulty.normal"),
            Difficulty::Hard => lang::t("instance.worlds.difficulty.hard"),
        }
    }
}

/// The "Saved …" caption: relative to `now` for the last few weeks, and a
/// calendar date after that.
pub fn format_saved(saved: DateTime<Local>, now: DateTime<Local>) -> String {
    let days = now.date_naive().signed_duration_since(saved.date_naive()).num_days();
    match days {
        ..=0 => lang::t_with("instance.saved.today", &[("time", saved.format("%H:%M").to_string().as_str())]),
        1 => lang::t("instance.saved.yesterday").to_owned(),
        2..=6 => lang::t_with("instance.saved.days_ago", &[("days", days.to_string().as_str())]),
        7..=13 => lang::t("instance.saved.last_week").to_owned(),
        14..=29 => lang::t_with("instance.saved.weeks_ago", &[("weeks", (days / 7).to_string().as_str())]),
        _ => lang::t_with("instance.saved.on", &[("date", saved.format("%b %-d, %Y").to_string().as_str())]),
    }
}

/// Messages the Worlds tab emits.
#[derive(Debug, Clone)]
pub enum Message {
    Search(String),
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
    worlds: Vec<WorldFile>,
    query: String,
    /// Indices into `worlds` that pass the search, kept current by `update`.
    visible: Vec<usize>,
    /// The moment the worlds were read, the reference for "Saved … today".
    now: DateTime<Local>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            worlds: Vec::new(),
            query: String::new(),
            visible: Vec::new(),
            now: Local::now(),
        }
    }
}

impl State {
    /// Replaces the worlds with the ones the folder holds. Save times are
    /// relative to `now`.
    pub fn set_worlds(&mut self, worlds: Vec<WorldFile>, now: DateTime<Local>) {
        self.worlds = worlds;
        self.now = now;
        self.refilter();
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::Search(query) => {
                self.query = query;
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
            .filter(|(_, world)| query.is_empty() || world.name.to_lowercase().contains(&query))
            .map(|(index, _)| index)
            .collect();
        self.visible = visible;
    }

    /// The toolbar above the two-column grid of worlds.
    pub fn view(&self) -> Element<'_, Message> {
        let cards: Vec<Element<'_, Message>> = self.visible.iter().map(|&index| self.world_card(&self.worlds[index])).collect();
        let body = if cards.is_empty() {
            let message = if self.worlds.is_empty() {
                lang::t("instance.worlds.empty")
            } else {
                lang::t("instance.worlds.no_match")
            };
            panel(Text::new(message).size(12).font(fonts::Inter::REGULAR).color(color::Text::MUTED))
        } else {
            grid(cards, 2, 14.0)
        };

        column![self.toolbar(), body].spacing(16).width(Fill).into()
    }

    fn toolbar(&self) -> Element<'_, Message> {
        let search = container(
            row![
                utility::icon(Icon::lucide().search(), 14.0, Some((color::Text::PLACEHOLDER, color::Text::PLACEHOLDER))),
                text_input(lang::t("instance.worlds.search_placeholder"), &self.query)
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
            Space::new().width(Fill),
            TextButton::labeled_icon(Icon::lucide().upload(), lang::t("instance.worlds.import"), Variant::Ghost).on_press(Message::Import),
            TextButton::labeled_icon(Icon::lucide().plus(), lang::t("instance.worlds.new"), Variant::Ghost).on_press(Message::NewWorld),
        ]
        .spacing(10)
        .align_y(alignment::Vertical::Center)
        .into()
    }

    /// One world: the thumbnail beside its name, its tags, seed and size, and the actions.
    fn world_card<'a>(&'a self, world: &'a WorldFile) -> Element<'a, Message> {
        let mut tags: Vec<Element<'a, Message>> = Vec::new();
        if let Some(mode) = world.game_mode {
            tags.push(mode_badge(mode).into());
        }
        if let Some(difficulty) = world.difficulty {
            tags.push(
                Badge::new(difficulty.label(), color::Text::MUTED, rgb!(255, 255, 255, 6%))
                    .compact()
                    .into(),
            );
        }
        let seed = world.seed.map_or_else(|| "# —".to_string(), |seed| format!("# {seed}"));
        let size = format_bytes(world.size_bytes);
        let saved = world.saved.map_or_else(
            || lang::t("instance.worlds.saved_unknown").to_owned(),
            |saved| format_saved(saved, self.now),
        );

        let body = column![
            Text::new(world.name.as_str())
                .size(14)
                .font(fonts::Inter::SEMI_BOLD)
                .color(color::Text::HEADLINE),
            row(tags).spacing(6),
            Text::new(lang::t_with(
                "instance.worlds.seed_size",
                &[("seed", seed.as_str()), ("size", size.as_str())]
            ))
            .size(11)
            .font(fonts::JetBrainsMono::REGULAR)
            .color(color::Text::PLACEHOLDER),
            Space::new().height(Fill),
            row![
                Text::new(saved).size(11).font(fonts::Inter::REGULAR).color(color::Text::MUTED),
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
                TextButton::primary(lang::t("common.play"))
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

/// The world's own `icon.png`, with the Minecraft version in the bottom-right
/// corner. A world without an icon gets a neutral placeholder.
fn thumbnail(world: &WorldFile) -> Element<'_, Message> {
    let art: Element<'_, Message> = match &world.icon {
        Some(handle) => image(handle.clone())
            .content_fit(ContentFit::Cover)
            .border_radius(THUMB_RADIUS)
            .width(Fill)
            .height(Fill)
            .into(),
        None => placeholder_art(THUMB_RADIUS),
    };

    let mut layers = stack![art];
    if let Some(version) = &world.version {
        layers = layers.push(layer(
            Badge::new(version.as_str(), color::Text::DEFAULT, rgb!(0, 0, 0, 45%)).compact().into(),
            alignment::Horizontal::Right,
            alignment::Vertical::Bottom,
            Padding::new(0.0).right(8.0).bottom(8.0),
        ));
    }

    container(layers).width(THUMB_WIDTH).height(Fill).into()
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
