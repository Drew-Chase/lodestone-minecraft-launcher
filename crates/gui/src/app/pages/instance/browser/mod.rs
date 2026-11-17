//! Files tab — the instance folder as a file manager shows it: one folder at a time, with folders to open, a way up,
//! and the rows to select, drag, cut, copy, paste, rename and delete.
//!
//! The tab keeps what is on screen: the folder, its rows, the selection and the drag. Whatever writes to the disk, the
//! clipboard or the system is a [`Request`] the root carries out, since the alerts, the clipboard thread and the lock
//! on an instance are the root's. The folder is read again when the watch reports a change, so the list follows the
//! game and the system's own file manager.

pub mod drag;
mod input;
mod keys;
pub mod menu;
pub mod ops;
mod selection;
mod view;
mod watch;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use iced::widget::scrollable::AbsoluteOffset;
use iced::widget::{Id, operation};
use iced::{Point, Subscription, Task, event};

use crate::app::components::editor::{self, Editor, Loaded};
use crate::app::folder_listing::{self, Column, Entries, Row, Sort};
use drag::Drag;
pub use keys::Key;
pub use menu::Action;
pub use ops::{Kind, Mode};
pub use selection::Held;
use selection::Selection;

/// How close together two presses on one row must come to open it.
const DOUBLE_CLICK: Duration = Duration::from_millis(400);
/// How long dropped files are gathered before they are copied, so files dropped together are one job.
const DROP_GATHER: Duration = Duration::from_millis(80);

/// The files on the launcher's clipboard, and whether a paste moves them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Clip {
    /// The entries, as absolute paths.
    pub paths: Vec<PathBuf>,
    /// Whether they were cut, so a paste moves them.
    pub cut: bool,
}

/// The entry being renamed in its row, and what the field holds.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Rename {
    path: String,
    text: String,
}

/// What the tab asks the root to carry out. Paths are inside the instance, with `/` separators, and the empty path is
/// the instance folder.
#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    /// Opens a file: in the editor when it is text, and in its own app when it is not.
    OpenFile(String),
    /// Opens a file in the app the system gives its type.
    OpenExternal(String),
    /// Shows an entry in the system's file manager.
    Reveal(String),
    /// Copies an entry's full path as text.
    CopyPath(String),
    /// Puts entries on the clipboard. A `cut` moves them when they are pasted.
    Clip { paths: Vec<String>, cut: bool },
    /// Pastes the clipboard's files into a folder.
    Paste { into: String },
    /// Copies or moves entries into a folder. The sources are absolute, since dropped files come from anywhere.
    Transfer { sources: Vec<PathBuf>, into: String, mode: Mode },
    /// Moves entries to the trash, once the user has said so.
    Delete(Vec<String>),
    /// Renames an entry in its folder.
    Rename { path: String, name: String },
    /// Makes a new file or folder in a folder.
    Create { folder: String, kind: Kind },
    /// Writes the editor's text to its file.
    Save,
    /// Reads the editor's file again, in place of the text on screen.
    Revert,
    /// Closes the editor.
    CloseEditor,
    /// Looks at the editor's file on disk, after its folder changed.
    CheckFile,
}

/// Messages the Files tab handles, or passes to the root.
#[derive(Debug, Clone)]
pub enum Message {
    /// Shows a folder, by its path inside the instance.
    Open(String),
    /// Shows the folder above the one on screen.
    Up,
    /// A folder was read. Stale when the tab has moved to another instance or another read has started since.
    Listed {
        root: PathBuf,
        request: u64,
        folder: String,
        result: Result<Vec<Row>, String>,
    },
    /// The watch saw the folder at this absolute path change.
    Changed(PathBuf),
    /// A column heading was pressed.
    Sort(Column),
    /// The list scrolled to `offset`, in a viewport `height` tall.
    Scrolled { offset: f32, height: f32 },
    /// The pointer entered (`hovered`) or left the row at `path`.
    Hover { path: String, hovered: bool },
    /// The pointer entered or left a place above the list that takes a drop: a breadcrumb, or the Up button. `folder` is
    /// the folder it stands for.
    CrumbHover { folder: String, hovered: bool },
    /// The row at this path was pressed.
    Press(String),
    /// The empty part of the list was pressed.
    ClearSelection,
    /// The pointer moved while a row is held, to this place in the tab.
    Moved(Point),
    /// The button that held a row came up.
    Released,
    /// The modifier keys changed.
    Held(Held),
    /// A key the list answers was pressed.
    Key(Key),
    /// A menu row was picked: for the row at `path`, or for the folder on screen when `None`. A separator sends no action.
    Menu { path: Option<String>, action: Option<Action> },
    /// The rename field's text changed.
    RenameInput(String),
    /// Enter was pressed in the rename field.
    RenameSubmit,
    /// A file is being dragged over the window (`true`), or has left it.
    FileHover(bool),
    /// A file was dropped on the window.
    Dropped(PathBuf),
    /// The files dropped together are ready to copy.
    DropsGathered,
    /// A message of the editor beside the list.
    Editor(editor::Message),
    /// Something for the root to carry out. The root takes it before the tab sees it.
    Request(Request),
}

/// The Files tab's state.
#[derive(Debug, Default)]
pub struct State {
    /// The instance folder.
    root: PathBuf,
    /// The folder on screen, inside the instance. Empty is the instance folder itself.
    folder: String,
    /// The folder's entries, as read.
    rows: Vec<Row>,
    /// Indices into `rows` in the order the list draws them.
    order: Vec<usize>,
    sort: Sort,
    /// Whether the folder on screen has been read. Until then the list says it is reading.
    listed: bool,
    /// Why the folder could not be read, when it could not.
    error: Option<String>,
    /// Counts the reads started. A read that is not the newest is dropped when it lands.
    request: u64,
    selection: Selection,
    held: Held,
    scroll_y: f32,
    /// How tall the list's viewport was when it last scrolled. Zero until it has.
    list_height: f32,
    /// The path of the row under the pointer.
    hovered: Option<String>,
    /// The folder of the breadcrumb or Up button under the pointer.
    crumb_hover: Option<String>,
    drag: Drag,
    /// The row pressed last and when, to tell a double click.
    last_press: Option<(String, Instant)>,
    rename: Option<Rename>,
    /// The row to start renaming once the folder has been read again, after a new entry was made.
    rename_next: Option<String>,
    /// The rows to select once the folder has been read again, after a paste or a rename wrote them.
    select_next: Vec<String>,
    clip: Option<Clip>,
    /// Whether the instance takes no change: it is locked to its modpack.
    read_only: bool,
    /// Whether a file is being dragged over the window.
    file_hover: bool,
    /// The files dropped on the window that have not been copied yet.
    drops: Vec<PathBuf>,
    /// The file open in the editor beside the list, when one is.
    editor: Option<Editor>,
}

impl State {
    /// The tab for the instance folder at `root`. A `read_only` tab changes nothing in it.
    pub fn new(root: PathBuf, read_only: bool) -> Self {
        Self {
            root,
            read_only,
            ..Self::default()
        }
    }

    /// The instance folder.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The folder on screen, inside the instance.
    #[cfg(test)]
    pub fn folder(&self) -> &str {
        &self.folder
    }

    /// Whether the tab changes nothing in the instance folder.
    #[cfg(test)]
    pub fn read_only(&self) -> bool {
        self.read_only
    }

    /// Records whether the instance is locked, which makes the tab read-only. A drag or a rename under way is dropped.
    pub fn set_read_only(&mut self, read_only: bool) {
        self.read_only = read_only;
        if read_only {
            self.drag = Drag::Idle;
            self.rename = None;
        }
        if let Some(editor) = &mut self.editor {
            editor.set_read_only(read_only);
        }
    }

    /// The file open in the editor, when one is.
    pub fn editor(&self) -> Option<&Editor> {
        self.editor.as_ref()
    }

    /// The editor, for the root to update as the disk answers.
    pub fn editor_mut(&mut self) -> Option<&mut Editor> {
        self.editor.as_mut()
    }

    /// Opens `loaded` in the editor beside the list, in place of any file open there.
    pub fn open_editor(&mut self, loaded: Loaded) {
        self.editor = Some(Editor::new(loaded, self.read_only));
    }

    /// Closes the editor, and whatever it held goes with it.
    pub fn close_editor(&mut self) {
        self.editor = None;
    }

    /// The folder that holds the editor's file, as an absolute path.
    fn editor_folder(&self) -> Option<PathBuf> {
        let editor = self.editor.as_ref()?;
        ops::inside(&self.root, drag::parent(editor.path()))
    }

    /// The files on the launcher's clipboard.
    pub fn clip(&self) -> Option<&Clip> {
        self.clip.as_ref()
    }

    /// Replaces the files on the launcher's clipboard.
    pub fn set_clip(&mut self, clip: Option<Clip>) {
        self.clip = clip;
    }

    /// The row at `path`, when the folder on screen holds it.
    pub fn row(&self, path: &str) -> Option<&Row> {
        self.rows.iter().find(|row| row.path == path)
    }

    /// Reads the folder on screen when it has not been read yet, as the tab does when it is first shown.
    pub fn open(&mut self) -> Task<Message> {
        // A press held when the tab was left has no release to wait for.
        self.drag = Drag::Idle;
        if self.listed || self.root.as_os_str().is_empty() {
            Task::none()
        } else {
            self.read()
        }
    }

    /// Reads the folder on screen again, after something in it changed.
    pub fn refresh(&mut self) -> Task<Message> {
        if self.root.as_os_str().is_empty() { Task::none() } else { self.read() }
    }

    /// Starts renaming the entry at `path` once the folder has been read again. A new file or folder is named this way.
    pub fn rename_when_listed(&mut self, path: String) {
        self.rename_next = Some(path);
    }

    /// Selects the entries at `paths` once the folder has been read again, as the entries a paste wrote are. The ones
    /// the folder on screen does not hold are passed over.
    pub fn select_when_listed(&mut self, paths: Vec<String>) {
        self.select_next = paths;
    }

    /// Whether the entry at `path` was cut and waits for its paste.
    fn is_cut(&self, path: &str) -> bool {
        self.clip
            .as_ref()
            .filter(|clip| clip.cut)
            .is_some_and(|clip| ops::inside(&self.root, path).is_some_and(|entry| clip.paths.contains(&entry)))
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Open(folder) => self.show(folder),
            Message::Up => {
                if self.folder.is_empty() {
                    Task::none()
                } else {
                    let parent = drag::parent(&self.folder).to_string();
                    self.show(parent)
                }
            }
            Message::Listed {
                root,
                request,
                folder,
                result,
            } => {
                if root == self.root && request == self.request && folder == self.folder {
                    self.listed(result)
                } else {
                    Task::none()
                }
            }
            Message::Changed(path) => {
                // The folder on screen is read again, and the file open in the editor is looked at when its folder is
                // the one that changed.
                let listed = if ops::inside(&self.root, &self.folder).is_some_and(|shown| shown == path) {
                    self.read()
                } else {
                    Task::none()
                };
                let checked = if self.editor_folder().is_some_and(|folder| folder == path) {
                    request(Request::CheckFile)
                } else {
                    Task::none()
                };
                Task::batch([listed, checked])
            }
            Message::Sort(column) => {
                self.sort = self.sort.by(column);
                self.resort();
                Task::none()
            }
            Message::Scrolled { offset, height } => {
                self.scroll_y = offset.max(0.0);
                self.list_height = height;
                Task::none()
            }
            Message::Hover { path, hovered } => {
                if hovered {
                    self.hovered = Some(path);
                } else if self.hovered.as_deref() == Some(path.as_str()) {
                    self.hovered = None;
                }
                Task::none()
            }
            Message::CrumbHover { folder, hovered } => {
                if hovered {
                    self.crumb_hover = Some(folder);
                } else if self.crumb_hover.as_deref() == Some(folder.as_str()) {
                    self.crumb_hover = None;
                }
                Task::none()
            }
            Message::Press(path) => self.pressed(path),
            Message::ClearSelection => {
                self.selection.clear();
                self.rename = None;
                Task::none()
            }
            Message::Moved(at) => {
                self.drag.moved(at, &self.selection.paths());
                Task::none()
            }
            Message::Released => self.released(),
            Message::Held(held) => {
                self.held = held;
                Task::none()
            }
            Message::Key(key) => self.key(key),
            Message::Menu { path, action } => match action {
                Some(action) => self.menu_action(path, action),
                None => Task::none(),
            },
            Message::RenameInput(text) => {
                if let Some(rename) = &mut self.rename {
                    rename.text = text;
                }
                Task::none()
            }
            Message::RenameSubmit => self.submit_rename(),
            Message::FileHover(hovering) => {
                self.file_hover = hovering && !self.read_only;
                Task::none()
            }
            Message::Dropped(path) => self.dropped(path),
            Message::DropsGathered => {
                let sources = std::mem::take(&mut self.drops);
                if sources.is_empty() {
                    Task::none()
                } else {
                    request(Request::Transfer {
                        sources,
                        into: self.folder.clone(),
                        mode: Mode::Copy,
                    })
                }
            }
            // Saving, reverting and closing touch the disk and may need a question asked, so the root answers them.
            Message::Editor(editor::Message::Save) => request(Request::Save),
            Message::Editor(editor::Message::Revert) => request(Request::Revert),
            Message::Editor(editor::Message::Close) => request(Request::CloseEditor),
            Message::Editor(message) => match &mut self.editor {
                Some(editor) => editor.update(message).map(Message::Editor),
                None => Task::none(),
            },
            // The root carries these out before the tab sees them.
            Message::Request(_) => Task::none(),
        }
    }

    /// The tab's keys, its file drops, and the watch on the folder on screen. `blocked` is whether a dialog is over the
    /// page, which then has the keys and the drops.
    pub fn subscription(&self, blocked: bool) -> Subscription<Message> {
        if self.root.as_os_str().is_empty() {
            return Subscription::none();
        }
        let watch = match ops::inside(&self.root, &self.folder) {
            Some(folder) => Subscription::run_with(folder, watch::subscription),
            None => Subscription::none(),
        };
        // The editor's file may sit in another folder than the one on screen, and it is followed too.
        let shown = ops::inside(&self.root, &self.folder);
        let editing = match self.editor_folder().filter(|folder| Some(folder) != shown.as_ref()) {
            Some(folder) => Subscription::run_with(folder, watch::subscription),
            None => Subscription::none(),
        };
        if blocked {
            return Subscription::batch([watch, editing]);
        }
        Subscription::batch([watch, editing, event::listen_with(keys::event), event::listen_with(keys::drop_event)])
    }

    /// Makes `folder` the folder on screen and reads it.
    fn show(&mut self, folder: String) -> Task<Message> {
        if ops::inside(&self.root, &folder).is_none() {
            return Task::none();
        }
        self.folder = folder;
        self.rows.clear();
        self.order.clear();
        self.listed = false;
        self.error = None;
        self.selection.clear();
        self.hovered = None;
        self.rename = None;
        self.drag = Drag::Idle;
        self.scroll_y = 0.0;
        Task::batch([self.read(), scroll_to(0.0)])
    }

    /// Reads the folder on screen on the blocking pool. The read lands as `Listed`.
    fn read(&mut self) -> Task<Message> {
        self.request += 1;
        let (root, folder, request) = (self.root.clone(), self.folder.clone(), self.request);
        let listing = folder_listing::read_folder(root.clone(), folder.clone(), Entries::All);
        Task::perform(listing, move |result| Message::Listed {
            root: root.clone(),
            request,
            folder: folder.clone(),
            result,
        })
    }

    /// Takes a read of the folder on screen. The selection keeps the rows that are still there.
    fn listed(&mut self, result: Result<Vec<Row>, String>) -> Task<Message> {
        self.listed = true;
        match result {
            Ok(rows) => {
                self.rows = rows;
                self.error = None;
            }
            Err(error) => {
                self.rows.clear();
                self.error = Some(error);
            }
        }
        self.resort();
        let rows = &self.rows;
        self.selection.retain(|path| rows.iter().any(|row| row.path == path));
        if self.rename.as_ref().is_some_and(|rename| self.row(&rename.path).is_none()) {
            self.rename = None;
        }
        let select = std::mem::take(&mut self.select_next);
        if select.iter().any(|path| self.row(path).is_some()) {
            self.selection.clear();
            let order = paths_of(&self.rows, &self.order);
            let adding = Held { command: true, shift: false };
            for path in select.iter().filter(|path| order.contains(&path.as_str())) {
                self.selection.click(&order, path, adding);
            }
        }
        match self.rename_next.take() {
            Some(path) if self.row(&path).is_some() => {
                self.selection.only(&path);
                Task::batch([self.begin_rename(path), self.keep_lead_on_screen()])
            }
            _ => Task::none(),
        }
    }

    /// Sorts the rows again, after the rows or the sort changed.
    fn resort(&mut self) {
        self.order = folder_listing::order(&self.rows, self.sort);
    }
}

/// The paths of `rows` in the order the list draws them.
fn paths_of<'a>(rows: &'a [Row], order: &[usize]) -> Vec<&'a str> {
    order.iter().map(|&index| rows[index].path.as_str()).collect()
}

/// A request for the root, as the task that delivers it.
fn request(request: Request) -> Task<Message> {
    Task::done(Message::Request(request))
}

/// Scrolls the list to `offset`.
fn scroll_to(offset: f32) -> Task<Message> {
    operation::scroll_to(Id::new(view::SCROLL_ID), AbsoluteOffset { x: None, y: Some(offset) })
}

#[cfg(test)]
#[path = "../../../../../tests/app/pages/instance/browser/mod.rs"]
mod tests;
