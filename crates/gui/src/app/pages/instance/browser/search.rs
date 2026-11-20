//! Find in the Files tab: a search for the entries under the folder on screen, by name, and optionally by the text inside files.
//!
//! The search runs on the blocking pool and can be cancelled. A new query, or a change to a toggle, cancels the walk in flight, and
//! its answer is dropped when it lands, since it belongs to a query that is no longer asked. Symbolic links are listed but never
//! followed, so a loop of links cannot hold the walk.

use std::fs;
use std::io::Read;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use chrono::{DateTime, Local};
use iced::Task;

use super::drag::Drag;
use super::{Message as BrowserMessage, Stash, State, scroll_to};
use crate::app::folder_listing::{self, Row};

/// The most matches a search lists. A search that finds more stops there, and says so.
pub const LIMIT: usize = 500;
/// The largest file whose text a search reads. A bigger file is matched by its name alone.
pub const CONTENT_LIMIT: u64 = 2 * 1024 * 1024;
/// The bytes read from the start of a file to tell text from binary: a NUL among them makes the file binary.
const SNIFF: usize = 8 * 1024;

/// The search the Files tab runs: the text asked for, the two toggles, and the state that lets a late answer be told apart.
#[derive(Debug, Default)]
pub struct Search {
    /// The text searched for, as typed.
    pub query: String,
    /// Whether the search goes into the folders under the folder on screen, not only the folder itself.
    pub subfolders: bool,
    /// Whether the search reads the text inside files, as well as their names.
    pub contents: bool,
    /// Whether the list shows the search rather than the folder.
    pub shown: bool,
    /// Whether the results for the current query have landed. Until they do the list says it is searching.
    pub ready: bool,
    /// Whether the results were cut at [`LIMIT`].
    pub truncated: bool,
    /// Counts the queries asked. An answer for an older count is dropped.
    pub request: u64,
    /// Stops the walk in flight, when a newer query or toggle replaces it.
    cancel: Arc<AtomicBool>,
}

impl Search {
    /// Whether the text searched for is not empty, so the list is a search.
    pub fn active(&self) -> bool {
        !self.query.is_empty()
    }

    /// Starts a new query: the walk in flight is stopped, and the answers asked for before are dropped. Returns the count the new
    /// query carries.
    pub fn renew(&mut self) -> u64 {
        self.cancel.store(true, Ordering::Relaxed);
        self.cancel = Arc::new(AtomicBool::new(false));
        self.request += 1;
        self.ready = false;
        self.truncated = false;
        self.request
    }

    /// The flag a walk of the current query checks, which a later [`Search::renew`] sets.
    pub fn walk_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.cancel)
    }
}

/// The matches of a search, in the order the list draws them.
#[derive(Debug, Default)]
pub struct Results {
    pub rows: Vec<Row>,
    pub truncated: bool,
}

/// Walks the folder `folder` of the instance at `root` and returns the entries that match `needle`, which is lowercase, and
/// whether the matches were cut at [`LIMIT`]. The walk stops early when `cancel` is set, and then returns what it has. Blocks, so
/// it runs on the blocking pool.
pub fn walk(root: &Path, folder: &str, subfolders: bool, contents: bool, needle: &str, cancel: &AtomicBool) -> Results {
    let start = if folder.is_empty() { root.to_path_buf() } else { root.join(folder) };
    let mut found = Results::default();
    let mut pending = vec![(start, folder.to_string())];
    while let Some((directory, prefix)) = pending.pop() {
        let Ok(entries) = fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            if cancel.load(Ordering::Relaxed) {
                return found;
            }
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            let name = entry.file_name().to_string_lossy().into_owned();
            let path = if prefix.is_empty() { name.clone() } else { format!("{prefix}/{name}") };
            let link = file_type.is_symlink();
            let hit = name.to_lowercase().contains(needle) || (contents && file_type.is_file() && text_contains(&entry.path(), needle));
            if hit {
                if found.rows.len() == LIMIT {
                    found.truncated = true;
                    return found;
                }
                found.rows.push(row_of(&entry.path(), name, path.clone(), link));
            }
            if file_type.is_dir() && subfolders && !link {
                pending.push((entry.path(), path));
            }
        }
    }
    found
}

/// The row a match shows: its name and path, and what the file system says of it. A link is shown as what it points at, as the
/// Files tab lists links.
fn row_of(full: &Path, name: String, path: String, link: bool) -> Row {
    let metadata = if link {
        fs::metadata(full).ok()
    } else {
        fs::symlink_metadata(full).ok()
    };
    let is_folder = metadata.as_ref().is_some_and(fs::Metadata::is_dir);
    Row {
        name,
        path,
        is_folder,
        is_link: link,
        size: metadata.as_ref().filter(|_| !is_folder).map_or(0, fs::Metadata::len),
        modified: metadata.and_then(|metadata| metadata.modified().ok()).map(DateTime::<Local>::from),
    }
}

/// Whether the text of the file at `path` contains `needle`, which is lowercase. A file over [`CONTENT_LIMIT`] is not read, and nor
/// is one with a NUL among its first bytes, since that is binary.
fn text_contains(path: &Path, needle: &str) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    if metadata.len() > CONTENT_LIMIT {
        return false;
    }
    let Ok(mut file) = fs::File::open(path) else {
        return false;
    };
    let mut bytes = Vec::new();
    if file.read_to_end(&mut bytes).is_err() {
        return false;
    }
    if bytes[..bytes.len().min(SNIFF)].contains(&0) {
        return false;
    }
    String::from_utf8_lossy(&bytes).to_lowercase().contains(needle)
}

/// How long a query waits for the next keystroke before it is searched, so a word typed quickly is searched once.
const DEBOUNCE: Duration = Duration::from_millis(200);

/// The messages of the find bar: the query, the two toggles, the clear button, and the answers of the walks.
#[derive(Debug, Clone)]
pub enum Message {
    /// The query text changed.
    Query(String),
    /// The Subfolders chip was pressed. The flag is its new value.
    Subfolders(bool),
    /// The File contents chip was pressed. The flag is its new value.
    Contents(bool),
    /// The clear button was pressed, or Escape was while the search showed.
    Clear,
    /// The wait after a query ran out for the query `request`, which starts its walk unless a newer query came since.
    Debounced(u64),
    /// The walk of the query `request` finished with these matches.
    Found { request: u64, rows: Vec<Row>, truncated: bool },
}

impl State {
    /// Carries out a message of the find bar.
    pub(super) fn search_message(&mut self, message: Message) -> Task<BrowserMessage> {
        match message {
            Message::Query(text) => {
                self.search.query = text;
                if !self.search.active() {
                    self.drop_search();
                    return Task::none();
                }
                self.enter_results();
                let request = self.search.renew();
                Task::perform(async { tokio::time::sleep(DEBOUNCE).await }, move |()| {
                    BrowserMessage::Search(Message::Debounced(request))
                })
            }
            Message::Subfolders(on) => {
                self.search.subfolders = on;
                self.restart_search()
            }
            Message::Contents(on) => {
                self.search.contents = on;
                self.restart_search()
            }
            Message::Clear => {
                self.drop_search();
                Task::none()
            }
            Message::Debounced(request) => {
                if request == self.search.request && self.search.active() {
                    self.start_walk()
                } else {
                    Task::none()
                }
            }
            Message::Found { request, rows, truncated } => {
                if request != self.search.request || !self.search.shown {
                    return Task::none();
                }
                self.search.ready = true;
                self.search.truncated = truncated;
                self.order = folder_listing::order(&rows, self.sort);
                self.rows = rows;
                self.selection.clear();
                self.scroll_y = 0.0;
                scroll_to(0.0)
            }
        }
    }

    /// Asks again for the query on screen, straight away, after a toggle changed how it is searched.
    fn restart_search(&mut self) -> Task<BrowserMessage> {
        if !self.search.active() {
            return Task::none();
        }
        self.enter_results();
        self.search.renew();
        self.start_walk()
    }

    /// Starts the walk of the query on screen on the blocking pool. Its answer lands as [`Message::Found`].
    fn start_walk(&mut self) -> Task<BrowserMessage> {
        let request = self.search.request;
        let cancel = self.search.walk_flag();
        let root = self.root.clone();
        let folder = self.folder.clone();
        let needle = self.search.query.to_lowercase();
        let (subfolders, contents) = (self.search.subfolders, self.search.contents);
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || walk(&root, &folder, subfolders, contents, &needle, &cancel))
                    .await
                    .unwrap_or_default()
            },
            move |results: Results| {
                BrowserMessage::Search(Message::Found {
                    request,
                    rows: results.rows,
                    truncated: results.truncated,
                })
            },
        )
    }

    /// Makes the list show the search. The folder's rows are set aside, to come back when the search ends.
    fn enter_results(&mut self) {
        if self.search.shown {
            return;
        }
        self.stash = Some(Stash {
            rows: std::mem::take(&mut self.rows),
            order: std::mem::take(&mut self.order),
            error: self.error.take(),
            listed: self.listed,
        });
        self.search.shown = true;
        self.search.ready = false;
        self.selection.clear();
        self.rename = None;
        self.hovered = None;
        self.drag = Drag::Idle;
        self.scroll_y = 0.0;
    }

    /// Ends the search: the list shows the folder again, as it was before the search.
    fn leave_results(&mut self) {
        if let Some(stash) = self.stash.take() {
            self.rows = stash.rows;
            self.order = stash.order;
            self.error = stash.error;
            self.listed = stash.listed;
        }
        self.search.shown = false;
        self.search.ready = false;
        self.search.truncated = false;
        self.selection.clear();
        self.rename = None;
        self.hovered = None;
        self.drag = Drag::Idle;
    }

    /// Drops the search: its query is cleared, its walk is stopped, and the list shows the folder again.
    pub(super) fn drop_search(&mut self) {
        self.search.query.clear();
        self.search.renew();
        self.leave_results();
    }
}

#[cfg(test)]
#[path = "../../../../../tests/app/pages/instance/browser/search.rs"]
mod tests;
