//! Screenshots tab — the instance's captures as a three-column grid of footer
//! tiles. Each thumbnail decodes in the background and fills its tile when it
//! lands; until then the tile shows a placeholder. A press on a tile opens the
//! viewer, and a right-click offers the capture's menu.

mod menu;
mod strip;
mod viewer;

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};

use iced::widget::image::Handle;
use iced::widget::{Text, mouse_area};
use iced::{Element, Size, mouse};

use super::files::ScreenshotFile;
use crate::app::components::grid::grid;
use crate::app::components::panel::panel;
use crate::app::components::screenshot_tile::{Caption, photo_tile};
use crate::app::lang;
use crate::app::theme::{color, fonts};

pub use menu::Action;
pub use viewer::{Move, key_event};

/// Tiles per row.
const COLUMNS: usize = 3;
/// Space between tiles, per the reference.
const GAP: f32 = 14.0;
/// About 161px of art over a 34px footer, per the reference.
const TILE_HEIGHT: f32 = 195.0;
/// The box a thumbnail is scaled down to. Sixteen by nine keeps the art sharp
/// at the grid's width without holding full-size captures in memory.
pub const THUMBNAIL: (u32, u32) = (480, 270);
/// The box a capture is scaled down to in the viewer: a 1440p screen's pixels.
/// Copying the image uses the original.
pub const FULL: (u32, u32) = (2560, 1440);

/// Messages the Screenshots tab emits and handles.
#[derive(Debug, Clone)]
pub enum Message {
    /// Shows the capture at this index in the viewer, opening the viewer if it is closed.
    Show(usize),
    /// Closes the viewer.
    Close,
    /// Moves the viewer to another capture.
    Move(Move),
    /// A key the viewer acts on for its capture. The page turns it into a [`Message::Menu`].
    Key(Key),
    /// A capture's menu row, or one of the viewer's buttons, was picked. The root carries it out. A
    /// separator sends `None`.
    Menu { path: PathBuf, action: Option<Action> },
    /// The user answered the delete alert for the capture at `path`. The root trashes it on a yes.
    DeleteAnswered { path: PathBuf, confirmed: bool },
    /// The capture at `path` went to the trash, or could not.
    Deleted { path: PathBuf, result: Result<(), String> },
    /// The viewer's image was copied to the clipboard, or could not be.
    Copied(Result<(), String>),
    /// The capture was handed to its own app, or could not be.
    Opened(Result<(), String>),
    /// A capture's full-size image was decoded, or could not be (`None`). Stale when `generation` has
    /// moved on.
    Full {
        generation: u64,
        path: PathBuf,
        handle: Option<Handle>,
    },
}

/// The keys the viewer acts on for its capture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    /// Delete: asks before the capture goes to the trash.
    Delete,
    /// Ctrl+C: copies the image to the clipboard.
    Copy,
}

/// The Screenshots tab's state: the captures, most recent first, their thumbnails, and the viewer's
/// place among them with its full-size images.
#[derive(Debug, Default)]
pub struct State {
    shots: Vec<ScreenshotFile>,
    /// Captures whose thumbnails have not been decoded yet, in display order.
    pending: VecDeque<PathBuf>,
    /// Decoded thumbnails by file path.
    thumbnails: HashMap<PathBuf, Handle>,
    /// The capture the viewer shows, as an index into `shots`. `None` while the viewer is closed.
    viewer: Option<usize>,
    /// Full-size images by file path. Only the viewer's capture and its neighbours are kept.
    full: HashMap<PathBuf, Handle>,
    /// Full-size decodes in flight, so a step does not start the same one twice.
    decoding: HashSet<PathBuf>,
    /// Whether a thumbnail is decoding now. Thumbnails decode one at a time.
    decoding_thumbnail: bool,
}

impl State {
    /// Replaces the captures with `shots`, most recent first. A capture whose file has not changed keeps
    /// its thumbnail and its full-size image; the rest decode again. The viewer stays on its capture while
    /// that capture is still listed, and otherwise moves to the capture that now sits at its place.
    pub fn set_shots(&mut self, shots: Vec<ScreenshotFile>) {
        let current = self.current_path();
        let shown = self.viewer;
        let unchanged: HashSet<PathBuf> = {
            let before: HashMap<&Path, &ScreenshotFile> = self.shots.iter().map(|shot| (shot.path.as_path(), shot)).collect();
            shots
                .iter()
                .filter(|shot| before.get(shot.path.as_path()).is_some_and(|old| **old == **shot))
                .map(|shot| shot.path.clone())
                .collect()
        };
        self.thumbnails.retain(|path, _| unchanged.contains(path));
        self.full.retain(|path, _| unchanged.contains(path));
        self.decoding.retain(|path| unchanged.contains(path));
        self.pending = shots
            .iter()
            .filter(|shot| !self.thumbnails.contains_key(&shot.path))
            .map(|shot| shot.path.clone())
            .collect();
        self.shots = shots;
        self.viewer = current
            .and_then(|path| self.shots.iter().position(|shot| shot.path == path))
            .or_else(|| shown.filter(|_| !self.shots.is_empty()).map(|index| index.min(self.shots.len() - 1)));
    }

    /// The captures, most recent first.
    pub fn shots(&self) -> &[ScreenshotFile] {
        &self.shots
    }

    /// The thumbnail of the capture at `path`, once it has decoded.
    pub fn thumbnail(&self, path: &Path) -> Option<&Handle> {
        self.thumbnails.get(path)
    }

    /// The image to show for the capture at `path`: its full-size decode once it has landed, else its
    /// thumbnail.
    pub fn picture(&self, path: &Path) -> Option<&Handle> {
        self.full.get(path).or_else(|| self.thumbnails.get(path))
    }

    /// The next capture still waiting for its thumbnail. Nothing comes out while another thumbnail is
    /// decoding, so one decode chain runs at a time.
    pub fn next_pending(&mut self) -> Option<PathBuf> {
        if self.decoding_thumbnail {
            return None;
        }
        let path = self.pending.pop_front()?;
        self.decoding_thumbnail = true;
        Some(path)
    }

    /// Records a decoded thumbnail and frees the decoder for the next one. A capture that would not decode
    /// keeps its placeholder.
    pub fn store(&mut self, path: PathBuf, handle: Option<Handle>) {
        self.decoding_thumbnail = false;
        if let Some(handle) = handle {
            self.thumbnails.insert(path, handle);
        }
    }

    /// The index of the capture the viewer shows, while the viewer is open.
    pub fn viewing(&self) -> Option<usize> {
        self.viewer
    }

    /// Opens the viewer on the capture at `index`. An index past the last capture is ignored.
    pub fn show(&mut self, index: usize) {
        if index < self.shots.len() {
            self.viewer = Some(index);
        }
    }

    /// Closes the viewer and lets go of its full-size images.
    pub fn close(&mut self) {
        self.viewer = None;
        self.full.clear();
        self.decoding.clear();
    }

    /// Moves the viewer to `movement`. Does nothing while it is closed.
    pub fn move_viewer(&mut self, movement: Move) {
        if let Some(current) = self.viewer {
            self.viewer = Some(viewer::moved(current, self.shots.len(), movement));
        }
    }

    /// The path of the capture the viewer shows.
    pub fn current_path(&self) -> Option<PathBuf> {
        self.viewer.and_then(|index| self.shots.get(index)).map(|shot| shot.path.clone())
    }

    /// The captures the viewer needs at full size: the one it shows first, then its neighbours.
    fn window_paths(&self) -> Vec<PathBuf> {
        let Some(current) = self.viewer else {
            return Vec::new();
        };
        viewer::needed(current, self.shots.len())
            .into_iter()
            .map(|index| self.shots[index].path.clone())
            .collect()
    }

    /// The full-size decodes the viewer still needs, marked as in flight. Empty when every one is
    /// decoded or already on its way.
    pub fn begin_decodes(&mut self) -> Vec<PathBuf> {
        let paths: Vec<PathBuf> = self
            .window_paths()
            .into_iter()
            .filter(|path| !self.full.contains_key(path) && !self.decoding.contains(path))
            .collect();
        self.decoding.extend(paths.iter().cloned());
        paths
    }

    /// Records a full-size decode. A decode the viewer no longer needs is dropped, as is one that
    /// failed; the images the viewer has moved past are released.
    pub fn store_full(&mut self, path: PathBuf, handle: Option<Handle>) {
        self.decoding.remove(&path);
        let wanted = self.window_paths();
        self.full.retain(|kept, _| wanted.contains(kept));
        if let Some(handle) = handle
            && wanted.contains(&path)
        {
            self.full.insert(path, handle);
        }
    }

    /// Drops the capture at `path` from the tab, with its images. The viewer moves to the capture that
    /// takes its place, or the one before it when it was the last, and closes when none is left.
    pub fn remove(&mut self, path: &Path) {
        let Some(index) = self.shots.iter().position(|shot| shot.path.as_path() == path) else {
            return;
        };
        self.shots.remove(index);
        self.pending.retain(|pending| pending.as_path() != path);
        self.thumbnails.remove(path);
        self.full.remove(path);
        self.decoding.remove(path);
        self.viewer = viewer::after_removal(self.viewer, index, self.shots.len());
    }

    /// The viewer over the window, or nothing while it is closed.
    pub fn overlay(&self, window: Size) -> Option<Element<'_, Message>> {
        let index = self.viewer?;
        Some(viewer::view(self, index, window))
    }
}

/// The grid of screenshot tiles.
pub fn view(state: &State) -> Element<'_, Message> {
    if state.shots.is_empty() {
        return panel(
            Text::new(lang::t("instance.screenshots.empty"))
                .size(12)
                .font(fonts::Inter::REGULAR)
                .color(color::Text::MUTED),
        );
    }

    let tiles = (0..state.shots.len())
        .map(|index| {
            let shot = &state.shots[index];
            clickable(
                state,
                index,
                Caption::Footer {
                    name: &shot.file_name,
                    size: &shot.size_label,
                },
                TILE_HEIGHT,
            )
        })
        .collect();

    grid(tiles, COLUMNS, GAP)
}

/// The tile of the capture at `index`, with its thumbnail under `caption`. A press opens the viewer
/// on the capture, and a right-click offers its menu. The Overview strip uses it too.
pub fn clickable<'a>(state: &'a State, index: usize, caption: Caption<'a>, height: f32) -> Element<'a, Message> {
    let shot = &state.shots[index];
    let tile = mouse_area(photo_tile(state.thumbnail(&shot.path), caption, height))
        .interaction(mouse::Interaction::Pointer)
        .on_press(Message::Show(index));
    menu::attach(tile, &shot.path)
}

#[cfg(test)]
#[path = "../../../../../tests/app/pages/instance/screenshots/mod.rs"]
mod tests;
