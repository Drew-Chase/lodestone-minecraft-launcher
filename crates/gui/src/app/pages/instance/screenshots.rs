//! Screenshots tab — the instance's captures as a three-column grid of
//! footer tiles. The tab has no controls yet, so the view takes any message
//! type.

use iced::Element;

use crate::app::components::grid::grid;
use crate::app::components::screenshot_tile::{Caption, screenshot_tile};
use crate::app::pages::library::instance::{Biome, Instance};

use super::demo;

/// Tiles per row.
const COLUMNS: usize = 3;
/// Space between tiles, per the reference.
const GAP: f32 = 14.0;
/// About 161px of art over a 34px footer, per the reference.
const TILE_HEIGHT: f32 = 195.0;

/// One captured screenshot, ready to draw.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Screenshot {
    pub file_name: String,
    /// The file size as displayed, e.g. "2.1 MB".
    pub size_label: String,
    pub biome: Biome,
    pub art_seed: u64,
}

/// The Screenshots tab's state: the captures, newest first.
#[derive(Debug, Default)]
pub struct State {
    shots: Vec<Screenshot>,
}

impl State {
    pub fn for_instance(instance: &Instance) -> Self {
        Self {
            shots: demo::screenshots(instance),
        }
    }
}

/// The grid of screenshot tiles.
pub fn view<'a, M: 'a>(state: &'a State) -> Element<'a, M> {
    let tiles = state
        .shots
        .iter()
        .map(|shot| {
            screenshot_tile(
                shot.biome,
                shot.art_seed,
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
