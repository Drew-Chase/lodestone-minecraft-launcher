//! The skin previews: each skin in an open section, drawn in 3D in the pose the viewer shows, at the moment of its motion on
//! show, with the layers on show and the cape the account wears. The poses move, so the previews are drawn again at each
//! frame. They are drawn on the CPU in a background task, one drawing at a time, so the list stays responsive.

use std::sync::{Arc, OnceLock};

use iced::widget::image::Handle;
use image::RgbaImage;

use super::catalog::BUNDLED;
use super::viewer::{Layers, Model, Pose, Skin, atlas, decode_cape, raster, skin_texels};

/// The size a preview is drawn at: twice the 96 by 128 it is shown at, so it stays sharp on a dense screen.
pub const WIDTH: u32 = 192;
pub const HEIGHT: u32 = 256;

/// One saved or archived skin to preview: its texels, and whether it wears the slim model. `None` texels draw nothing.
#[derive(Debug, Clone)]
pub struct Item {
    pub texels: Option<Arc<Vec<u8>>>,
    pub slim: bool,
}

/// One drawing of the previews: which skins of each group to draw, the pose and the moment of it, the layers, and the cape
/// the account wears. Each skin names its place in its group, so the drawing lands where it belongs.
#[derive(Debug, Clone)]
pub struct Job {
    /// Which drawing this is. Drawings are answered in the order they were asked for.
    pub generation: u64,
    pub pose: Pose,
    /// The moment of the pose's motion that the drawing shows, in seconds.
    pub time: f32,
    pub layers: Layers,
    pub cape: Option<Vec<u8>>,
    /// How many skins each group holds, so that the drawing has a place for every one.
    pub saved_len: usize,
    pub archived_len: usize,
    /// The skins to draw, each with its place in its group. A bundled skin is named by its place in the catalog.
    pub saved: Vec<(usize, Item)>,
    pub archived: Vec<(usize, Item)>,
    pub bundled: Vec<(usize, bool)>,
}

/// The drawn previews, in the order of each group. A skin that was not drawn, or cannot be, has none.
#[derive(Debug, Clone, Default)]
pub struct Previews {
    pub saved: Vec<Option<Handle>>,
    pub archived: Vec<Option<Handle>>,
    pub bundled: Vec<Option<Handle>>,
}

/// What every preview of a drawing shares: the pose and its moment, the layers, and the cape.
struct Look<'a> {
    pose: Pose,
    layers: Layers,
    time: f32,
    cape: Option<&'a RgbaImage>,
}

impl Look<'_> {
    /// Draws one skin, given its texels and model, as a preview.
    fn draw(&self, texels: &[u8], slim: bool) -> Option<Handle> {
        let skin = Skin {
            id: 0,
            texels: atlas(texels, self.cape),
            cape: self.cape.is_some(),
        };
        let model = Model {
            skin: Some(Arc::new(skin)),
            pose: self.pose,
            layers: self.layers,
            slim,
            time: self.time,
        };
        Some(Handle::from_rgba(WIDTH, HEIGHT, raster::render(&model, WIDTH, HEIGHT)))
    }
}

/// Draws the skins of a job in its pose, and returns them with the job's generation.
pub fn draw(job: Job) -> (u64, Previews) {
    let cape = job.cape.as_deref().and_then(decode_cape);
    let look = Look {
        pose: job.pose,
        layers: job.layers,
        time: job.time,
        cape: cape.as_ref(),
    };
    let mut previews = Previews {
        saved: vec![None; job.saved_len],
        archived: vec![None; job.archived_len],
        bundled: vec![None; BUNDLED.len()],
    };
    for (index, item) in &job.saved {
        previews.saved[*index] = item.texels.as_deref().and_then(|texels| look.draw(texels, item.slim));
    }
    for (index, item) in &job.archived {
        previews.archived[*index] = item.texels.as_deref().and_then(|texels| look.draw(texels, item.slim));
    }
    for (index, slim) in &job.bundled {
        previews.bundled[*index] = bundled_texels(*index).and_then(|texels| look.draw(texels, *slim));
    }
    (job.generation, previews)
}

/// The texels of the bundled skin at `index`. Every bundled skin is decoded the first time one is drawn, and then kept.
fn bundled_texels(index: usize) -> Option<&'static [u8]> {
    static TEXELS: OnceLock<Vec<Option<Vec<u8>>>> = OnceLock::new();
    TEXELS
        .get_or_init(|| BUNDLED.iter().map(|skin| skin_texels(skin.png)).collect())
        .get(index)?
        .as_deref()
}

#[cfg(test)]
#[path = "../../../../tests/app/pages/skins/previews.rs"]
mod tests;
