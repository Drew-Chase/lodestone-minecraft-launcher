//! The Mods tab's list: one row per mod on screen, each a tile, the name and the author and version, the switch and the
//! menu. Only the rows the scroll reveals are built. Spacers hold the height of the rest, so the scrollbar reflects the
//! whole list and a folder of thousands of mods costs no more to draw than a screenful.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use aurora_leaf::Icon;
use iced::widget::{Space, Text, column, container, image, keyed_column, mouse_area, row};
use iced::{Background, Border, Color, Element, Fill, Length, Padding, alignment, mouse};
use iced_color_extension::rgb;

use super::menu::{self, Menu};
use super::{Message, Row, State};
use crate::app::components::Toggle;
use crate::app::components::badge::Badge;
use crate::app::components::context_menu::ContextMenu;
use crate::app::components::dropdown::Dropdown;
use crate::app::components::panel::{hairline, panel_with_padding};
use crate::app::lang;
use crate::app::perf;
use crate::app::theme::{color, fonts, utility};

/// The id of the scrollable the list sits in, so a filter change can scroll it back to the top.
pub const SCROLL_ID: &str = "instance-mods-list";
/// A row's height: the 36px menu trigger and its padding.
pub const ROW_HEIGHT: f32 = 60.0;
/// A row and the hairline under it.
pub const PITCH: f32 = ROW_HEIGHT + 1.0;
/// The rows built past each edge of the window, so a scroll never reveals a gap before the next frame.
const OVERSCAN: usize = 8;

/// The rows the list builds: the first one, and the one after the last. `total` is how many rows the list has.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Window {
    pub first: usize,
    pub end: usize,
    pub total: usize,
}

/// The rows to build when the list is scrolled to `scroll_y` in a viewport `height` tall, with the overscan on each
/// side. A scroll past the end, left by a filter that shrank the list, builds the last rows.
pub fn window(scroll_y: f32, height: f32, total: usize) -> Window {
    let top = (scroll_y.max(0.0) / PITCH).floor() as usize;
    let shown = (height.max(0.0) / PITCH).ceil() as usize;
    let first = top.saturating_sub(OVERSCAN).min(total.saturating_sub(1));
    let end = (top + shown + OVERSCAN).min(total).max(first);
    Window { first, end, total }
}

/// The bordered list: a spacer for the rows above the window, the rows in it, and a spacer for those below. Each row is
/// keyed by its mod, so the state its menu and switch keep follows the mod as the window scrolls. Keyed by position, a
/// menu opened on one mod would reopen on whichever mod had moved into its place.
pub fn view(state: &State, height: f32) -> Element<'_, Message> {
    let _scope = perf::scope("view.mods.list");
    let window = window(state.scroll_y, height, state.visible.len());
    perf::count("view.mods.rows", (window.end - window.first) as u64);
    let mut children: Vec<(u64, Element<'_, Message>)> = Vec::with_capacity(window.end - window.first + 2);
    children.push((TOP_SPACER, Space::new().height(window.first as f32 * PITCH).into()));
    for &index in &state.visible[window.first..window.end] {
        children.push((row_key(&state.rows[index].file.file_name), mod_row(state, index)));
    }
    children.push((BOTTOM_SPACER, Space::new().height((window.total - window.end) as f32 * PITCH).into()));
    panel_with_padding(keyed_column(children).width(Fill), Padding::new(0.0).left(16).right(16))
}

/// The keys of the spacers, which no mod's key can equal in practice.
const TOP_SPACER: u64 = u64::MAX;
const BOTTOM_SPACER: u64 = u64::MAX - 1;

/// The key of the row of the mod `file_name`. A hash of the name is stable for as long as the mod keeps its file name.
fn row_key(file_name: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    file_name.hash(&mut hasher);
    hasher.finish()
}

/// One mod's row: its tile, name and subtitle, the switch, and the menu. A press on the row opens the mod, and a
/// right-press opens the same menu as the ⋯ button. The row's hairline stays with it, so a windowed row keeps its
/// divider.
fn mod_row(state: &State, index: usize) -> Element<'_, Message> {
    let entry = &state.rows[index];
    let file = &entry.file;
    let Menu { items, actions } = menu::menu(entry, state.busy.contains(&file.file_name));
    let busy = state.busy.contains(&file.file_name);
    let (ink, fill) = if file.enabled {
        (color::primary(), color::primary_tint())
    } else {
        (color::text_muted(), rgb!(255, 255, 255, 6%))
    };
    // The jar's own icon fills the tile when the jar declares one. Otherwise the tile holds the package glyph.
    let tile: Element<'_, Message> = match &file.icon {
        Some(icon) => image(icon.clone()).width(32).height(32).into(),
        None => container(utility::icon(Icon::lucide().package(), 15.0, Some((ink, ink))))
            .width(32)
            .height(32)
            .align_x(alignment::Horizontal::Center)
            .align_y(alignment::Vertical::Center)
            .style(move |_| tile_style(fill))
            .into(),
    };

    let mut title = row![
        Text::new(file.name.as_str())
            .size(13)
            .font(fonts::Inter::SEMI_BOLD)
            .color(color::text_headline()),
    ]
    .spacing(8)
    .align_y(alignment::Vertical::Center);
    if !file.enabled {
        title = title.push(Badge::new(lang::t("instance.mods.disabled"), color::text_muted(), rgb!(255, 255, 255, 6%)).compact());
    }

    let details = column![
        title,
        Text::new(subtitle(entry))
            .size(11)
            .font(fonts::JetBrainsMono::REGULAR)
            .color(color::text_placeholder()),
    ]
    .spacing(4);

    let switch = Toggle::new(file.enabled).on_toggle_with(move |enabled| Message::Toggle { index, enabled });
    let more_name = file.file_name.clone();
    let more_actions = actions.clone();
    let more = Dropdown::new(Icon::lucide().ellipsis_vertical())
        .items(items.clone())
        .disabled(busy)
        .on_select(move |selected| Message::Menu {
            file_name: more_name.clone(),
            action: more_actions.get(selected).copied().flatten(),
        });

    let body = row![tile, details, Space::new().width(Fill), switch, more]
        .spacing(14)
        .align_y(alignment::Vertical::Center)
        .height(Length::Fixed(ROW_HEIGHT));
    let press = mouse_area(body)
        .interaction(mouse::Interaction::Pointer)
        .on_press(Message::Open(file.file_name.clone()));

    let key = file.file_name.clone();
    let menu_name = file.file_name.clone();
    let context = ContextMenu::new(press, key).items(items).on_select(move |selected| Message::Menu {
        file_name: menu_name.clone(),
        action: actions.get(selected).copied().flatten(),
    });
    column![context, hairline()].width(Fill).into()
}

/// The line under a mod's name: its author and version, or its version alone, or the file name when the jar declares
/// neither.
fn subtitle(entry: &Row) -> String {
    let file = &entry.file;
    match (&file.author, &file.version) {
        (Some(author), Some(version)) => lang::t_with(
            "instance.mods.by_author_version",
            &[("author", author.as_str()), ("version", version.as_str())],
        ),
        (Some(author), None) => lang::t_with("instance.mods.by_author", &[("author", author.as_str())]),
        (None, Some(version)) => lang::t_with("instance.mods.version", &[("version", version.as_str())]),
        (None, None) => file.file_name.clone(),
    }
}

fn tile_style(fill: Color) -> container::Style {
    container::Style {
        background: Some(Background::Color(fill)),
        border: Border {
            radius: 8.0.into(),
            width: 1.0,
            color: color::surface_line(),
        },
        ..container::Style::default()
    }
}

#[cfg(test)]
#[path = "../../../../../tests/app/pages/instance/mods/list.rs"]
mod tests;
