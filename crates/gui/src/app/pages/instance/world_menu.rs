//! The world menu: the rows a world's ⋯ button and a right-click on its card show, and the action each
//! row asks for. The root carries the actions out.

use aurora_leaf::Icon;

use crate::app::components::menu::MenuItem;
use crate::app::lang;

/// What a menu row asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Quick-plays the world, or starts the instance when the version cannot.
    Play,
    /// Opens the world's folder in the file manager.
    Reveal,
    /// Copies the world's seed to the clipboard.
    CopySeed,
    /// Zips the world into the instance's `backups/` folder.
    BackUp,
    /// Moves the world to the trash, once the user has said so.
    Delete,
}

/// A menu as the widgets show it: its rows, and the action each row asks for. A separator asks for
/// nothing, so its entry in `actions` is `None`.
pub struct Menu {
    pub items: Vec<MenuItem>,
    pub actions: Vec<Option<Action>>,
}

/// The menu for a world. Copy seed is listed only when the world has a seed to copy.
pub fn menu(has_seed: bool) -> Menu {
    let mut actions = vec![Some(Action::Play), None, Some(Action::Reveal)];
    if has_seed {
        actions.push(Some(Action::CopySeed));
    }
    actions.push(Some(Action::BackUp));
    actions.push(None);
    actions.push(Some(Action::Delete));
    let items = actions
        .iter()
        .map(|action| match action {
            Some(action) => item(*action),
            None => MenuItem::Separator,
        })
        .collect();
    Menu { items, actions }
}

/// The row an action shows as.
fn item(action: Action) -> MenuItem {
    let (label, icon, danger) = match action {
        Action::Play => (lang::t("common.play"), Icon::lucide().play(), false),
        Action::Reveal => (lang::t("instance.worlds.menu.open_folder"), Icon::lucide().folder(), false),
        Action::CopySeed => (lang::t("instance.worlds.menu.copy_seed"), Icon::lucide().copy(), false),
        Action::BackUp => (lang::t("instance.worlds.menu.back_up"), Icon::lucide().archive(), false),
        Action::Delete => (lang::t("instance.worlds.menu.delete"), Icon::lucide().trash(), true),
    };
    MenuItem::Action {
        label: label.to_owned(),
        icon: Some(icon),
        danger,
    }
}

#[cfg(test)]
#[path = "../../../../tests/app/pages/instance/world_menu.rs"]
mod tests;
