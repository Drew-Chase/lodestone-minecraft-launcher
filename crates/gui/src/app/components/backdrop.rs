//! Login backdrop — the layered ambient background of the login screen.
//!
//! Recreates the reference artboard's stack (`01 · Login`): a synthetic
//! forest scene under a heavy blur, an animated two-blob aurora, drifting
//! embers, and the dim overlay — all behind the transparent titlebar.
//!
//! **How the blur works:** iced has no content blur filter and only linear
//! gradients, so the scene is a procedurally generated SVG rasterized by
//! resvg (which implements `feGaussianBlur` + `feColorMatrix`) — one
//! rasterization per size, cached by the svg pipeline. The aurora blobs are
//! pre-softened radial-gradient sprites; the embers are layered quads.
//!
//! The widget redraws continuously while mounted (the design's loops are
//! infinite) and unmounts the moment the router leaves `Page::Login`.

use std::f32::consts::FRAC_PI_2;
use std::sync::OnceLock;
use std::time::Instant;

use iced::advanced::layout::{Limits, Node};
use iced::advanced::renderer;
use iced::advanced::svg::Renderer as _;
use iced::advanced::svg::{Handle, Svg};
use iced::advanced::widget::{Tree, tree};
use iced::advanced::{Clipboard, Layout, Renderer as _, Shell, Widget};
use iced::border::Radius;
use iced::mouse::{Cursor, Interaction};
use iced::{Background, Border, Element, Event, Length, Point, Radians, Rectangle, Renderer as BaseRenderer, Size, Theme, gradient};
use iced_color_extension::rgb;

use crate::app::theme::color;

/// The reference artboard's scene seed (`Scene biome="forest" seed={7}`).
const SCENE_SEED: f64 = 7.0;
/// CSS `aurora-float` — 18s ease-in-out infinite.
const AURORA_PERIOD: f32 = 18.0;
/// `.aurora::after` starts mid-cycle (`animation-delay: -9s`).
const VIOLET_DELAY: f32 = 9.0;
/// `Particles count={24}`.
const EMBER_COUNT: usize = 24;
/// The reference scene is inset −40px before blurring so the blur never
/// pulls transparent edges into view.
const SCENE_BLEED: f32 = 40.0;

/// One aurora blob — `.aurora::before` (green) or `.aurora::after` (violet).
#[derive(Debug, Clone, Copy, PartialEq)]
enum Blob {
    Green,
    Violet,
}

impl Blob {
    /// Sprite size in px (`::before` 600, `::after` 500).
    fn size(self) -> f32 {
        match self {
            Blob::Green => 600.0,
            Blob::Violet => 500.0,
        }
    }

    /// Untransformed top-left within `bounds` (CSS offsets: green
    /// `top:-200 left:-100`, violet `bottom:-150 right:-100`).
    fn base(self, bounds: Rectangle) -> Point {
        match self {
            Blob::Green => Point::new(bounds.x - 100.0, bounds.y - 200.0),
            Blob::Violet => Point::new(
                bounds.x + bounds.width - self.size() + 100.0,
                bounds.y + bounds.height - self.size() + 150.0,
            ),
        }
    }

    /// Glow tint, from the guide's `--mc-green` / `--violet`.
    fn hex(self) -> &'static str {
        match self {
            Blob::Green => "#22ff84",
            Blob::Violet => "#9747ff",
        }
    }

    fn sprite(self) -> &'static Handle {
        static GREEN: OnceLock<Handle> = OnceLock::new();
        static VIOLET: OnceLock<Handle> = OnceLock::new();
        match self {
            Blob::Green => GREEN.get_or_init(|| glow_sprite_handle(self.hex(), self.size() as u32)),
            Blob::Violet => VIOLET.get_or_init(|| glow_sprite_handle(self.hex(), self.size() as u32)),
        }
    }
}

/// Per-instance animation state.
struct State {
    /// Animation clock — everything is a pure function of `start.elapsed()`.
    start: Instant,
    /// The scene SVG, rasterized for a specific bounds size.
    scene: Option<(Size, Handle)>,
}

/// The full-screen ambient background of the login screen. Produces no
/// messages.
#[derive(Debug, Clone, Copy, Default)]
pub struct LoginBackdrop;

impl LoginBackdrop {
    pub fn new() -> Self {
        Self
    }
}

impl<Message> Widget<Message, Theme, BaseRenderer> for LoginBackdrop
where
    Message: 'static,
{
    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Fill,
            height: Length::Fill,
        }
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State {
            start: Instant::now(),
            scene: None,
        })
    }

    fn layout(&mut self, tree: &mut Tree, _renderer: &BaseRenderer, limits: &Limits) -> Node {
        let size = limits.resolve(Length::Fill, Length::Fill, Size::new(0.0, 0.0));
        // The scene SVG is sized to the resolved bounds — `draw` only reads,
        // so the rebuild happens here where the tree is mutable.
        let state = tree.state.downcast_mut::<State>();
        Self::update_scene(state, size);
        Node::new(size)
    }

    fn update(
        &mut self,
        _tree: &mut Tree,
        _event: &Event,
        _layout: Layout<'_>,
        _cursor: Cursor,
        _renderer: &BaseRenderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        // Ambient loops never settle — keep the frame loop alive while the
        // backdrop is mounted. It unmounts with the login page.
        shell.request_redraw();
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut BaseRenderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: Cursor,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State>();
        let bounds = layout.bounds();
        if bounds.width <= 0.0 || bounds.height <= 0.0 {
            return;
        }
        let t = state.start.elapsed().as_secs_f32();

        // 1 — the blurred forest scene, rebuilt in `layout` on resize.
        let Some((_, scene)) = state.scene.as_ref() else {
            return;
        };
        renderer.draw_svg(
            Svg {
                handle: scene.clone(),
                opacity: 1.0,
                rotation: Radians(0.0),
                color: None,
            },
            bounds,
            bounds,
        );

        // 2 — the dim overlay sits UNDER the glows (reference order: scene →
        // dim → aurora → particles), so they punch through the darkness.
        renderer.fill_quad(
            renderer::Quad {
                bounds,
                ..renderer::Quad::default()
            },
            Background::Gradient(
                gradient::Linear::new(Radians(FRAC_PI_2))
                    .add_stop(0.0, rgb!(0, 0, 0, 35%))
                    .add_stop(1.0, rgb!(0, 0, 0, 75%))
                    .into(),
            ),
        );

        // 3 — aurora blobs riding their 18s float cycles.
        draw_blob(renderer, bounds, Blob::Green, (t / AURORA_PERIOD) % 1.0);
        draw_blob(renderer, bounds, Blob::Violet, ((t + VIOLET_DELAY) / AURORA_PERIOD) % 1.0);

        // 4 — embers drifting bottom → top.
        for i in 0..EMBER_COUNT {
            draw_ember(renderer, bounds, i, t);
        }
    }

    fn mouse_interaction(&self, _tree: &Tree, _layout: Layout<'_>, _cursor: Cursor, _viewport: &Rectangle, _renderer: &BaseRenderer) -> Interaction {
        Interaction::default()
    }
}

impl LoginBackdrop {
    /// Rebuilds the scene SVG when the size changed. Split from `draw` so
    /// the mutable cache update reads cleanly.
    fn update_scene(state: &mut State, size: Size) -> Handle {
        if let Some((cached, handle)) = &state.scene
            && *cached == size
        {
            return handle.clone();
        }
        let handle = Handle::from_memory(scene_svg(size.width, size.height, SCENE_SEED).into_bytes());
        state.scene = Some((size, handle.clone()));
        handle
    }
}

impl<'a, Message: 'static> From<LoginBackdrop> for Element<'a, Message, Theme, BaseRenderer> {
    fn from(backdrop: LoginBackdrop) -> Self {
        Element::new(backdrop)
    }
}

/// Draws one aurora blob at its cycle position.
fn draw_blob(renderer: &mut BaseRenderer, bounds: Rectangle, blob: Blob, phase: f32) {
    let (dx, dy, scale) = aurora_offset(phase);
    let size = blob.size() * scale;
    let center = blob.base(bounds) + Vector::new(blob.size() / 2.0 + dx, blob.size() / 2.0 + dy);
    let sprite_bounds = Rectangle::new(center - Vector::new(size / 2.0, size / 2.0), Size::new(size, size));
    renderer.draw_svg(
        Svg {
            handle: blob.sprite().clone(),
            // CSS: `.aurora { opacity: 0.7 }` over `::before/::after { opacity: 0.55 }`
            // — the 0.55 is baked into the sprite's gradient stops.
            opacity: 0.7,
            rotation: Radians(0.0),
            color: None,
        },
        sprite_bounds,
        bounds,
    );
}

use iced::Vector;

/// Draws one ember (`Particles` + CSS `drift`).
fn draw_ember(renderer: &mut BaseRenderer, bounds: Rectangle, i: usize, t: f32) {
    let (left_pct, size, delay, dur) = ember_layout(i);
    let p = ((t + delay) / dur) % 1.0;

    // 0% → 100%: translateY(-110vh) translateX(30px), fading in over the
    // first 10% and out over the last 10%.
    let progress = p * 1.1;
    let x = bounds.x + (left_pct / 100.0) * bounds.width + 30.0 * p;
    let y = bounds.y + bounds.height + 10.0 - progress * bounds.height;
    let alpha = if p < 0.1 {
        p / 0.1
    } else if p > 0.9 {
        (1.0 - p) / 0.1
    } else {
        1.0
    } * 0.7;

    // Core dot with a soft halo approximating `box-shadow: 0 0 4+size px`.
    let core = Rectangle::new(Point::new(x, y) - Vector::new(size / 2.0, size / 2.0), Size::new(size, size));
    let halo_size = size + 8.0;
    let halo = Rectangle::new(
        Point::new(x, y) - Vector::new(halo_size / 2.0, halo_size / 2.0),
        Size::new(halo_size, halo_size),
    );
    renderer.fill_quad(
        renderer::Quad {
            bounds: halo,
            border: Border {
                radius: Radius::from(halo_size / 2.0),
                ..Border::default()
            },
            ..renderer::Quad::default()
        },
        anim_fade(color::Primary::DEFAULT, alpha * 0.2),
    );
    renderer.fill_quad(
        renderer::Quad {
            bounds: core,
            border: Border {
                radius: Radius::from(size / 2.0),
                ..Border::default()
            },
            ..renderer::Quad::default()
        },
        anim_fade(color::Primary::DEFAULT, alpha),
    );
}

/// `anim::fade` without importing the module's easing machinery.
fn anim_fade(color: iced::Color, alpha: f32) -> iced::Color {
    iced::Color { a: color.a * alpha, ..color }
}

/// Ember field layout — a faithful port of `Particles.jsx`:
/// `delay = (i·0.37) % 12`, `dur = 8 + (i·0.7) % 6`, `left = (i·17.3) % 100`,
/// `size = 1 + i % 3`. Returns `(left %, size px, delay s, duration s)`.
fn ember_layout(i: usize) -> (f32, f32, f32, f32) {
    let i = i as f32;
    ((i * 17.3) % 100.0, 1.0 + (i as u32 % 3) as f32, (i * 0.37) % 12.0, 8.0 + (i * 0.7) % 6.0)
}

/// CSS `aurora-float` — piecewise-linear through the keyframes:
/// `0%/100% → (0, 0, 1)`, `33% → (60px, −40px, 1.1)`, `66% → (−40px, 50px, 0.95)`.
/// Returns `(dx, dy, scale)`.
fn aurora_offset(phase: f32) -> (f32, f32, f32) {
    let keys = [
        (0.0, 0.0, 0.0, 1.0),
        (1.0 / 3.0, 60.0, -40.0, 1.1),
        (2.0 / 3.0, -40.0, 50.0, 0.95),
        (1.0, 0.0, 0.0, 1.0),
    ];
    let phase = phase.clamp(0.0, 1.0);
    let (a, b) = keys
        .iter()
        .zip(keys.iter().skip(1))
        .find(|(_, end)| phase <= end.0)
        .unwrap_or((&keys[keys.len() - 2], &keys[keys.len() - 1]));
    let t = if b.0 > a.0 { (phase - a.0) / (b.0 - a.0) } else { 0.0 };
    (a.1 + (b.1 - a.1) * t, a.2 + (b.2 - a.2) * t, a.3 + (b.3 - a.3) * t)
}

/// The scene's deterministic pseudo-random — `Scene.jsx`'s
/// `fract(sin((seed+1)·9301 + i·49297)·233280)`, always in `[0, 1)`.
fn prng(seed: f64, i: usize) -> f64 {
    let x = ((seed + 1.0) * 9301.0 + i as f64 * 49297.0).sin() * 233280.0;
    x - x.floor()
}

/// Generates the blurred forest scene: sky, sun glow, back hills, terrain
/// with a grass ridge, voxel grid, and vignette — all under
/// `blur(24px) saturate(1.2)`, exactly like the reference wrapper.
pub fn scene_svg(w: f32, h: f32, seed: f64) -> String {
    // Sun/moon position — `sunX = 15 + r(99)·70 %`, `sunY = 18 + r(100)·20 %`.
    let sun_x = 15.0 + prng(seed, 99) * 70.0;
    let sun_y = 18.0 + prng(seed, 100) * 20.0;
    let sun_r = 400.0; // glow radius — the disc + its halo in one radial falloff

    // Jagged terrain silhouette — 18 columns, heights 30–85% (`Scene.jsx`).
    let cols = 18usize;
    let ridge: Vec<String> = (0..cols)
        .map(|i| {
            let height_pct = 30.0 + prng(seed, i) * 55.0;
            let x = i as f64 / (cols - 1) as f64;
            format!("{:.1},{:.1}", x * w as f64, (1.0 - height_pct / 100.0) * h as f64)
        })
        .collect();

    // Back hills — the fixed clip-path silhouette from `Scene.jsx`, as px.
    let back: [&str; 9] = ["0,60", "12,58", "22,65", "34,52", "48,58", "62,50", "76,56", "88,48", "100,54"];
    let back_points: Vec<String> = std::iter::once(format!("{:.1},{:.1}", -SCENE_BLEED, h + SCENE_BLEED))
        .chain(back.iter().map(|p| {
            let (x, y) = p.split_once(',').unwrap();
            format!(
                "{:.1},{:.1}",
                x.parse::<f64>().unwrap() / 100.0 * w as f64,
                y.parse::<f64>().unwrap() / 100.0 * h as f64
            )
        }))
        .chain(std::iter::once(format!("{:.1},{:.1}", w + SCENE_BLEED, h + SCENE_BLEED)))
        .collect();

    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">
  <defs>
    <linearGradient id="sky" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#060b08"/>
      <stop offset="0.55" stop-color="#0e2013"/>
      <stop offset="1" stop-color="#1a3f24"/>
    </linearGradient>
    <radialGradient id="sun" cx="0.5" cy="0.5" r="0.5">
      <stop offset="0" stop-color="#fff2b8" stop-opacity="0.55"/>
      <stop offset="0.2" stop-color="#ffe9a0" stop-opacity="0.35"/>
      <stop offset="0.5" stop-color="#ffd98a" stop-opacity="0.12"/>
      <stop offset="1" stop-color="#ffd98a" stop-opacity="0"/>
    </radialGradient>
    <linearGradient id="hills" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#000000" stop-opacity="0.2"/>
      <stop offset="1" stop-color="#000000" stop-opacity="0.5"/>
    </linearGradient>
    <linearGradient id="terrain" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#1c3d25"/>
      <stop offset="1" stop-color="#081509"/>
    </linearGradient>
    <radialGradient id="vignette" cx="0.5" cy="0.5" r="0.75">
      <stop offset="0.4" stop-color="#000000" stop-opacity="0"/>
      <stop offset="1" stop-color="#000000" stop-opacity="0.5"/>
    </radialGradient>
    <pattern id="grid" width="16" height="16" patternUnits="userSpaceOnUse">
      <path d="M 16 0 L 0 0 0 16" fill="none" stroke="#000000" stroke-opacity="0.15" stroke-width="1"/>
    </pattern>
    <filter id="soften" x="-15%" y="-15%" width="130%" height="130%">
      <feGaussianBlur stdDeviation="36"/>
      <feColorMatrix type="saturate" values="1.1"/>
    </filter>
  </defs>
  <g filter="url(#soften)">
    <rect x="-{bleed}" y="-{bleed}" width="{wide}" height="{tall}" fill="url(#sky)"/>
    <rect x="{sun_left:.1}" y="{sun_top:.1}" width="{sun_d}" height="{sun_d}" fill="url(#sun)"/>
    <polygon points="{back}" fill="url(#hills)" opacity="0.4"/>
    <polygon points="{terrain}" fill="url(#terrain)" stroke="#2c7a42" stroke-opacity="0.5" stroke-width="4"/>
    <rect x="-{bleed}" y="-{bleed}" width="{wide}" height="{tall}" fill="url(#grid)" opacity="0.35"/>
  </g>
  <rect width="{w}" height="{h}" fill="url(#vignette)"/>
</svg>"##,
        w = w,
        h = h,
        bleed = SCENE_BLEED,
        wide = w + SCENE_BLEED * 2.0,
        tall = h + SCENE_BLEED * 2.0,
        sun_left = sun_x / 100.0 * w as f64 - sun_r,
        sun_top = sun_y / 100.0 * h as f64 - sun_r,
        sun_d = sun_r * 2.0,
        back = back_points.join(" "),
        terrain = ridge.join(" "),
    )
}

/// A pre-softened radial glow sprite — the CSS
/// `radial-gradient(circle, C 0%, transparent 65%)` + `blur(80px)` combo,
/// expressed as eased gradient stops. Colors are baked in at the CSS
/// layer's 0.55 opacity.
fn glow_sprite_handle(hex: &str, size: u32) -> Handle {
    let svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{size}" height="{size}" viewBox="0 0 {size} {size}">
  <defs>
    <radialGradient id="g" cx="0.5" cy="0.5" r="0.5">
      <stop offset="0" stop-color="{hex}" stop-opacity="0.55"/>
      <stop offset="0.3" stop-color="{hex}" stop-opacity="0.4"/>
      <stop offset="0.65" stop-color="{hex}" stop-opacity="0.12"/>
      <stop offset="1" stop-color="{hex}" stop-opacity="0"/>
    </radialGradient>
  </defs>
  <rect width="{size}" height="{size}" fill="url(#g)"/>
</svg>"##
    );
    Handle::from_memory(svg.into_bytes())
}

/// Glow halo behind the login card's logo mark — [`glow_sprite_handle`] at
/// card scale, exposed for `pages/login.rs`.
pub(crate) fn logo_glow_handle() -> &'static Handle {
    static GLOW: OnceLock<Handle> = OnceLock::new();
    GLOW.get_or_init(|| glow_sprite_handle("#22ff84", 96))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prng_is_deterministic_and_bounded() {
        for i in 0..64 {
            let a = prng(SCENE_SEED, i);
            let b = prng(SCENE_SEED, i);
            assert_eq!(a, b, "prng must be deterministic");
            assert!((0.0..1.0).contains(&a), "prng({i}) = {a} out of range");
        }
        assert_ne!(prng(SCENE_SEED, 1), prng(SCENE_SEED, 2), "values must vary");
    }

    #[test]
    fn aurora_offset_hits_the_css_keyframes() {
        let (dx, dy, scale) = aurora_offset(0.0);
        assert_eq!((dx, dy, scale), (0.0, 0.0, 1.0));
        let (dx, dy, scale) = aurora_offset(1.0 / 3.0);
        assert!((dx - 60.0).abs() < 1e-4 && (dy + 40.0).abs() < 1e-4 && (scale - 1.1).abs() < 1e-4);
        let (dx, dy, scale) = aurora_offset(2.0 / 3.0);
        assert!((dx + 40.0).abs() < 1e-4 && (dy - 50.0).abs() < 1e-4 && (scale - 0.95).abs() < 1e-4);
        let (dx, dy, scale) = aurora_offset(1.0);
        assert_eq!((dx, dy, scale), (0.0, 0.0, 1.0));
    }

    #[test]
    fn ember_layout_matches_the_reference_field() {
        let (left, size, delay, dur) = ember_layout(0);
        assert_eq!((left, size, delay), (0.0, 1.0, 0.0));
        assert!((8.0..14.0).contains(&dur));
        let (left, size, ..) = ember_layout(1);
        assert!((0.0..100.0).contains(&left), "left must be a percentage");
        assert!((1.0..=3.0).contains(&size));
        // Two embers never share the same spawn column.
        assert_ne!(ember_layout(0).0, ember_layout(3).0);
    }

    #[test]
    fn scene_svg_carries_every_layer() {
        let svg = scene_svg(1280.0, 820.0, SCENE_SEED);
        assert!(svg.contains("feGaussianBlur"), "scene must be blurred");
        assert!(svg.contains("feColorMatrix"), "scene must be saturated");
        assert!(svg.contains("radialGradient"), "sun glow needs a radial falloff");
        assert!(svg.contains("<polygon"), "hills and terrain are silhouettes");
        assert!(svg.contains("url(#grid)"), "voxel grid overlay");
        assert!(svg.contains("url(#vignette)"));
        // The blur bleeds in from outside, so the painted content must
        // extend past the viewBox by the bleed amount.
        assert!(svg.contains(&format!("x=\"-{SCENE_BLEED}\"")));
    }
}
