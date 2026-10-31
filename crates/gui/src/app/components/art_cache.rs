//! The library's scene art, rendered to pixels in the background. iced rasterises an SVG while it draws,
//! and it evicts the raster it has not drawn lately, so an SVG card was rasterised again on the UI thread
//! each time the library came back. Here each picture is rendered once on the blocking pool, and the views
//! draw the finished bitmap. The root keeps the cache; the library names the pictures its rows need.

use std::collections::{HashMap, HashSet};

use iced::Task;
use iced::widget::image::Handle;
use resvg::{tiny_skia, usvg};

use super::scene_art::{SIZE_STEP, card_wrapped, hero_wrapped, scene_text};
use crate::app::pages::library::instance::Biome;
use crate::app::perf;

/// Which picture a key renders.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    /// A grid card's banner, rounded at the top.
    Card,
    /// A compact row's square thumbnail.
    Thumb,
    /// The library hero's banner, rounded, with its left fade.
    Hero,
}

/// One picture: its kind, its scene, and the box it fills, in whole pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArtKey {
    kind: Kind,
    biome: Biome,
    seed: u64,
    width: u32,
    height: u32,
}

impl ArtKey {
    /// The key for `kind` of the scene `(biome, seed)` in a `width`×`height` box. The width snaps to the
    /// scene wrappers' step, so a box that moves by a pixel keeps its picture.
    pub fn new(kind: Kind, biome: Biome, seed: u64, width: f32, height: f32) -> Self {
        Self {
            kind,
            biome,
            seed,
            width: ((width / SIZE_STEP).round().max(1.0) * SIZE_STEP) as u32,
            height: height.round().max(1.0) as u32,
        }
    }

    /// Whether `other` is the same picture at another width, which can stand in for it while it renders.
    fn same_scene(&self, other: &ArtKey) -> bool {
        self.kind == other.kind && self.biome == other.biome && self.seed == other.seed && self.height == other.height
    }
}

/// The rendered pictures, and the renders still running. The root keeps one; the library reads it while it
/// draws, and each render comes back as a message.
#[derive(Default)]
pub struct ArtCache {
    ready: HashMap<ArtKey, Handle>,
    pending: HashSet<ArtKey>,
    failed: HashSet<ArtKey>,
}

impl ArtCache {
    /// The picture for `key`. While `key` renders, the nearest rendered width of the same scene stands in, so
    /// a resize shows art at once. `None` only until something of that scene has rendered.
    pub fn get(&self, key: ArtKey) -> Option<Handle> {
        if let Some(handle) = self.ready.get(&key) {
            return Some(handle.clone());
        }
        self.ready
            .iter()
            .filter(|(known, _)| known.same_scene(&key))
            .min_by_key(|(known, _)| known.width.abs_diff(key.width))
            .map(|(_, handle)| handle.clone())
    }

    /// Drops every picture and render that `wanted` does not name, so a resize or a scroll does not keep the
    /// sizes it passed through.
    pub fn retain(&mut self, wanted: &HashSet<ArtKey>) {
        self.ready.retain(|key, _| wanted.contains(key));
        self.pending.retain(|key| wanted.contains(key));
        self.failed.retain(|key| wanted.contains(key));
    }

    /// The keys among `wanted` that are neither rendered, rendering, nor failed, each once.
    pub fn missing(&self, wanted: &[ArtKey]) -> Vec<ArtKey> {
        let mut seen = HashSet::new();
        wanted
            .iter()
            .copied()
            .filter(|key| !self.ready.contains_key(key) && !self.pending.contains(key) && !self.failed.contains(key))
            .filter(|key| seen.insert(*key))
            .collect()
    }

    /// Starts rendering `keys`. Each render comes back through `done` once it finishes.
    pub fn request<M: Send + 'static>(&mut self, keys: Vec<ArtKey>, done: fn(ArtKey, Option<Handle>) -> M) -> Task<M> {
        let tasks: Vec<Task<M>> = keys
            .into_iter()
            .map(|key| {
                self.pending.insert(key);
                Task::perform(
                    async move { tokio::task::spawn_blocking(move || render(key)).await.ok().flatten() },
                    move |handle| done(key, handle),
                )
            })
            .collect();
        Task::batch(tasks)
    }

    /// Records a finished render. A render whose key was dropped while it ran is ignored.
    pub fn finish(&mut self, key: ArtKey, handle: Option<Handle>) {
        if !self.pending.remove(&key) {
            return;
        }
        match handle {
            Some(handle) => {
                self.ready.insert(key, handle);
            }
            None => {
                self.failed.insert(key);
            }
        }
    }
}

/// Renders the picture for `key`. It runs on the blocking pool.
fn render(key: ArtKey) -> Option<Handle> {
    let _scope = perf::scope("art.render");
    perf::count("art.rendered", 1);
    let scene = scene_text(key.biome, key.seed);
    let svg = match key.kind {
        Kind::Card => card_wrapped(scene, key.width as f32, key.height as f32),
        Kind::Hero => hero_wrapped(scene, key.width as f32, key.height as f32),
        Kind::Thumb => scene.to_owned(),
    };
    rasterise(&svg, key.width, key.height)
}

/// Draws `svg` over a `width`×`height` pixmap, scaled and centred the way an image covers its box, and
/// returns the pixels as straight-alpha RGBA.
fn rasterise(svg: &str, width: u32, height: u32) -> Option<Handle> {
    let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).ok()?;
    let size = tree.size();
    let scale = (width as f32 / size.width()).max(height as f32 / size.height());
    let x = (width as f32 - size.width() * scale) / 2.0;
    let y = (height as f32 - size.height() * scale) / 2.0;
    let mut pixmap = tiny_skia::Pixmap::new(width, height)?;
    resvg::render(&tree, tiny_skia::Transform::from_row(scale, 0.0, 0.0, scale, x, y), &mut pixmap.as_mut());
    let mut pixels = pixmap.take();
    unpremultiply(&mut pixels);
    Some(Handle::from_rgba(width, height, pixels))
}

/// tiny-skia stores premultiplied colour, and iced draws straight alpha, so each colour channel is divided
/// back out of its alpha.
fn unpremultiply(pixels: &mut [u8]) {
    for pixel in pixels.chunks_exact_mut(4) {
        let alpha = u32::from(pixel[3]);
        if alpha != 0 && alpha != 255 {
            for channel in &mut pixel[..3] {
                *channel = (u32::from(*channel) * 255 / alpha).min(255) as u8;
            }
        }
    }
}

#[cfg(test)]
#[path = "../../../tests/app/components/art_cache.rs"]
mod tests;
