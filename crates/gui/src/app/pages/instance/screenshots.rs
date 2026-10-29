//! Screenshots tab — the instance's captures as a three-column grid of footer
//! tiles. Each thumbnail decodes in the background and fills its tile when it
//! lands; until then the tile shows a placeholder. The tab has no controls yet,
//! so the view takes any message type.

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};

use iced::Element;
use iced::widget::Text;
use iced::widget::image::Handle;

use super::files::ScreenshotFile;
use crate::app::components::grid::grid;
use crate::app::components::panel::panel;
use crate::app::components::screenshot_tile::{Caption, photo_tile};
use crate::app::theme::{color, fonts};

/// Tiles per row.
const COLUMNS: usize = 3;
/// Space between tiles, per the reference.
const GAP: f32 = 14.0;
/// About 161px of art over a 34px footer, per the reference.
const TILE_HEIGHT: f32 = 195.0;
/// The box a thumbnail is scaled down to. Sixteen by nine keeps the art sharp
/// at the grid's width without holding full-size captures in memory.
pub const THUMBNAIL: (u32, u32) = (480, 270);

/// The Screenshots tab's state: the captures, most recent first, and their
/// thumbnails.
#[derive(Debug, Default)]
pub struct State {
    shots: Vec<ScreenshotFile>,
    /// Captures whose thumbnails have not been decoded yet, in display order.
    pending: VecDeque<PathBuf>,
    /// Decoded thumbnails by file path.
    thumbnails: HashMap<PathBuf, Handle>,
}

impl State {
    /// Replaces the captures. Their thumbnails start over.
    pub fn set_shots(&mut self, shots: Vec<ScreenshotFile>) {
        self.pending = shots.iter().map(|shot| shot.path.clone()).collect();
        self.thumbnails.clear();
        self.shots = shots;
    }

    /// The captures, most recent first.
    pub fn shots(&self) -> &[ScreenshotFile] {
        &self.shots
    }

    /// The thumbnail of the capture at `path`, once it has decoded.
    pub fn thumbnail(&self, path: &Path) -> Option<&Handle> {
        self.thumbnails.get(path)
    }

    /// The next capture still waiting for its thumbnail.
    pub fn next_pending(&mut self) -> Option<PathBuf> {
        self.pending.pop_front()
    }

    /// Records a decoded thumbnail. A capture that would not decode keeps its
    /// placeholder.
    pub fn store(&mut self, path: PathBuf, handle: Option<Handle>) {
        if let Some(handle) = handle {
            self.thumbnails.insert(path, handle);
        }
    }
}

/// The grid of screenshot tiles.
pub fn view<'a, M: 'a>(state: &'a State) -> Element<'a, M> {
    if state.shots.is_empty() {
        return panel(
            Text::new("No screenshots yet. Captures taken in the game appear here.")
                .size(12)
                .font(fonts::Inter::REGULAR)
                .color(color::Text::MUTED),
        );
    }

    let tiles = state
        .shots
        .iter()
        .map(|shot| {
            photo_tile(
                state.thumbnail(&shot.path),
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
