//! The Files tab's menus: the rows a right-click on an entry shows, which the ⋯ button on the row shows too, and the
//! rows a right-click on the empty part of the list shows for the folder itself. The root carries the actions out.

use aurora_leaf::Icon;

use crate::app::components::menu::MenuItem;
use crate::app::lang;

/// What a menu row asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Opens a folder in the tab, or a file in the editor when it is text and in its own app when it is not.
    Open,
    /// Opens the file in the app the system gives its type.
    OpenExternal,
    /// Shows the entry in the system's file manager.
    Reveal,
    /// Puts the entries on the clipboard, to paste as copies.
    Copy,
    /// Puts the entries on the clipboard, to move when they are pasted.
    Cut,
    /// Pastes the clipboard's files into the folder.
    Paste,
    /// Copies the entry's full path as text.
    CopyPath,
    /// Starts renaming the entry in its row.
    Rename,
    /// Makes a folder, and starts naming it.
    NewFolder,
    /// Makes an empty file, and starts naming it.
    NewFile,
    /// Moves the entries to the trash, once the user has said so.
    Delete,
}

impl Action {
    /// Whether the action changes what the instance folder holds. A read-only tab offers none of these.
    pub fn changes(self) -> bool {
        matches!(
            self,
            Self::Cut | Self::Paste | Self::Rename | Self::NewFolder | Self::NewFile | Self::Delete
        )
    }
}

/// A menu as the widgets show it: its rows, and the action each row asks for. A separator asks for nothing, so its
/// entry in `actions` is `None`.
#[derive(Debug, Clone, Default)]
pub struct Menu {
    pub items: Vec<MenuItem>,
    pub actions: Vec<Option<Action>>,
}

/// What a row menu is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Target {
    pub is_folder: bool,
    /// How many entries the menu acts on: the selection when the row is part of it, or the row alone.
    pub count: usize,
}

/// The menu for an entry. `read_only` leaves out every row that changes the folder, and `can_paste` says whether the
/// clipboard may hold files.
pub fn entry(target: Target, read_only: bool, can_paste: bool) -> Menu {
    let groups: Vec<Vec<Action>> = if target.count > 1 {
        vec![vec![Action::Copy, Action::Cut], vec![Action::Delete]]
    } else if target.is_folder {
        let mut clipboard = vec![Action::Copy, Action::Cut];
        if can_paste {
            clipboard.push(Action::Paste);
        }
        clipboard.push(Action::CopyPath);
        vec![vec![Action::Open, Action::Reveal], clipboard, vec![Action::Rename, Action::Delete]]
    } else {
        vec![
            vec![Action::Open, Action::OpenExternal, Action::Reveal],
            vec![Action::Copy, Action::Cut, Action::CopyPath],
            vec![Action::Rename, Action::Delete],
        ]
    };
    build(groups, read_only)
}

/// The menu for the folder on screen, shown from the empty part of the list.
pub fn folder(read_only: bool, can_paste: bool) -> Menu {
    let mut make = Vec::new();
    if can_paste {
        make.push(Action::Paste);
    }
    make.extend([Action::NewFolder, Action::NewFile]);
    build(vec![make, vec![Action::Reveal, Action::CopyPath]], read_only)
}

/// Joins `groups` into one menu with a separator between each pair. A read-only menu drops the rows that change the
/// folder, and a group left empty takes its separator with it.
fn build(groups: Vec<Vec<Action>>, read_only: bool) -> Menu {
    let mut menu = Menu::default();
    for group in groups {
        let group: Vec<Action> = group.into_iter().filter(|action| !(read_only && action.changes())).collect();
        if group.is_empty() {
            continue;
        }
        if !menu.items.is_empty() {
            menu.items.push(MenuItem::Separator);
            menu.actions.push(None);
        }
        for action in group {
            menu.items.push(item(action));
            menu.actions.push(Some(action));
        }
    }
    menu
}

/// The row an action shows as.
fn item(action: Action) -> MenuItem {
    let (label, icon, danger) = match action {
        Action::Open => (lang::t("instance.files.menu.open"), Icon::lucide().folder_open(), false),
        Action::OpenExternal => (lang::t("instance.files.menu.open_external"), Icon::lucide().external_link(), false),
        Action::Reveal => (lang::t("instance.files.menu.reveal"), Icon::lucide().folder(), false),
        Action::Copy => (lang::t("instance.files.menu.copy"), Icon::lucide().copy(), false),
        Action::Cut => (lang::t("instance.files.menu.cut"), Icon::lucide().scissors(), false),
        Action::Paste => (lang::t("instance.files.menu.paste"), Icon::lucide().clipboard_paste(), false),
        Action::CopyPath => (lang::t("instance.files.menu.copy_path"), Icon::lucide().clipboard(), false),
        Action::Rename => (lang::t("instance.files.menu.rename"), Icon::lucide().pencil(), false),
        Action::NewFolder => (lang::t("instance.files.menu.new_folder"), Icon::lucide().folder_plus(), false),
        Action::NewFile => (lang::t("instance.files.menu.new_file"), Icon::lucide().file_plus(), false),
        Action::Delete => (lang::t("instance.files.menu.delete"), Icon::lucide().trash(), true),
    };
    MenuItem::Action {
        label: label.to_owned(),
        icon: Some(icon),
        danger,
    }
}

#[cfg(test)]
#[path = "../../../../../tests/app/pages/instance/browser/menu.rs"]
mod tests;
