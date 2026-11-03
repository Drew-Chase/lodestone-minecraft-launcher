//! The skin previews: each skin in an open section, drawn in 3D in the pose the viewer shows. The poses move, so the previews are
//! cached a frame at a time. A loop of a pose is drawn once, frame by frame, with each skin on the page drawn once for each
//! frame. After that the loop plays from the cache, and nothing is drawn. The skins of a frame share one projection of the model
//! for each arm width. The frames are drawn on the CPU in a background task, one drawing at a time, so the list stays responsive.

use std::sync::{Arc, OnceLock};
use std::time::Duration;

use iced::widget::image::Handle;

use super::catalog::BUNDLED;
use super::viewer::{Layers, Model, Pose, atlas, decode_cape, raster, skin_texels};

/// The size a preview is drawn at: twice the 96 by 128 it is shown at, so it stays sharp on a dense screen.
pub const WIDTH: u32 = 192;
pub const HEIGHT: u32 = 256;

/// How often the previews take a step through a pose's motion, which is one frame of the cache.
pub const FRAME: Duration = Duration::from_millis(66);

/// The most frames a loop is cached as. A walk and a run are cached one frame per step, and a slow pose in this many, so that
/// the cache of a loop is never bigger than a walk's.
pub const MAX_FRAMES_PER_LOOP: usize = 15;

/// Where a previewed skin stands on the page: a saved skin, an archived one, or a bundled one, by its place in its list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Slot {
    Saved(usize),
    Archived(usize),
    Bundled(usize),
}

/// One skin to draw in a frame: its texels, and the model it wears. A bundled skin has no texels here, since the drawing takes
/// it from the catalog.
#[derive(Debug, Clone)]
pub struct Item {
    pub texels: Option<Arc<Vec<u8>>>,
    pub slim: bool,
}

/// One frame of the previews that is missing from the cache: the skins to draw in it, and the pose, the layers and the cape it
/// is drawn with.
#[derive(Debug, Clone)]
pub struct Job {
    /// Which look of the previews the frame belongs to. A drawing made for an older look is dropped.
    pub generation: u64,
    /// The frame of the loop, which the cache keys it by.
    pub frame: usize,
    pub pose: Pose,
    /// The moment of the pose's motion the frame shows, in seconds.
    pub time: f32,
    pub layers: Layers,
    pub cape: Option<Vec<u8>>,
    pub items: Vec<(Slot, Item)>,
}

/// The frame a drawing made, with each drawn skin's slot. A skin that cannot be drawn has no entry.
#[derive(Debug, Clone)]
pub struct Drawn {
    pub generation: u64,
    pub frame: usize,
    pub previews: Vec<(Slot, Handle)>,
}

/// How many frames a loop of the pose is cached as: one per step of the clock where the loop is short enough, and at most
/// `MAX_FRAMES_PER_LOOP` where it is not.
pub fn frames_per_loop(pose: Pose) -> usize {
    let per_step = (pose.loop_seconds() / FRAME.as_secs_f32()).round() as usize;
    per_step.clamp(1, MAX_FRAMES_PER_LOOP)
}

/// The moment of the pose's motion that frame `frame` of its loop shows.
pub fn frame_time(pose: Pose, frame: usize) -> f32 {
    pose.loop_seconds() * frame as f32 / frames_per_loop(pose) as f32
}

/// The frame of the pose's loop that shows the moment `time`: the frame whose moment is nearest. The loop repeats, so the frame
/// wraps round to the start.
pub fn frame_at(pose: Pose, time: f32) -> usize {
    let frames = frames_per_loop(pose);
    let step = pose.loop_seconds() / frames as f32;
    ((time / step).round() as usize) % frames
}

/// Draws the items of a job, and returns them with the job's generation and frame. The skins of one model are drawn from one
/// projection, so the model is projected once for each arm width.
pub fn draw(job: Job) -> Drawn {
    let cape = job.cape.as_deref().and_then(decode_cape);
    let mut projections: [Option<raster::Projection>; 2] = [None, None];
    let previews = job
        .items
        .iter()
        .filter_map(|(slot, item)| {
            let texels = texels_of(*slot, item)?;
            let projection = projections[usize::from(item.slim)].get_or_insert_with(|| {
                let model = Model {
                    skin: None,
                    pose: job.pose,
                    layers: job.layers,
                    slim: item.slim,
                    time: job.time,
                };
                raster::project(&model, cape.is_some(), WIDTH, HEIGHT)
            });
            let texture = atlas(texels, cape.as_ref());
            Some((*slot, Handle::from_rgba(WIDTH, HEIGHT, raster::fill(projection, &texture))))
        })
        .collect();
    Drawn {
        generation: job.generation,
        frame: job.frame,
        previews,
    }
}

/// The texels a slot draws: a saved or archived skin's own, or a bundled skin's from the catalog.
fn texels_of(slot: Slot, item: &Item) -> Option<&[u8]> {
    match slot {
        Slot::Bundled(index) => bundled_texels(index),
        Slot::Saved(_) | Slot::Archived(_) => item.texels.as_deref().map(Vec::as_slice),
    }
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
