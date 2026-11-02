//! The Worlds page's list: the cards or the table, windowed against the page's scroll the way the Library's list is.
//! It also holds the notes shown while the saves are read, when there are none, and when a search finds none.

use std::rc::Rc;

use iced::widget::{Space, Text, column, container, row};
use iced::{Element, Fill, Length, Size, alignment};

use super::filters::WorldView;
use super::format::format_ago;
use super::{Message, State, WorldEntry};
use crate::app::components::page::content_width;
use crate::app::components::world_card::{CARD_HEIGHT, TABLE_ROW_HEIGHT, WorldLabels, WorldMessages, table_header, table_row, world_card};
use crate::app::lang;
use crate::app::pages::instance::world_menu::Action;
use crate::app::theme::{color, fonts};
use crate::app::world_actions::WorldMessage;

/// The gap between cards, and between their rows.
const GAP: f32 = 16.0;
/// The least width a card takes before the grid wraps to fewer columns, and the most columns it shows.
const CARD_MIN_WIDTH: f32 = 340.0;
const MAX_COLUMNS: usize = 4;
/// Where the list starts below the top of the scrolling content: the toolbar and the gap under it. It is an
/// estimate, and the overscan absorbs the drift.
const LIST_TOP: f32 = 52.0;
/// The height of a note that stands in for the list.
const NOTE_HEIGHT: f32 = 160.0;

/// The list for the page's state at `viewport`.
pub fn view(state: &State, viewport: Size) -> Element<'_, Message> {
    if !state.loaded {
        return note(lang::t("worlds.loading"));
    }
    if state.visible.is_empty() {
        let text = if state.entries.is_empty() {
            lang::t("worlds.empty")
        } else {
            lang::t("worlds.no_match")
        };
        return note(text);
    }
    match state.view {
        WorldView::Grid => grid(state, viewport),
        WorldView::Table => table(state, viewport),
    }
}

/// The rows the list builds: how many cells share a row, how many rows the list has, and the rows around the scroll.
struct Window {
    columns: usize,
    total: usize,
    first: usize,
    end: usize,
}

/// The rows to build for `state` at `viewport`, when a row is `pitch` tall and holds `columns` cells.
fn window(state: &State, viewport: Size, columns: usize, pitch: f32) -> Window {
    let total = state.visible.len().div_ceil(columns);
    // One row above the estimate, and the viewport plus two rows below.
    let first = ((((state.scroll_y - LIST_TOP) / pitch).floor() as isize - 1).clamp(0, total as isize - 1)) as usize;
    let viewport_rows = (viewport.height / pitch).ceil() as usize;
    let end = (first + viewport_rows + 3).min(total);
    Window { columns, total, first, end }
}

/// How many cards share a row in a content area `width` wide: as many as the minimum card width allows, from one to
/// [`MAX_COLUMNS`].
fn grid_columns(width: f32) -> usize {
    ((width / CARD_MIN_WIDTH) as usize).clamp(1, MAX_COLUMNS)
}

/// The cards, in as many columns as the width holds, with the rows off-screen left out.
fn grid(state: &State, viewport: Size) -> Element<'_, Message> {
    let columns = grid_columns(content_width(viewport.width));
    let pitch = CARD_HEIGHT + GAP;
    let window = window(state, viewport, columns, pitch);
    let mut rows: Vec<Element<'_, Message>> = Vec::with_capacity(window.end - window.first + 2);
    if window.first > 0 {
        rows.push(spacer(window.first as f32 * pitch));
    }
    for row_index in window.first..window.end {
        let start = row_index * window.columns;
        let slice = &state.visible[start..(start + window.columns).min(state.visible.len())];
        let mut cards: Vec<Element<'_, Message>> = slice.iter().map(|&index| card(state, &state.entries[index])).collect();
        // A short last row keeps the width of the rows above it.
        cards.resize_with(window.columns, || Space::new().width(Fill).height(CARD_HEIGHT).into());
        rows.push(row(cards).spacing(GAP).width(Fill).into());
    }
    let below = (window.total - window.end) as f32 * pitch;
    if below > 0.0 {
        rows.push(spacer(below));
    }
    column(rows).spacing(GAP).width(Fill).into()
}

/// The table: its header, then the rows in view. A row's stripe follows its place in the whole list, so it holds
/// while the rows scroll.
fn table(state: &State, viewport: Size) -> Element<'_, Message> {
    let window = window(state, viewport, 1, TABLE_ROW_HEIGHT);
    let mut rows: Vec<Element<'_, Message>> = Vec::with_capacity(window.end - window.first + 2);
    if window.first > 0 {
        rows.push(Space::new().width(Fill).height(window.first as f32 * TABLE_ROW_HEIGHT).into());
    }
    for index in window.first..window.end {
        let entry = &state.entries[state.visible[index]];
        let last_played = entry
            .world
            .saved
            .map_or_else(|| "—".to_owned(), |saved| format_ago(saved, state.now.unwrap_or(saved)));
        rows.push(table_row(index, &entry.world, world_labels(state, entry), last_played, messages(entry)));
    }
    let below = (window.total - window.end) as f32 * TABLE_ROW_HEIGHT;
    if below > 0.0 {
        rows.push(Space::new().width(Fill).height(below).into());
    }
    column![table_header(), column(rows).width(Fill)].width(Fill).into()
}

/// One world's card, with whether the pointer is over it.
fn card(state: &State, entry: &WorldEntry) -> Element<'static, Message> {
    let hovered = state.hovered.as_deref() == Some(entry.world.path.as_path());
    world_card(&entry.world, world_labels(state, entry), hovered, messages(entry))
}

/// What a card or row shows beside its save: the instance and version line, and whether the world is pinned.
fn world_labels(state: &State, entry: &WorldEntry) -> WorldLabels {
    WorldLabels {
        subtitle: entry.subtitle(),
        pinned: state.pins.contains(&entry.instance_id, &entry.world.folder()),
    }
}

/// The messages one world's card or row sends. Each names the world's instance and save.
fn messages(entry: &WorldEntry) -> WorldMessages<Message> {
    let instance = entry.instance_id.clone();
    let path = entry.world.path.clone();
    let menu_instance = instance.clone();
    let menu_path = path.clone();
    WorldMessages {
        play: Message::World(WorldMessage::Menu {
            instance,
            path: path.clone(),
            action: Some(Action::Play),
        }),
        menu: Rc::new(move |action| {
            Message::World(WorldMessage::Menu {
                instance: menu_instance.clone(),
                path: menu_path.clone(),
                action,
            })
        }),
        enter: Message::Hovered(Some(path.clone())),
        exit: Message::Hovered(None),
    }
}

/// An empty stretch that stands in for the rows scrolled past. The column's spacing adds a gap after it, so the
/// spacer is shorter by one gap.
fn spacer(height: f32) -> Element<'static, Message> {
    Space::new().width(Fill).height((height - GAP).max(0.0)).into()
}

/// A line of text in place of the list: the loading and the empty states.
fn note(text: &'static str) -> Element<'static, Message> {
    container(Text::new(text).size(13).font(fonts::Inter::REGULAR).color(color::Text::MUTED))
        .width(Fill)
        .height(Length::Fixed(NOTE_HEIGHT))
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .into()
}

#[cfg(test)]
#[path = "../../../../tests/app/pages/worlds/list.rs"]
mod tests;
