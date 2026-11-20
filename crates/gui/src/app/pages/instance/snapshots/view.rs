//! The Snapshots tab's rows: one per snapshot, newest first, each with its date and kind, how many mods it holds and how many of
//! them differ from the instance now, a Roll back button, and a menu. A right-click on the row opens the same menu.

use std::collections::HashMap;

use aurora_leaf::Icon;
use chrono::NaiveDateTime;
use iced::widget::{Text, column, container, row};
use iced::{Element, Fill, Padding, alignment};
use lodestone_snapshots::{Kind, Platform};

use super::{Action, Loaded, Message, State};
use crate::app::components::TextButton;
use crate::app::components::context_menu::ContextMenu;
use crate::app::components::dropdown::Dropdown;
use crate::app::components::menu::MenuItem;
use crate::app::components::panel::panel_with_padding;
use crate::app::components::scrollbar;
use crate::app::lang;
use crate::app::pages::instance::GUTTER;
use crate::app::pages::instance::mods::Row;
use crate::app::theme::{color, fonts};

/// The side of the square menu button: the same 36px icon button the other menus use. The row is taller, so the button keeps room above and below.
const MORE_WIDTH: f32 = 36.0;

/// The version each platform mod the instance holds has now, by platform and project id, for the rows to compare against.
pub fn installed_index(rows: &[Row]) -> HashMap<(Platform, String), String> {
    rows.iter()
        .filter_map(|row| {
            let record = row.source.as_ref()?;
            let platform = Platform::from_key(&record.platform)?;
            Some(((platform, record.project_id.clone()), record.version_id.clone()))
        })
        .collect()
}

/// How many of a snapshot's platform mods differ from the instance now: another version, or no record of the mod at all.
fn differ(loaded: &Loaded, installed: &HashMap<(Platform, String), String>) -> usize {
    loaded
        .snapshot
        .linked()
        .filter(|(platform, entry)| installed.get(&(*platform, entry.modid.clone())) != Some(&entry.fileid))
        .count()
}

/// What a kind of snapshot is called in its row.
fn kind_label(kind: Kind) -> &'static str {
    match kind {
        Kind::UpdateAll => lang::t("snapshots.kind.update_all"),
        Kind::Manual => lang::t("snapshots.kind.manual"),
        Kind::Rollback => lang::t("snapshots.kind.rollback"),
    }
}

/// When a snapshot was taken, as a row shows it.
fn when_label(when: NaiveDateTime) -> String {
    when.format("%-d %b %Y, %-I:%M %p").to_string()
}

/// The menu a snapshot's row shows, and the action each of its rows asks for. A separator asks for nothing. A locked instance keeps
/// no rollback, so its menu leaves out the two rows that roll back.
pub fn menu_rows(locked: bool) -> (Vec<MenuItem>, Vec<Option<Action>>) {
    let mut entries = Vec::new();
    if !locked {
        entries.push(Some((
            Action::Compare,
            lang::t("snapshots.menu.compare"),
            Icon::lucide().git_compare(),
            false,
        )));
        entries.push(Some((
            Action::RollBackAll,
            lang::t("snapshots.menu.roll_back"),
            Icon::lucide().rotate_ccw(),
            false,
        )));
    }
    entries.push(Some((
        Action::Reveal,
        lang::t("instance.files.menu.reveal"),
        Icon::lucide().folder_open(),
        false,
    )));
    entries.push(None);
    entries.push(Some((Action::Delete, lang::t("snapshots.menu.delete"), Icon::lucide().trash(), true)));
    entries
        .into_iter()
        .map(|entry| match entry {
            Some((action, label, icon, danger)) => (
                MenuItem::Action {
                    label: label.to_string(),
                    icon: Some(icon),
                    danger,
                },
                Some(action),
            ),
            None => (MenuItem::Separator, None),
        })
        .unzip()
}

/// The Snapshots tab: one row per snapshot, newest first, in a scroll of its own. A locked instance can look at its snapshots, but
/// not roll them back.
pub fn tab<'a>(state: &'a State, installed: &HashMap<(Platform, String), String>, locked: bool) -> Element<'a, Message> {
    let rows: Vec<Element<'a, Message>> = state.list().iter().map(|loaded| row_view(loaded, installed, locked)).collect();
    // The rows keep the page's gutters, as the mod list does, so the scrollbar sits in the gutter on the right.
    let padded = container(column(rows).spacing(10).width(Fill))
        .width(Fill)
        .padding(Padding::new(0.0).left(GUTTER).right(GUTTER).bottom(40.0));
    scrollbar::scroll(padded).width(Fill).height(Fill).into()
}

/// One snapshot's row: what it is and when it was taken, how many mods it holds and differ now, and its buttons.
fn row_view<'a>(loaded: &'a Loaded, installed: &HashMap<(Platform, String), String>, locked: bool) -> Element<'a, Message> {
    let name = &loaded.listed.name;
    let stem = name.stem();
    let locals = loaded.snapshot.local.len();
    let mut counts = lang::t_count("snapshots.row.mods", loaded.snapshot.linked_count());
    counts.push_str(" · ");
    counts.push_str(&lang::t_with(
        "snapshots.row.differ",
        &[("count", differ(loaded, installed).to_string().as_str())],
    ));
    if locals > 0 {
        counts.push_str(" · ");
        counts.push_str(&lang::t_count("snapshots.row.local", locals));
    }
    let title = format!("{} · {}", kind_label(name.kind), when_label(name.when));
    let text = column![
        Text::new(title).size(13).font(fonts::Inter::MEDIUM).color(color::text_default()),
        Text::new(counts).size(11).font(fonts::JetBrainsMono::REGULAR).color(color::text_muted()),
    ]
    .spacing(4)
    .width(Fill);

    let roll_back = TextButton::ghost(lang::t("snapshots.row.roll_back"));
    let roll_back = if locked {
        roll_back.disabled(true)
    } else {
        roll_back.on_press(Message::RollBack(stem.clone()))
    };
    let (items, actions) = menu_rows(locked);
    let more = {
        let (stem, actions) = (stem.clone(), actions.clone());
        Dropdown::new(Icon::lucide().ellipsis_vertical())
            .items(items.clone())
            .width(MORE_WIDTH)
            .height(MORE_WIDTH)
            .on_select(move |picked| Message::Menu {
                stem: stem.clone(),
                action: actions.get(picked).copied().flatten(),
            })
    };
    let content = row![text, roll_back, container(more).width(MORE_WIDTH)]
        .spacing(12)
        .align_y(alignment::Vertical::Center);
    let lit = panel_with_padding(content, Padding::new(14.0).left(18.0).right(12.0));
    ContextMenu::new(lit, format!("snapshot:{stem}"))
        .items(items)
        .on_select(move |picked| Message::Menu {
            stem: stem.clone(),
            action: actions.get(picked).copied().flatten(),
        })
        .into()
}
