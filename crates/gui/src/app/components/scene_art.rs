//! Generated biome scene art — the instance cards' cover images.
//!
//! A Rust port of the design's `Scene.jsx`: sky gradient, sun/moon, back
//! hills, front terrain, pixel grid and vignette — emitted as a compact
//! SVG string. The variation is deterministic per `(biome, seed)` through
//! the design's sine-based pseudo-random.

use crate::app::pages::library::instance::Biome;

/// The scene's viewport — the svg widget scales it to the art box.
const VIEW_W: f32 = 640.0;
const VIEW_H: f32 = 360.0;

/// The design's sine-based pseudo-random —
/// `fract(sin((seed+1)·9301 + i·49297)·233280)`, always in `[0, 1)`.
fn prng(seed: u64, i: u64) -> f64 {
    let x = (((seed % 1000) as f64 + 1.0) * 9301.0 + i as f64 * 49297.0).sin() * 233_280.0;
    x - x.floor()
}

struct Palette {
    sky_top: &'static str,
    sky_bottom: &'static str,
    terrain_top: &'static str,
    terrain_bottom: &'static str,
    grass: Option<&'static str>,
    sun: &'static str,
}

/// The biome palettes — ported from the design CSS (`.biome-*`).
fn palette(biome: Biome) -> Palette {
    match biome {
        Biome::End => Palette {
            sky_top: "#1a0f2e",
            sky_bottom: "#5a3a8a",
            terrain_top: "#3a2a5a",
            terrain_bottom: "#0f0a1a",
            grass: None,
            sun: "#ffffff",
        },
        Biome::Forest => Palette {
            sky_top: "#0e3566",
            sky_bottom: "#1f5c2c",
            terrain_top: "#2a5a34",
            terrain_bottom: "#0e2414",
            grass: Some("#3faa5a"),
            sun: "#fff2b8",
        },
        Biome::Cherry => Palette {
            sky_top: "#5a1e48",
            sky_bottom: "#d86bac",
            terrain_top: "#7c2a5a",
            terrain_bottom: "#2a0a1e",
            grass: Some("#ffc4e1"),
            sun: "#fff2b8",
        },
        Biome::Ocean => Palette {
            sky_top: "#0a1e3a",
            sky_bottom: "#5fb4ff",
            terrain_top: "#1e5aa8",
            terrain_bottom: "#0a1e3a",
            grass: None,
            sun: "#ffffff",
        },
        Biome::Desert => Palette {
            sky_top: "#5a3e1a",
            sky_bottom: "#ffd88c",
            terrain_top: "#c48b3a",
            terrain_bottom: "#5a3e1a",
            grass: Some("#ffd88c"),
            sun: "#fff2b8",
        },
        Biome::Snow => Palette {
            sky_top: "#3a5a82",
            sky_bottom: "#a8c8e8",
            terrain_top: "#8aa8c8",
            terrain_bottom: "#e8f0fa",
            grass: Some("#ffffff"),
            sun: "#ffffff",
        },
        Biome::Nether => Palette {
            sky_top: "#3d0a08",
            sky_bottom: "#ff6b3d",
            terrain_top: "#a81f0c",
            terrain_bottom: "#3d0a08",
            grass: None,
            sun: "#ffd98a",
        },
        Biome::Mushroom => Palette {
            sky_top: "#2e1a3a",
            sky_bottom: "#e8b8d8",
            terrain_top: "#7c4b82",
            terrain_bottom: "#2e1a3a",
            grass: Some("#e8b8d8"),
            sun: "#ffffff",
        },
    }
}

/// Generates the biome scene SVG for an instance card — a deterministic
/// variation per `seed`, matching the design's `Scene.jsx` composition
/// (sky gradient, sun/moon, back hills, front terrain, grass strip, pixel
/// grid, vignette).
pub fn scene_svg(biome: Biome, seed: u64) -> String {
    let palette = palette(biome);
    let sun_x = 15.0 + prng(seed, 99) * 70.0;
    let sun_y = 18.0 + prng(seed, 100) * 20.0;

    // The jagged terrain silhouette — 18 columns, heights 30–85%.
    let cols = 18usize;
    let ridge: Vec<String> = (0..cols)
        .map(|i| {
            let height_pct = 30.0 + prng(seed, i as u64) * 55.0;
            let x = i as f64 / (cols - 1) as f64;
            format!("{:.1},{:.1}", x * VIEW_W as f64, (1.0 - height_pct / 100.0) * VIEW_H as f64)
        })
        .collect();

    // Back hills — the fixed clip-path silhouette from `Scene.jsx`.
    let back: [&str; 9] = ["0,60", "12,58", "22,65", "34,52", "48,58", "62,50", "76,56", "88,48", "100,54"];
    let back_points: Vec<String> = std::iter::once("0,360".to_string())
        .chain(back.iter().map(|point| {
            let (x, y) = point.split_once(',').unwrap();
            format!(
                "{:.1},{:.1}",
                x.parse::<f64>().unwrap() / 100.0 * VIEW_W as f64,
                y.parse::<f64>().unwrap() / 100.0 * VIEW_H as f64
            )
        }))
        .chain(std::iter::once("640,360".to_string()))
        .collect();

    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">
  <defs>
    <linearGradient id="sky" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="{sky_top}"/>
      <stop offset="1" stop-color="{sky_bottom}"/>
    </linearGradient>
    <radialGradient id="sun" cx="0.5" cy="0.5" r="0.5">
      <stop offset="0" stop-color="{sun}" stop-opacity="0.9"/>
      <stop offset="0.35" stop-color="{sun}" stop-opacity="0.45"/>
      <stop offset="1" stop-color="{sun}" stop-opacity="0"/>
    </radialGradient>
    <linearGradient id="hills" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#000000" stop-opacity="0.2"/>
      <stop offset="1" stop-color="#000000" stop-opacity="0.5"/>
    </linearGradient>
    <linearGradient id="terrain" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="{terrain_top}"/>
      <stop offset="1" stop-color="{terrain_bottom}"/>
    </linearGradient>
    <radialGradient id="vignette" cx="0.5" cy="0.5" r="0.75">
      <stop offset="0.5" stop-color="#000000" stop-opacity="0"/>
      <stop offset="1" stop-color="#000000" stop-opacity="0.45"/>
    </radialGradient>
    <pattern id="grid" width="24" height="24" patternUnits="userSpaceOnUse">
      <path d="M 24 0 L 0 0 0 24" fill="none" stroke="#000000" stroke-opacity="0.14" stroke-width="1"/>
    </pattern>
  </defs>
  <rect width="{w}" height="{h}" fill="url(#sky)"/>
  <rect x="{sun_left:.0}" y="{sun_top:.0}" width="{sun_d:.0}" height="{sun_d:.0}" fill="url(#sun)"/>
  <polygon points="{back}" fill="url(#hills)" opacity="0.55"/>
  <polygon points="{terrain}" fill="url(#terrain)" stroke="{grass_stroke}" stroke-opacity="0.5" stroke-width="4"/>
  <rect width="{w}" height="{h}" fill="url(#grid)" opacity="0.3"/>
  <rect width="{w}" height="{h}" fill="url(#vignette)"/>
</svg>"##,
        w = VIEW_W as u32,
        h = VIEW_H as u32,
        sky_top = palette.sky_top,
        sky_bottom = palette.sky_bottom,
        sun = palette.sun,
        terrain_top = palette.terrain_top,
        terrain_bottom = palette.terrain_bottom,
        sun_left = sun_x / 100.0 * VIEW_W as f64 - 160.0,
        sun_top = sun_y / 100.0 * VIEW_H as f64 - 80.0,
        sun_d = 320.0,
        back = back_points.join(" "),
        terrain = ridge.join(" "),
        grass_stroke = palette.grass.unwrap_or("none"),
    )
}

/// The scene as an svg handle, ready for the `svg` widget — the shared
/// entry point for the cards and the hero.
pub fn scene_handle(biome: Biome, seed: u64) -> iced::widget::svg::Handle {
    iced::widget::svg::Handle::from_memory(scene_svg(biome, seed).into_bytes())
}
