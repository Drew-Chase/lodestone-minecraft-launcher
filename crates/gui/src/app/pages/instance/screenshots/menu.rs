//! The capture menu: the rows a right-click on a capture shows, and the action each row asks for. The
//! grid, the Overview strip and the viewer all use it, and the root carries the actions out.

use std::path::Path;

use aurora_leaf::Icon;
use iced::Element;

use super::Message;
use crate::app::components::context_menu::ContextMenu;
use crate::app::components::menu::MenuItem;
use crate::app::lang;

/// What a menu row asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Copies the image to the clipboard.
    Copy,
    /// Opens the file in the app the system gives PNG files.
    Open,
    /// Opens the screenshots folder in the file manager.
    Reveal,
    /// Moves the capture to the trash, once the user has said so.
    Delete,
}

/// A menu as the widgets show it: its rows, and the action each row asks for. A separator asks for
/// nothing, so its entry in `actions` is `None`.
pub struct Menu {
    pub items: Vec<MenuItem>,
    pub actions: Vec<Option<Action>>,
}

/// The menu for a capture.
pub fn menu() -> Menu {
    let actions = vec![Some(Action::Copy), Some(Action::Open), Some(Action::Reveal), None, Some(Action::Delete)];
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
        Action::Copy => (lang::t("instance.screenshots.menu.copy"), Icon::lucide().copy(), false),
        Action::Open => (lang::t("instance.screenshots.menu.open"), Icon::lucide().external_link(), false),
        Action::Reveal => (lang::t("instance.screenshots.menu.reveal"), Icon::lucide().folder(), false),
        Action::Delete => (lang::t("instance.screenshots.menu.delete"), Icon::lucide().trash(), true),
    };
    MenuItem::Action {
        label: label.to_owned(),
        icon: Some(icon),
        danger,
    }
}

/// Wraps `content` so a right-click on it opens the menu for the capture at `path`.
pub fn attach<'a>(content: impl Into<Element<'a, Message>>, path: &Path) -> Element<'a, Message> {
    let Menu { items, actions } = menu();
    let path = path.to_path_buf();
    ContextMenu::new(content, path.display().to_string())
        .items(items)
        .on_select(move |index| Message::Menu {
            path: path.clone(),
            action: actions[index],
        })
        .into()
}

#[cfg(test)]
#[path = "../../../../../tests/app/pages/instance/screenshots/menu.rs"]
mod tests;
