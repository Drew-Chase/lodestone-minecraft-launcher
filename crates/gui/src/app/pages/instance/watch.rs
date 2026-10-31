//! Watches the instance's `saves/` and `screenshots/` folders, so the Worlds and Screenshots tabs follow
//! the game as it writes them. A game writes in bursts, so the events are gathered into a batch that is
//! read back once the folders go quiet. The page applies what the batch reads; this module only reports
//! which saves and captures changed.

use std::collections::{BTreeSet, HashSet};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::time::Duration;

use iced::futures::channel::mpsc::Sender;
use iced::futures::{SinkExt, Stream};
use notify::event::{AccessKind, AccessMode, EventKind};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc;
use tokio::time::Instant;
use tracing::warn;

use super::Message;
use super::files::{self, ScreenshotFile, WorldFile};

/// The folders the watch follows, each recursively once it exists.
const FOLDERS: [&str; 2] = ["saves", "screenshots"];
/// The lock a running game holds on a save, written inside the save's folder.
const SESSION_LOCK: &str = "session.lock";
/// How long the folders must stay quiet before a batch is read: half a second.
const QUIET: Duration = Duration::from_millis(500);
/// The longest a batch may wait, so a game that writes without pause still refreshes the tabs.
const LONGEST: Duration = Duration::from_secs(2);

/// What changed in the instance folder.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Change {
    /// The `saves` folder itself was created, removed or changed: every save is read again.
    Worlds,
    /// A save's folder, or something inside it, changed: the save in this folder is read again.
    World(PathBuf),
    /// The `screenshots` folder itself was created, removed or changed: every capture is listed again.
    Shots,
    /// A capture changed: its file is read again.
    Shot(PathBuf),
}

/// What reading a change back from disk found. A save or capture that is gone reads as `None`.
#[derive(Debug, Clone)]
pub enum Update {
    /// Every save in `saves/`, most recent first.
    Worlds(Vec<WorldFile>),
    /// The save in this folder, or `None` when it is gone or has no `level.dat`.
    World(PathBuf, Option<WorldFile>),
    /// Every capture in `screenshots/`, most recent first.
    Shots(Vec<ScreenshotFile>),
    /// The capture at this path, or `None` when it is gone or is not a PNG file.
    Shot(PathBuf, Option<ScreenshotFile>),
}

/// The change a path reports, or `None` when it lies outside the two folders or is not a capture.
pub fn classify(folder: &Path, path: &Path) -> Option<Change> {
    let relative = path.strip_prefix(folder).ok()?;
    let parts: Vec<&OsStr> = relative.components().map(|part| part.as_os_str()).collect();
    match parts.as_slice() {
        [root] if *root == OsStr::new("saves") => Some(Change::Worlds),
        // The lock a running game holds on its save is not part of the save, so it is no change to it.
        [root, _, lock] if *root == OsStr::new("saves") && *lock == OsStr::new(SESSION_LOCK) => None,
        [root, save, ..] if *root == OsStr::new("saves") => Some(Change::World(folder.join("saves").join(save))),
        [root] if *root == OsStr::new("screenshots") => Some(Change::Shots),
        [root, _] if *root == OsStr::new("screenshots") && is_png(path) => Some(Change::Shot(path.to_path_buf())),
        _ => None,
    }
}

/// The changes to read, sorted, with those a listing already covers left out: a save change is covered by
/// a change to `saves` itself, and a capture change by a change to `screenshots`.
pub fn coalesce(changes: impl IntoIterator<Item = Change>) -> Vec<Change> {
    let all: BTreeSet<Change> = changes.into_iter().collect();
    let worlds = all.contains(&Change::Worlds);
    let shots = all.contains(&Change::Shots);
    all.into_iter()
        .filter(|change| match change {
            Change::World(_) => !worlds,
            Change::Shot(_) => !shots,
            Change::Worlds | Change::Shots => true,
        })
        .collect()
}

/// Reads the changes back from the instance folder at `folder`, in the order they are given. Runs on the
/// blocking pool.
pub fn read(folder: &Path, changes: Vec<Change>) -> Vec<Update> {
    changes
        .into_iter()
        .map(|change| match change {
            Change::Worlds => Update::Worlds(files::scan_worlds(folder)),
            Change::World(path) => {
                let world = files::read_world(&path);
                Update::World(path, world)
            }
            Change::Shots => Update::Shots(files::scan_screenshots(folder)),
            Change::Shot(path) => {
                let shot = files::read_shot(&path);
                Update::Shot(path, shot)
            }
        })
        .collect()
}

/// The changes in the instance folder at `folder`, as messages. The folders stay watched for as long as the
/// subscription lives, so leaving the instance page stops the watch.
#[allow(clippy::ptr_arg, reason = "Subscription::run_with hands its builder the data by reference, as a PathBuf")]
pub fn subscription(folder: &PathBuf) -> impl Stream<Item = Message> + use<> {
    let folder = folder.clone();
    iced::stream::channel(16, async move |mut output| {
        if let Err(error) = run(&folder, &mut output).await {
            warn!("Watching {} failed: {error}", folder.display());
        }
    })
}

/// Watches `folder` and reports each batch of changes, until the page drops the subscription.
async fn run(folder: &Path, output: &mut Sender<Message>) -> notify::Result<()> {
    let (events_tx, mut events) = mpsc::unbounded_channel::<notify::Event>();
    let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        if let Ok(event) = event {
            // The receiver is gone only once the watch has ended, when nothing needs the event.
            let _ = events_tx.send(event);
        }
    })?;
    watcher.watch(folder, RecursiveMode::NonRecursive)?;
    let canonical = std::fs::canonicalize(folder).unwrap_or_else(|_| folder.to_path_buf());
    let mut followed = HashSet::new();
    for name in FOLDERS {
        follow(&mut watcher, &folder.join(name), &mut followed);
    }

    let mut batch = Batch::default();
    loop {
        let deadline = batch.deadline();
        let quiet = async move {
            match deadline {
                Some(at) => tokio::time::sleep_until(at).await,
                None => std::future::pending::<()>().await,
            }
        };
        tokio::select! {
            event = events.recv() => {
                let Some(event) = event else {
                    break;
                };
                if !relevant(&event.kind) {
                    continue;
                }
                for event_path in &event.paths {
                    let path = rebase(folder, &canonical, event_path);
                    if is_folder(folder, &path) {
                        follow(&mut watcher, &path, &mut followed);
                    }
                    if let Some(change) = classify(folder, &path) {
                        batch.push(change);
                    }
                }
            }
            () = quiet => {
                let changes = batch.take();
                if output.send(Message::FilesChanged { folder: folder.to_path_buf(), changes }).await.is_err() {
                    break;
                }
            }
        }
    }
    Ok(())
}

/// The changes gathered since the last batch went out, and when the batch began and last grew.
#[derive(Debug, Default)]
struct Batch {
    changes: BTreeSet<Change>,
    first: Option<Instant>,
    latest: Option<Instant>,
}

impl Batch {
    fn push(&mut self, change: Change) {
        let now = Instant::now();
        self.first.get_or_insert(now);
        self.latest = Some(now);
        self.changes.insert(change);
    }

    /// When the batch goes out: after a quiet spell of [`QUIET`], and no later than [`LONGEST`] after it
    /// began. `None` while the batch is empty.
    fn deadline(&self) -> Option<Instant> {
        let (first, latest) = (self.first?, self.latest?);
        Some((latest + QUIET).min(first + LONGEST))
    }

    /// Takes the batch's changes, leaving an empty batch behind.
    fn take(&mut self) -> Vec<Change> {
        let changes = coalesce(std::mem::take(&mut self.changes));
        *self = Self::default();
        changes
    }
}

/// Whether an event can mean a file was written or removed. Reads do not count, so the watch never reacts
/// to its own reads. Closing a file after writing it does count, since that is when a capture is complete.
fn relevant(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::Create(_) | EventKind::Remove(_) | EventKind::Modify(_) | EventKind::Access(AccessKind::Close(AccessMode::Write))
    )
}

/// The path an event names, written under `folder` as the page knows the folder. Some systems report a path
/// under the folder's resolved location, so a path under `canonical` moves back under `folder`.
fn rebase(folder: &Path, canonical: &Path, path: &Path) -> PathBuf {
    match path.strip_prefix(canonical) {
        Ok(relative) if folder != canonical => folder.join(relative),
        _ => path.to_path_buf(),
    }
}

/// Whether `path` is one of the two folders that the watch follows, directly inside `folder`.
fn is_folder(folder: &Path, path: &Path) -> bool {
    path.parent() == Some(folder) && FOLDERS.iter().any(|name| path.file_name() == Some(OsStr::new(name)))
}

/// Watches `path` recursively while it exists, and stops watching it once it is gone.
fn follow(watcher: &mut RecommendedWatcher, path: &Path, followed: &mut HashSet<PathBuf>) {
    if path.is_dir() {
        if !followed.contains(path) && watcher.watch(path, RecursiveMode::Recursive).is_ok() {
            followed.insert(path.to_path_buf());
        }
    } else if followed.remove(path) {
        // The watch may already have gone with the folder.
        let _ = watcher.unwatch(path);
    }
}

fn is_png(path: &Path) -> bool {
    path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("png"))
}

#[cfg(test)]
#[path = "../../../../tests/app/pages/instance/watch.rs"]
mod tests;
