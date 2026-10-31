//! The instance's action menu: the rows that the right-click menu on a card or row and the instance
//! page's ⋯ button show, and the action each row asks for. The root carries the actions out.

use aurora_leaf::Icon;

use crate::app::components::menu::MenuItem;
use crate::app::lang;

/// What a menu row asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Play,
    Edit,
    Reveal,
    Delete,
    /// Stops an install that has not finished, and removes what it made.
    CancelInstall,
}

/// A menu as the widgets show it: its rows, and the action each row asks for. A separator asks for
/// nothing, so its entry in `actions` is `None`.
pub struct Menu {
    pub items: Vec<MenuItem>,
    pub actions: Vec<Option<Action>>,
}

/// The menu for an instance. An install that has not finished offers only cancelling it. Reveal is
/// listed when `reveal` is set; the instance page leaves it out for a local instance, whose folder
/// button already reveals the folder.
pub fn menu(installing: bool, reveal: bool) -> Menu {
    let actions: Vec<Option<Action>> = if installing {
        vec![Some(Action::CancelInstall)]
    } else {
        let mut actions = vec![Some(Action::Play), Some(Action::Edit)];
        if reveal {
            actions.push(Some(Action::Reveal));
        }
        actions.push(None);
        actions.push(Some(Action::Delete));
        actions
    };
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
        Action::Edit => (lang::t("instance.menu.edit"), Icon::lucide().settings(), false),
        Action::Reveal => (lang::t("instance.menu.reveal"), Icon::lucide().folder(), false),
        Action::Delete => (lang::t("instance.menu.delete"), Icon::lucide().trash(), true),
        Action::CancelInstall => (lang::t("downloads.cancel_install"), Icon::lucide().x(), true),
    };
    MenuItem::Action {
        label: label.to_owned(),
        icon: Some(icon),
        danger,
    }
}

#[cfg(test)]
#[path = "../../../../tests/app/pages/library/instance_menu.rs"]
mod tests;
