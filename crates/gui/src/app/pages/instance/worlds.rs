//! Worlds tab — the saved worlds as a two-column grid of cards, with a search field and the actions the Worlds
//! pages share.
//!
//! The list is read from the instance's `saves/` folder by [`super::files`]. `update` keeps the visible indices
//! current, so `view` only walks them. The actions that touch the disk or the game are the root's: this tab emits
//! them and the root carries them out for the instance the page shows.

use std::path::{Path, PathBuf};
use std::rc::Rc;

use aurora_leaf::Icon;
use chrono::{DateTime, Local};
use iced::widget::{Space, Text, column, container, row, text_input};
use iced::{Background, Border, Color, Element, Fill, Length, Padding, Theme, alignment};
use iced_color_extension::rgb;

use super::files::WorldFile;
use super::world_menu::Action;
use crate::app::components::TextButton;
use crate::app::components::button::Variant;
use crate::app::components::grid::grid;
use crate::app::components::input;
use crate::app::components::panel::panel;
use crate::app::components::world_card::{CARD_MAX_WIDTH, WorldLabels, WorldMessages, world_card};
use crate::app::lang;
use crate::app::theme::{color, fonts, utility};
use crate::app::world_pins::WorldPins;

/// The "Saved …" caption: relative to `now` for the last few weeks, and a calendar date after that. The hero shows
/// it for the newest save.
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

/// Messages the Worlds tab emits and handles.
#[derive(Debug, Clone)]
pub enum Message {
    Search(String),
    /// Imports a world from a zip. The root asks for the file and carries the import out.
    Import,
    /// Starts the instance, so the game creates a new world on its first load.
    NewWorld,
    /// A world's button or menu row was picked, for the save at `path`. A separator sends `None`. The root carries
    /// the action out for the instance the page shows.
    Menu {
        path: PathBuf,
        action: Option<Action>,
    },
    /// The pointer entered (`Some`) or left (`None`) the card of the save at this path.
    Hovered(Option<PathBuf>),
    /// The saves were read again. Stale when `generation` has moved on.
    Rescanned {
        generation: u64,
        worlds: Vec<WorldFile>,
    },
}

/// The Worlds tab's state.
#[derive(Debug, Default)]
pub struct State {
    worlds: Vec<WorldFile>,
    query: String,
    /// Indices into `worlds` that pass the search, kept current by `update`.
    visible: Vec<usize>,
    /// The save whose card the pointer is over, if any.
    hovered: Option<PathBuf>,
}

impl State {
    /// Replaces the worlds with the ones the folder holds.
    pub fn set_worlds(&mut self, worlds: Vec<WorldFile>) {
        self.worlds = worlds;
        self.refilter();
    }

    /// Drops the save in `path` from the list, as it went to the trash.
    pub fn remove(&mut self, path: &Path) {
        self.worlds.retain(|world| world.path.as_path() != path);
        self.refilter();
    }

    pub fn update(&mut self, message: Message) {
        // Any message but a hover ends the hover: the card under the pointer may have moved or gone, and then its
        // exit would never arrive.
        if !matches!(message, Message::Hovered(_)) {
            self.hovered = None;
        }
        match message {
            Message::Search(query) => {
                self.query = query;
                self.refilter();
            }
            Message::Hovered(path) => self.hovered = path,
            // The root carries these out, and the page reads its saves again on `Rescanned`, which the page
            // applies before this point.
            Message::Import | Message::NewWorld | Message::Menu { .. } | Message::Rescanned { .. } => {}
        }
    }

    fn refilter(&mut self) {
        let query = self.query.to_lowercase();
        self.visible = self
            .worlds
            .iter()
            .enumerate()
            .filter(|(_, world)| world.matches(&query))
            .map(|(index, _)| index)
            .collect();
    }

    /// The toolbar above the two-column grid of the instance's worlds. `instance` is the id of the instance the
    /// page shows, which the pins name the worlds by.
    pub fn view(&self, instance: &str, pins: &WorldPins) -> Element<'_, Message> {
        let cards: Vec<Element<'_, Message>> = self
            .visible
            .iter()
            .map(|&index| {
                let world = &self.worlds[index];
                let labels = WorldLabels {
                    subtitle: world.version.clone().unwrap_or_default(),
                    pinned: pins.contains(instance, &world.folder()),
                };
                let hovered = self.hovered.as_deref() == Some(world.path.as_path());
                // A card stops at the reference width, so one card on a wide page does not stretch across it.
                container(world_card(world, labels, hovered, messages(&world.path)))
                    .max_width(CARD_MAX_WIDTH)
                    .into()
            })
            .collect();
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
}

/// The messages one world's card sends: its Load button, its menu rows, and its hover.
fn messages(path: &Path) -> WorldMessages<Message> {
    let menu_path = path.to_path_buf();
    WorldMessages {
        play: Message::Menu {
            path: path.to_path_buf(),
            action: Some(Action::Play),
        },
        menu: Rc::new(move |action| Message::Menu {
            path: menu_path.clone(),
            action,
        }),
        enter: Message::Hovered(Some(path.to_path_buf())),
        exit: Message::Hovered(None),
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

#[cfg(test)]
#[path = "../../../../tests/app/pages/instance/worlds.rs"]
mod tests;
