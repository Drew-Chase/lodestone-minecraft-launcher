//! The instance's action menu: the rows that the right-click menu on a card or row and the
//! hero's ⋯ button share, and the action each row asks for. The root carries the actions out.

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
}

/// The menu's rows, top to bottom. [`action_at`] reads them by index.
pub fn items() -> Vec<MenuItem> {
    vec![
        MenuItem::Action {
            label: lang::t("common.play").to_owned(),
            icon: Some(Icon::lucide().play()),
            danger: false,
        },
        MenuItem::Action {
            label: lang::t("instance.menu.edit").to_owned(),
            icon: Some(Icon::lucide().settings()),
            danger: false,
        },
        MenuItem::Action {
            label: lang::t("instance.menu.reveal").to_owned(),
            icon: Some(Icon::lucide().folder()),
            danger: false,
        },
        MenuItem::Separator,
        MenuItem::Action {
            label: lang::t("instance.menu.delete").to_owned(),
            icon: Some(Icon::lucide().trash()),
            danger: true,
        },
    ]
}

/// The action a row index picks. A separator picks nothing.
pub fn action_at(index: usize) -> Option<Action> {
    match index {
        0 => Some(Action::Play),
        1 => Some(Action::Edit),
        2 => Some(Action::Reveal),
        4 => Some(Action::Delete),
        _ => None,
    }
}

#[cfg(test)]
#[path = "../../../../tests/app/pages/library/instance_menu.rs"]
mod tests;
