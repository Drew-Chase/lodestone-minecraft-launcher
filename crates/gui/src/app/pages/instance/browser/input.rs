//! What the pointer and the keyboard do in the Files tab: a press selects a row and arms a drag, two presses open it,
//! a release drops what was dragged, a menu row or a key runs its action, and the rename field takes a new name.

use std::path::PathBuf;
use std::time::Instant;

use iced::Task;
use iced::widget::{Id, operation};

use super::drag::{self, Drag, Released};
use super::{Action, DOUBLE_CLICK, DROP_GATHER, Key, Kind, Message, Mode, Rename, Request, State, ops, paths_of, request, scroll_to, view};

impl State {
    /// A row was pressed. Two presses close together open it. Otherwise the press selects, and arms a drag of the row.
    pub(super) fn pressed(&mut self, path: String) -> Task<Message> {
        if self.rename.as_ref().is_some_and(|rename| rename.path != path) {
            self.rename = None;
        }
        let now = Instant::now();
        let modified = self.held.command || self.held.shift;
        let double = !modified
            && self
                .last_press
                .as_ref()
                .is_some_and(|(last, at)| *last == path && now.duration_since(*at) < DOUBLE_CLICK);
        if double {
            self.last_press = None;
            self.drag = Drag::Idle;
            return self.activate(&path);
        }
        self.last_press = Some((path.clone(), now));
        if modified {
            let order = paths_of(&self.rows, &self.order);
            self.selection.click(&order, &path, self.held);
        } else if !self.selection.contains(&path) || self.read_only {
            // A press on a row that is already selected keeps the selection, so the whole of it can be dragged. The
            // release narrows it to the row when nothing was dragged.
            self.selection.only(&path);
        }
        if !modified && !self.read_only && self.rename.is_none() {
            self.drag = Drag::press(path);
        }
        Task::none()
    }

    /// The button came up: a press that never became a drag selects its row alone, and a drag over a folder that can
    /// take the rows moves them there, or copies them while the command key is held.
    pub(super) fn released(&mut self) -> Task<Message> {
        match self.drag.release() {
            Released::Nothing => Task::none(),
            Released::Click(path) => {
                self.selection.only(&path);
                Task::none()
            }
            Released::Drop(paths) => {
                let Some(into) = self.drop_target(&paths) else {
                    return Task::none();
                };
                let sources: Vec<PathBuf> = paths.iter().filter_map(|path| ops::inside(&self.root, path)).collect();
                let mode = if self.held.command { Mode::Copy } else { Mode::Move };
                request(Request::Transfer { sources, into, mode })
            }
        }
    }

    /// The folder under the pointer that can take the rows at `dragged`: a folder's row, a breadcrumb, or the Up button.
    pub(super) fn drop_target(&self, dragged: &[String]) -> Option<String> {
        let row = self
            .hovered
            .as_deref()
            .filter(|path| self.row(path).is_some_and(|row| row.is_folder))
            .map(str::to_string);
        row.or_else(|| self.crumb_hover.clone()).filter(|target| drag::accepts(target, dragged))
    }

    /// Opens the entry at `path`: a folder in the tab, and a file through the root.
    fn activate(&mut self, path: &str) -> Task<Message> {
        match self.row(path) {
            Some(row) if row.is_folder => self.show(row.path.clone()),
            Some(row) => request(Request::OpenFile(row.path.clone())),
            None => Task::none(),
        }
    }

    /// The entries an action on the row at `path` covers: the selection when the row is part of it, or the row alone.
    fn targets(&self, path: &str) -> Vec<String> {
        if self.selection.contains(path) {
            self.selection.paths()
        } else {
            vec![path.to_string()]
        }
    }

    /// Carries out a menu row, or asks the root to. `path` is the row the menu was for, or `None` for the folder on screen.
    pub(super) fn menu_action(&mut self, path: Option<String>, action: Action) -> Task<Message> {
        if self.read_only && action.changes() {
            return Task::none();
        }
        let on_folder = path.is_none();
        let path = path.unwrap_or_else(|| self.folder.clone());
        match action {
            Action::Open => self.activate(&path),
            Action::OpenExternal => request(Request::OpenExternal(path)),
            Action::Reveal => request(Request::Reveal(path)),
            Action::CopyPath => request(Request::CopyPath(path)),
            Action::Copy | Action::Cut => request(Request::Clip {
                paths: self.targets(&path),
                cut: action == Action::Cut,
            }),
            Action::Paste => {
                // A paste from a folder's row goes into that folder, and one from a file's row or the empty part of
                // the list goes into the folder on screen.
                let into = if on_folder || self.row(&path).is_some_and(|row| row.is_folder) {
                    path
                } else {
                    self.folder.clone()
                };
                request(Request::Paste { into })
            }
            Action::Rename => {
                self.selection.only(&path);
                self.begin_rename(path)
            }
            Action::NewFolder | Action::NewFile => request(Request::Create {
                folder: self.folder.clone(),
                kind: if action == Action::NewFolder { Kind::Folder } else { Kind::File },
            }),
            Action::Delete => request(Request::Delete(self.targets(&path))),
        }
    }

    /// Answers a key the list takes.
    pub(super) fn key(&mut self, key: Key) -> Task<Message> {
        let changes = matches!(key, Key::Cut | Key::Paste | Key::Delete | Key::Rename);
        if self.read_only && changes {
            return Task::none();
        }
        match key {
            Key::Copy | Key::Cut if !self.selection.is_empty() => request(Request::Clip {
                paths: self.selection.paths(),
                cut: key == Key::Cut,
            }),
            Key::Paste => request(Request::Paste { into: self.folder.clone() }),
            Key::SelectAll => {
                let order = paths_of(&self.rows, &self.order);
                self.selection.all(&order);
                Task::none()
            }
            Key::Delete if !self.selection.is_empty() => request(Request::Delete(self.selection.paths())),
            Key::Rename => match self.selection.single().map(str::to_string) {
                Some(path) => self.begin_rename(path),
                None => Task::none(),
            },
            Key::Open => match self.selection.single().map(str::to_string) {
                Some(path) => self.activate(&path),
                None => Task::none(),
            },
            Key::Up => self.update(Message::Up),
            Key::Step { delta, extend } => {
                let order = paths_of(&self.rows, &self.order);
                self.selection.step(&order, delta, extend);
                self.keep_lead_on_screen()
            }
            Key::Escape { taken } => {
                self.escape(taken);
                Task::none()
            }
            Key::Copy | Key::Cut | Key::Delete => Task::none(),
        }
    }

    /// Escape backs out of one thing at a time: a rename, then a drag, then a cut that waits for its paste, then the
    /// selection. An Escape the rename field took ends the rename and nothing else.
    fn escape(&mut self, taken: bool) {
        if self.rename.take().is_some() || taken {
            return;
        }
        if self.drag.is_armed() {
            self.drag = Drag::Idle;
        } else if self.clip.as_ref().is_some_and(|clip| clip.cut) {
            self.clip = None;
        } else {
            self.selection.clear();
        }
    }

    /// Shows the rename field in the row at `path`, holding its name, and puts the caret in it.
    pub(super) fn begin_rename(&mut self, path: String) -> Task<Message> {
        let Some(row) = self.row(&path) else {
            return Task::none();
        };
        if self.read_only {
            return Task::none();
        }
        // The name is selected up to its extension, so typing replaces the name and keeps the type.
        let stem = match row.name.rfind('.') {
            Some(dot) if dot > 0 && !row.is_folder => row.name[..dot].chars().count(),
            _ => row.name.chars().count(),
        };
        self.rename = Some(Rename {
            text: row.name.clone(),
            path,
        });
        self.drag = Drag::Idle;
        Task::batch([
            operation::focus(Id::new(view::RENAME_ID)),
            operation::select_range(Id::new(view::RENAME_ID), 0, stem),
        ])
    }

    /// Enter in the rename field: the root renames the entry, unless the name is unchanged.
    pub(super) fn submit_rename(&mut self) -> Task<Message> {
        let Some(rename) = self.rename.take() else {
            return Task::none();
        };
        let name = rename.text.trim().to_string();
        match self.row(&rename.path) {
            Some(row) if row.name != name => request(Request::Rename { path: rename.path, name }),
            _ => Task::none(),
        }
    }

    /// A file was dropped on the window. Files dropped together arrive one by one, so they are gathered for a moment and
    /// copied as one job.
    pub(super) fn dropped(&mut self, path: PathBuf) -> Task<Message> {
        self.file_hover = false;
        if self.read_only {
            return Task::none();
        }
        let first = self.drops.is_empty();
        self.drops.push(path);
        if first {
            // The sleep is made inside the future, so it is made on the runtime that polls it.
            Task::perform(async { tokio::time::sleep(DROP_GATHER).await }, |()| Message::DropsGathered)
        } else {
            Task::none()
        }
    }

    /// Scrolls the list so the row the keyboard reached is on screen.
    pub(super) fn keep_lead_on_screen(&mut self) -> Task<Message> {
        let Some(index) = self
            .selection
            .lead()
            .and_then(|lead| self.order.iter().position(|&row| self.rows[row].path == lead))
        else {
            return Task::none();
        };
        let top = index as f32 * view::ROW_HEIGHT;
        // Before the list has scrolled its height is not known, so a screenful is assumed.
        let height = if self.list_height > 0.0 {
            self.list_height
        } else {
            view::ASSUMED_HEIGHT
        };
        let target = if top < self.scroll_y {
            top
        } else if top + view::ROW_HEIGHT > self.scroll_y + height {
            top + view::ROW_HEIGHT - height
        } else {
            return Task::none();
        };
        self.scroll_y = target.max(0.0);
        scroll_to(self.scroll_y)
    }
}
