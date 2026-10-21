//! Instance artwork — the per-biome scene illustrations from
//! `res/artwork/scenes` (base scene plus two variants each), embedded into
//! the binary and served as cached svg handles.
//!
//! The variant is picked by the instance's `art_seed`, so the grid mixes
//! scenes without any runtime generation.

use std::sync::OnceLock;

use crate::app::pages::library::instance::Biome;
use iced::widget::svg::Handle;

/// Every scene file, grouped in [`Biome`] declaration order: base scene,
/// variant 1, variant 2. `slot(biome) * 3 + (seed % 3)` indexes it.
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

/// The embedded handles, built once. One `Handle` per file so iced_wgpu's
/// svg cache keys stay stable — re-parsing per call would re-rasterize on
/// every frame (its cache keys on the handle id and drawn size).
static HANDLES: OnceLock<Vec<Handle>> = OnceLock::new();

/// The hero variants of the artwork — the scene clipped to the banner's
/// rounded corners with the left readability gradient baked in. The fade
/// must live in the texture (iced_wgpu renders quads beneath svg textures,
/// so an overlay container would vanish behind the art), and translucent
/// svg stops render ≈ squared through the texture pipeline, so the target
/// alphas are sqrt-compensated here (0.80 → 0.894, 0.30 → 0.548).
static HERO_HANDLES: OnceLock<Vec<Handle>> = OnceLock::new();

/// The scene artwork for an instance — deterministic per `(biome, seed)`:
/// the seed selects the base scene or one of its two variants.
pub fn scene_handle(biome: Biome, seed: u64) -> Handle {
    let handles = HANDLES.get_or_init(|| SCENES.iter().map(|(_, svg)| Handle::from_memory(svg.as_bytes().to_vec())).collect());
    handles[slot(biome) * 3 + (seed % 3) as usize].clone()
}

/// The hero-banner artwork — the same scene, rounded and left-faded.
pub fn hero_handle(biome: Biome, seed: u64) -> Handle {
    let handles = HERO_HANDLES.get_or_init(|| {
        SCENES
            .iter()
            .map(|(_, svg)| Handle::from_memory(hero_wrapped(svg).into_bytes()))
            .collect()
    });
    handles[slot(biome) * 3 + (seed % 3) as usize].clone()
}

/// Wraps a scene file's content in the hero treatment: a rounded-corner
/// clip (12px in the 800×450 viewBox ≈ the banner's 18px radius at the
/// default window) plus the left-dark fade. The C2PA provenance blob is
/// stripped — it carries nothing visual and its namespace prefix dies with
/// the outer tag (an undefined prefix makes the wrapped doc unparseable).
/// The banner's own border still draws on the container; the transparent
/// corners let the page show.
fn hero_wrapped(scene: &str) -> String {
    let inner = scene.split_once('>').map(|(_, rest)| rest).unwrap_or(scene);
    let inner = inner.trim_end();
    let inner = inner.strip_suffix("</svg>").unwrap_or(inner);
    let inner = match (inner.find("<metadata>"), inner.find("</metadata>")) {
        (Some(start), Some(end)) => {
            let mut without = String::with_capacity(inner.len());
            without.push_str(&inner[..start]);
            without.push_str(&inner[end + "</metadata>".len()..]);
            without
        }
        _ => inner.to_string(),
    };
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="800" height="450" viewBox="0 0 800 450">
  <defs>
    <clipPath id="banner"><rect width="800" height="450" rx="12"/></clipPath>
    <linearGradient id="fade" x1="0" y1="0" x2="1" y2="0">
      <stop offset="0" stop-color="#06080a" stop-opacity="0.92"/>
      <stop offset="0.4" stop-color="#06080a" stop-opacity="0.49"/>
      <stop offset="0.65" stop-color="#06080a" stop-opacity="0"/>
    </linearGradient>
  </defs>
  <g clip-path="url(#banner)">
{inner}
    <rect width="800" height="450" fill="url(#fade)"/>
  </g>
</svg>"##
    )
}
