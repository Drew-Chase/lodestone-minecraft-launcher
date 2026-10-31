//! Instance artwork — the per-biome scene illustrations from
//! `res/artwork/scenes` (base scene plus two variants each), embedded into
//! the binary and served as cached svg handles.
//!
//! The variant is picked by the instance's `art_seed`, so the grid mixes
//! scenes without any runtime generation. The masthead, the grid card and the
//! screenshot thumbnails are the exception: they are wrapped at the exact pixel
//! size they are drawn at, so the art is never stretched and their rounded
//! corners stay circular. Those wrappers are cached per size.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex, OnceLock, PoisonError};

use crate::app::pages::library::instance::Biome;
use crate::app::perf;
use iced::Size;
use iced::widget::svg::Handle;

/// Every scene file, grouped in [`Biome`] declaration order: base scene,
/// variant 1, variant 2. [`scene_index`] indexes it.
const SCENES: [(Biome, &str); 24] = [
    (Biome::End, include_str!("../../../../../res/artwork/scenes/scene-end.svg")),
    (Biome::End, include_str!("../../../../../res/artwork/scenes/variants/scene-end-1.svg")),
    (Biome::End, include_str!("../../../../../res/artwork/scenes/variants/scene-end-2.svg")),
    (Biome::Forest, include_str!("../../../../../res/artwork/scenes/scene-forest.svg")),
    (
        Biome::Forest,
        include_str!("../../../../../res/artwork/scenes/variants/scene-forest-1.svg"),
    ),
    (
        Biome::Forest,
        include_str!("../../../../../res/artwork/scenes/variants/scene-forest-2.svg"),
    ),
    (Biome::Cherry, include_str!("../../../../../res/artwork/scenes/scene-cherry.svg")),
    (
        Biome::Cherry,
        include_str!("../../../../../res/artwork/scenes/variants/scene-cherry-1.svg"),
    ),
    (
        Biome::Cherry,
        include_str!("../../../../../res/artwork/scenes/variants/scene-cherry-2.svg"),
    ),
    (Biome::Ocean, include_str!("../../../../../res/artwork/scenes/scene-ocean.svg")),
    (Biome::Ocean, include_str!("../../../../../res/artwork/scenes/variants/scene-ocean-1.svg")),
    (Biome::Ocean, include_str!("../../../../../res/artwork/scenes/variants/scene-ocean-2.svg")),
    (Biome::Desert, include_str!("../../../../../res/artwork/scenes/scene-desert.svg")),
    (
        Biome::Desert,
        include_str!("../../../../../res/artwork/scenes/variants/scene-desert-1.svg"),
    ),
    (
        Biome::Desert,
        include_str!("../../../../../res/artwork/scenes/variants/scene-desert-2.svg"),
    ),
    (Biome::Snow, include_str!("../../../../../res/artwork/scenes/scene-snow.svg")),
    (Biome::Snow, include_str!("../../../../../res/artwork/scenes/variants/scene-snow-1.svg")),
    (Biome::Snow, include_str!("../../../../../res/artwork/scenes/variants/scene-snow-2.svg")),
    (Biome::Nether, include_str!("../../../../../res/artwork/scenes/scene-nether.svg")),
    (
        Biome::Nether,
        include_str!("../../../../../res/artwork/scenes/variants/scene-nether-1.svg"),
    ),
    (
        Biome::Nether,
        include_str!("../../../../../res/artwork/scenes/variants/scene-nether-2.svg"),
    ),
    (Biome::Mushroom, include_str!("../../../../../res/artwork/scenes/scene-mushroom.svg")),
    (
        Biome::Mushroom,
        include_str!("../../../../../res/artwork/scenes/variants/scene-mushroom-1.svg"),
    ),
    (
        Biome::Mushroom,
        include_str!("../../../../../res/artwork/scenes/variants/scene-mushroom-2.svg"),
    ),
];

/// Every scene file is drawn in this viewBox.
const SCENE_WIDTH: f32 = 800.0;
const SCENE_HEIGHT: f32 = 450.0;
/// The moon's centre height in the scene files (about 118–128). The crops keep
/// it a third of the way down the frame, as in the reference.
const MOON_Y: f32 = 122.0;
/// Generated widths are rounded to this step, so resizing reuses handles
/// instead of building a new one for every pixel.
pub(crate) const SIZE_STEP: f32 = 4.0;
/// The grid card's art corner radius. The card's border is 14px and the art sits one pixel
/// inside it, so the art's top corners use 13px to stay concentric with the border.
pub const CARD_RADIUS: f32 = 13.0;
/// The library hero's corner radius (`--r-lg`). Its art is clipped to it in pixels, and the banner's border matches.
pub const HERO_RADIUS: f32 = 18.0;

/// The scene slot a biome occupies — must follow the [`SCENES`] grouping.
fn slot(biome: Biome) -> usize {
    match biome {
        Biome::End => 0,
        Biome::Forest => 1,
        Biome::Cherry => 2,
        Biome::Ocean => 3,
        Biome::Desert => 4,
        Biome::Snow => 5,
        Biome::Nether => 6,
        Biome::Mushroom => 7,
    }
}

/// The [`SCENES`] index for a biome and seed: the seed picks the base scene
/// or one of its two variants.
fn scene_index(biome: Biome, seed: u64) -> usize {
    slot(biome) * 3 + (seed % 3) as usize
}

/// The embedded handles, built once. One `Handle` per file so iced_wgpu's
/// svg cache keys stay stable — re-parsing per call would re-rasterize on
/// every frame (its cache keys on the handle id and drawn size).
static HANDLES: OnceLock<Vec<Handle>> = OnceLock::new();

/// Wrappers generated for the instance page, keyed by kind, scene index and
/// size. A new `Handle` per call would re-rasterise on every redraw, so each
/// size is built once.
/// A cache entry's key: the wrapper, the scene index, and the quantised width
/// and height.
type GeneratedKey = (Wrapper, usize, u32, u32);

static GENERATED: LazyLock<Mutex<HashMap<GeneratedKey, Handle>>> = LazyLock::new(Mutex::default);

/// The instance-page wrappers that are generated at a size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Wrapper {
    /// The masthead band.
    Banner,
}

/// The scene artwork for an instance — deterministic per `(biome, seed)`:
/// the seed selects the base scene or one of its two variants.
pub fn scene_handle(biome: Biome, seed: u64) -> Handle {
    let handles = HANDLES.get_or_init(|| SCENES.iter().map(|(_, svg)| Handle::from_memory(svg.as_bytes().to_vec())).collect());
    handles[scene_index(biome, seed)].clone()
}

/// The scene file's markup for a biome and seed, which the art cache wraps and rasterises.
pub(crate) fn scene_text(biome: Biome, seed: u64) -> &'static str {
    SCENES[scene_index(biome, seed)].1
}

/// The instance-page masthead at `size`, the band's exact pixel size. The
/// scene is covered to the band's width and cropped so the moon sits a third
/// of the way down. The fade into the page is the page's overlay, not baked in.
pub fn banner_handle(biome: Biome, seed: u64, size: Size) -> Handle {
    generated(Wrapper::Banner, biome, seed, size)
}

/// The cached wrapper for `wrapper` at `size`, built on first use.
fn generated(wrapper: Wrapper, biome: Biome, seed: u64, size: Size) -> Handle {
    let index = scene_index(biome, seed);
    let width = quantise(size.width);
    let height = size.height.round().max(1.0) as u32;

    let mut cache = GENERATED.lock().unwrap_or_else(PoisonError::into_inner);
    let key = (wrapper, index, width, height);
    if let Some(handle) = cache.get(&key) {
        perf::count("art.hit", 1);
        return handle.clone();
    }
    perf::count("art.miss", 1);
    let _scope = perf::scope("art.build");
    let (scene, width, height) = (SCENES[index].1, width as f32, height as f32);
    let svg = match wrapper {
        Wrapper::Banner => banner_wrapped(scene, width, height),
    };
    let handle = Handle::from_memory(svg.into_bytes());
    cache.insert(key, handle.clone());
    handle
}

/// Rounds a width to the nearest [`SIZE_STEP`], never below one step.
fn quantise(width: f32) -> u32 {
    ((width / SIZE_STEP).round().max(1.0) * SIZE_STEP) as u32
}

/// The transform that covers a `width`×`height` frame with the scene. It
/// scales the scene to the frame's width, or to its height if that is
/// larger, then crops vertically so the moon sits a third of the way down.
/// Returns `(scale, offset_x, offset_y)`: a scene unit maps to
/// `unit * scale + offset` in pixels.
fn cover(width: f32, height: f32) -> (f32, f32, f32) {
    let scale = (width / SCENE_WIDTH).max(height / SCENE_HEIGHT);
    let visible = height / scale;
    let top = (MOON_Y - visible / 3.0).clamp(0.0, (SCENE_HEIGHT - visible).max(0.0));
    (scale, (width - SCENE_WIDTH * scale) / 2.0, -top * scale)
}

/// The scene file's contents without its outer `<svg>` tag and C2PA
/// `<metadata>` blob. Keeping only the drawing nodes lets each wrapper
/// supply its own dimensions/viewBox and clip. The provenance blob must
/// go — its namespace prefix dies with the outer tag, which makes the
/// wrapped document unparseable.
fn scene_inner(scene: &str) -> String {
    let inner = scene.split_once('>').map(|(_, rest)| rest).unwrap_or(scene);
    let inner = inner.trim_end();
    let inner = inner.strip_suffix("</svg>").unwrap_or(inner);
    match (inner.find("<metadata>"), inner.find("</metadata>")) {
        (Some(start), Some(end)) => {
            let mut without = String::with_capacity(inner.len());
            without.push_str(&inner[..start]);
            without.push_str(&inner[end + "</metadata>".len()..]);
            without
        }
        _ => inner.to_string(),
    }
}

/// Wraps a scene file's content in the hero treatment at `width`×`height`: the scene covered and cropped
/// by [`cover`], clipped to a rounded rectangle of [`HERO_RADIUS`] pixels, plus the left-dark fade. The clip
/// is drawn in the frame's own units, so the crop cannot remove its corners. The banner's own border still
/// draws on the container; the transparent corners let the page show.
pub(crate) fn hero_wrapped(scene: &str, width: f32, height: f32) -> String {
    let (scale, x, y) = cover(width, height);
    let r = HERO_RADIUS;
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">
  <defs>
    <clipPath id="banner"><rect width="{width}" height="{height}" rx="{r}"/></clipPath>
    <linearGradient id="fade" x1="0" y1="0" x2="1" y2="0">
      <stop offset="0" stop-color="#06080a" stop-opacity="0.92"/>
      <stop offset="0.4" stop-color="#06080a" stop-opacity="0.49"/>
      <stop offset="0.65" stop-color="#06080a" stop-opacity="0"/>
    </linearGradient>
  </defs>
  <g clip-path="url(#banner)">
  <g transform="translate({x} {y}) scale({scale})">
{inner}
  </g>
    <rect width="{width}" height="{height}" fill="url(#fade)"/>
  </g>
</svg>"##,
        inner = scene_inner(scene),
    )
}

/// The instance-page masthead at `width`×`height`: the scene covered and
/// cropped by [`cover`]. There is no clip and no fade; the page overlays both.
fn banner_wrapped(scene: &str, width: f32, height: f32) -> String {
    let (scale, x, y) = cover(width, height);
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">
  <g transform="translate({x} {y}) scale({scale})">
{inner}
  </g>
</svg>"##,
        inner = scene_inner(scene),
    )
}

/// The grid card's art at `width`×`height`: the scene covered and cropped by [`cover`], clipped to
/// a shape with its top corners rounded by [`CARD_RADIUS`]. The bottom corners stay square, since
/// the card's information block continues below the art.
pub(crate) fn card_wrapped(scene: &str, width: f32, height: f32) -> String {
    let (scale, x, y) = cover(width, height);
    let r = CARD_RADIUS;
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">
  <defs>
    <clipPath id="card"><path d="M0,{r} A{r},{r} 0 0 1 {r},0 H{right} A{r},{r} 0 0 1 {width},{r} V{height} H0 Z"/></clipPath>
  </defs>
  <g clip-path="url(#card)">
  <g transform="translate({x} {y}) scale({scale})">
{inner}
  </g>
  </g>
</svg>"##,
        right = width - r,
        inner = scene_inner(scene),
    )
}

#[cfg(test)]
#[path = "../../../tests/app/components/scene_art.rs"]
mod tests;
