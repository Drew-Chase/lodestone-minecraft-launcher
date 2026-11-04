//! The mod menu: the rows a mod's ⋯ button and a right-click on its row show, and the action each row asks for. The
//! root carries the actions out. A mod with no recorded source has no switch or dependencies, since those need a
//! project to read; an update row shows only when the platform has a newer release.

use aurora_leaf::Icon;

use super::Row;
use crate::app::components::menu::MenuItem;
use crate::app::lang;

/// What a menu row asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Installs the newer release the platform has, in place of the mod's current one.
    Update,
    /// Lists the releases that fit the instance, and installs the one picked.
    Switch,
    /// Lists the dependencies of the mod's current release.
    Dependencies,
    /// Searches the platforms for the project the mod is, and records it.
    Associate,
    /// Shows the jar in the file browser, with the jar selected.
    Reveal,
    /// Moves the jar to the trash, once the user has said so.
    Delete,
}

/// A menu as the widgets show it: its rows, and the action each row asks for. A separator asks for nothing, so its
/// entry in `actions` is `None`.
pub struct Menu {
    pub items: Vec<MenuItem>,
    pub actions: Vec<Option<Action>>,
}

/// The menu for `row`. A mod with an action in flight has none, since its file is being replaced.
pub fn menu(row: &Row, busy: bool) -> Menu {
    if busy {
        return Menu {
            items: Vec::new(),
            actions: Vec::new(),
        };
    }
    let mut actions = Vec::new();
    if row.update.is_some() {
        actions.push(Some(Action::Update));
    }
    if row.source.is_some() {
        actions.push(Some(Action::Switch));
        actions.push(Some(Action::Dependencies));
    }
    if !actions.is_empty() {
        actions.push(None);
    }
    actions.push(Some(Action::Associate));
    actions.push(Some(Action::Reveal));
    actions.push(None);
    actions.push(Some(Action::Delete));
    let items = actions
        .iter()
        .map(|action| match action {
            Some(action) => item(*action, row),
            None => MenuItem::Separator,
        })
        .collect();
    Menu { items, actions }
}

/// The row an action shows as.
fn item(action: Action, row: &Row) -> MenuItem {
    let (label, icon, danger) = match action {
        Action::Update => (update_label(row), Icon::lucide().arrow_up(), false),
        Action::Switch => (lang::t("instance.mods.menu.switch").to_owned(), Icon::lucide().repeat(), false),
        Action::Dependencies => (lang::t("instance.mods.menu.dependencies").to_owned(), Icon::lucide().layers(), false),
        Action::Associate => (lang::t("instance.mods.menu.associate").to_owned(), Icon::lucide().link(), false),
        Action::Reveal => (lang::t("instance.mods.menu.reveal").to_owned(), Icon::lucide().folder(), false),
        Action::Delete => (lang::t("instance.mods.menu.delete").to_owned(), Icon::lucide().trash(), true),
    };
    MenuItem::Action {
        label,
        icon: Some(icon),
        danger,
    }
}

/// The update row's label, naming the release it installs.
fn update_label(row: &Row) -> String {
    match row.update.as_ref() {
        Some(release) => lang::t_with("instance.mods.menu.update", &[("version", release.version_number.as_str())]).to_owned(),
        None => String::new(),
    }
}

#[cfg(test)]
#[path = "../../../../../tests/app/pages/instance/mods/menu.rs"]
mod tests;
