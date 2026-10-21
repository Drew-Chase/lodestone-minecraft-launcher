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

use std::sync::OnceLock;
use std::time::Instant;

use iced::advanced::layout::{Limits, Node};
use iced::advanced::renderer;
use iced::advanced::svg::Renderer as _;
use iced::advanced::svg::{Handle, Svg};
use iced::advanced::widget::{Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget};
use iced::mouse::{Cursor, Interaction};
use iced::{Element, Event, Length, Point, Radians, Rectangle, Renderer as BaseRenderer, Size, Theme};

/// CSS `aurora-float` — 18s ease-in-out infinite.
const AURORA_PERIOD: f32 = 18.0;
/// `.aurora::after` starts mid-cycle (`animation-delay: -9s`).
const VIOLET_DELAY: f32 = 9.0;
/// `Particles count={24}`.
const EMBER_COUNT: usize = 24;

/// Per-instance animation state.
struct State {
    /// Animation clock — everything is a pure function of `start.elapsed()`.
    start: Instant,
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
        tree::State::new(State { start: Instant::now() })
    }

    fn layout(&mut self, _tree: &mut Tree, _renderer: &BaseRenderer, limits: &Limits) -> Node {
        Node::new(limits.resolve(Length::Fill, Length::Fill, Size::new(0.0, 0.0)))
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
        // The reference artboard is 1280×820 — washes and embers scale with
        // the window so proportions hold at any size.
        let factor = (bounds.width / 1280.0).clamp(0.75, 2.0);

        // 1 — the static ambient field: generated once at the canonical
        // artboard size and stretched by the GPU. Resizing must never
        // re-rasterize a full-window SVG (that was seconds of stall).
        renderer.draw_svg(
            Svg {
                handle: scene_handle().clone(),
                opacity: 1.0,
                rotation: Radians(0.0),
                color: None,
            },
            bounds,
            bounds,
        );

        // 2 — the three aurora glows riding their 18s float cycles: Lode
        // Green upper-left, the scene's warm sun top-center, violet
        // bottom-right — distinct blobs over the dark field, like the
        // reference.
        draw_blob(
            renderer,
            bounds,
            green_blob_handle(),
            Point::new(bounds.x + bounds.width * 0.22, bounds.y + bounds.height * 0.22),
            900.0 * factor,
            (t / AURORA_PERIOD) % 1.0,
        );
        draw_blob(
            renderer,
            bounds,
            amber_blob_handle(),
            Point::new(bounds.x + bounds.width * 0.5, bounds.y + bounds.height * 0.16),
            620.0 * factor,
            ((t + VIOLET_DELAY) / AURORA_PERIOD) % 1.0,
        );
        draw_blob(
            renderer,
            bounds,
            violet_blob_handle(),
            Point::new(bounds.x + bounds.width * 0.88, bounds.y + bounds.height * 0.82),
            760.0 * factor,
            ((t + VIOLET_DELAY * 2.0) / AURORA_PERIOD) % 1.0,
        );

        // 3 — embers drifting bottom → top.
        for i in 0..EMBER_COUNT {
            draw_ember(renderer, bounds, i, t, factor);
        }
    }

    fn mouse_interaction(&self, _tree: &Tree, _layout: Layout<'_>, _cursor: Cursor, _viewport: &Rectangle, _renderer: &BaseRenderer) -> Interaction {
        Interaction::default()
    }
}

impl<'a, Message: 'static> From<LoginBackdrop> for Element<'a, Message, Theme, BaseRenderer> {
    fn from(backdrop: LoginBackdrop) -> Self {
        Element::new(backdrop)
    }
}

/// Draws one aurora blob at its cycle position.
fn draw_blob(renderer: &mut BaseRenderer, bounds: Rectangle, sprite: &Handle, center: Point, size: f32, phase: f32) {
    let (dx, dy, scale) = aurora_offset(phase);
    let size = size * scale;
    let center = center + Vector::new(dx * factor_of(size), dy * factor_of(size));
    let sprite_bounds = Rectangle::new(center - Vector::new(size / 2.0, size / 2.0), Size::new(size, size));
    renderer.draw_svg(
        Svg {
            handle: sprite.clone(),
            opacity: 1.0,
            rotation: Radians(0.0),
            color: None,
        },
        sprite_bounds,
        bounds,
    );
}

/// The blob's CSS drift is 60px per 1300px of blob — keep the translation
/// proportional at any scale.
fn factor_of(size: f32) -> f32 {
    size / 1300.0
}

use iced::Vector;

/// Draws one ember (`Particles` + CSS `drift`) — a sprite, so it layers
/// above the scene and wash textures.
fn draw_ember(renderer: &mut BaseRenderer, bounds: Rectangle, i: usize, t: f32, factor: f32) {
    let (left_pct, base_size, delay, dur) = ember_layout(i);
    let p = ((t + delay) / dur) % 1.0;

    // 0% → 100%: translateY(-110vh) translateX(30px), fading in over the
    // first 10% and out over the last 10%.
    let progress = p * 1.1;
    let x = bounds.x + (left_pct / 100.0) * bounds.width + 30.0 * factor * p;
    let y = bounds.y + bounds.height + 10.0 - progress * bounds.height;
    let alpha = if p < 0.1 {
        p / 0.1
    } else if p > 0.9 {
        (1.0 - p) / 0.1
    } else {
        1.0
    };

    // The sprite's solid core reaches half its radius; draw at 4× the core
    // so the dot is `core` px with a tight 1-core-wide glow falloff.
    let core = (base_size * factor).clamp(1.5, 3.5);
    let sprite_size = core * 4.0;
    let sprite_bounds = Rectangle::new(
        Point::new(x, y) - Vector::new(sprite_size / 2.0, sprite_size / 2.0),
        Size::new(sprite_size, sprite_size),
    );
    renderer.draw_svg(
        Svg {
            handle: ember_sprite_handle().clone(),
            opacity: alpha,
            rotation: Radians(0.0),
            color: None,
        },
        sprite_bounds,
        bounds,
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

/// Generates the ambient field: a flat dark-green base with corner falloff
/// and a fine grain (the reference's `.dust` dither) — the animated washes
/// and embers ride on top as sprites. The reference's terrain, hills, and
/// sun blur into nothing visible, so they are not painted; the field is
/// tuned against pixels sampled from `01-login.png`.
pub fn scene_svg(w: f32, h: f32) -> String {
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">
  <defs>
    <linearGradient id="base" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#0a130c"/>
      <stop offset="0.5" stop-color="#081009"/>
      <stop offset="1" stop-color="#070d08"/>
    </linearGradient>
    <radialGradient id="vignette" cx="0.5" cy="0.5" r="0.75">
      <stop offset="0.55" stop-color="#000000" stop-opacity="0"/>
      <stop offset="1" stop-color="#000000" stop-opacity="0.3"/>
    </radialGradient>
    <pattern id="dust" width="3" height="3" patternUnits="userSpaceOnUse">
      <circle cx="1.5" cy="1.5" r="0.5" fill="#ffffff" fill-opacity="0.05"/>
    </pattern>
  </defs>
  <rect width="{w}" height="{h}" fill="url(#base)"/>
  <rect width="{w}" height="{h}" fill="url(#dust)"/>
  <rect width="{w}" height="{h}" fill="url(#vignette)"/>
</svg>"##,
        w = w,
        h = h,
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

/// The aurora glow sprite — a distinct blob (not a screen-filling wash):
/// bright core, steep CSS-aurora falloff (`transparent 65%` + 80px blur).
/// Stop alphas are authored pre-compensated: the svg texture pipeline
/// renders translucent stops ≈ squared, so callers pass `sqrt(target)`.
fn blob_sprite(hex: &str, peak: f32) -> Handle {
    Handle::from_memory(blob_svg(hex, peak).into_bytes())
}

/// The blob's SVG markup — [`blob_sprite`] minus the handle wrapping, so
/// tests can assert on the stops.
fn blob_svg(hex: &str, peak: f32) -> String {
    let mid = peak * 0.55;
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="1000" height="1000" viewBox="0 0 1000 1000">
  <defs>
    <radialGradient id="b" cx="0.5" cy="0.5" r="0.5">
      <stop offset="0" stop-color="{hex}" stop-opacity="{peak:.2}"/>
      <stop offset="0.45" stop-color="{hex}" stop-opacity="{mid:.2}"/>
      <stop offset="0.8" stop-color="{hex}" stop-opacity="0.04"/>
      <stop offset="1" stop-color="{hex}" stop-opacity="0"/>
    </radialGradient>
  </defs>
  <rect width="1000" height="1000" fill="url(#b)"/>
</svg>"##
    )
}

fn green_blob_handle() -> &'static Handle {
    static BLOB: OnceLock<Handle> = OnceLock::new();
    BLOB.get_or_init(|| blob_sprite("#22ff84", 0.59))
}

fn amber_blob_handle() -> &'static Handle {
    static BLOB: OnceLock<Handle> = OnceLock::new();
    BLOB.get_or_init(|| blob_sprite("#ffcf7d", 0.50))
}

fn violet_blob_handle() -> &'static Handle {
    static BLOB: OnceLock<Handle> = OnceLock::new();
    BLOB.get_or_init(|| blob_sprite("#9747ff", 0.55))
}

/// One ember — a glowing green speck. SVG, not a quad: iced_wgpu renders a
/// layer's quads *before* its textures, so quad embers would sink beneath
/// the scene and blob sprites.
fn ember_sprite_handle() -> &'static Handle {
    static EMBER: OnceLock<Handle> = OnceLock::new();
    EMBER.get_or_init(|| {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 64 64">
  <defs>
    <radialGradient id="e" cx="0.5" cy="0.5" r="0.5">
      <stop offset="0" stop-color="#22ff84" stop-opacity="1"/>
      <stop offset="0.5" stop-color="#22ff84" stop-opacity="0.85"/>
      <stop offset="0.75" stop-color="#22ff84" stop-opacity="0.3"/>
      <stop offset="1" stop-color="#22ff84" stop-opacity="0"/>
    </radialGradient>
  </defs>
  <rect width="64" height="64" fill="url(#e)"/>
</svg>"##;
        Handle::from_memory(svg.as_bytes().to_vec())
    })
}

/// The ambient field, generated ONCE at the artboard's canonical 1280×820
/// size and drawn stretched to the window: resizing rescales a cached GPU
/// texture instead of re-rasterizing a full-window SVG per size tick. The
/// base gradient and vignette scale cleanly; the 3px dust dots stretch
/// imperceptibly at 5% opacity.
fn scene_handle() -> &'static Handle {
    static SCENE: OnceLock<Handle> = OnceLock::new();
    SCENE.get_or_init(|| Handle::from_memory(scene_svg(1280.0, 820.0).into_bytes()))
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
        let svg = scene_svg(1280.0, 820.0);
        assert!(svg.contains("url(#base)"), "flat dark-green base");
        assert!(svg.contains("url(#dust)"), "pixel-dust texture");
        assert!(svg.contains("url(#vignette)"), "corner falloff");
        // Full-window filters cost seconds of CPU on first raster — the
        // scene must stay filter-free.
        assert!(!svg.contains("feTurbulence"), "no turbulence filters");
        assert!(!svg.contains("feGaussianBlur"), "no per-scene blur");
        assert!(!svg.contains("<polygon"), "no terrain silhouettes");
    }

    #[test]
    fn blobs_are_authored_precompensated() {
        let green = blob_svg("#22ff84", 0.59);
        assert!(green.contains("stop-color=\"#22ff84\""));
        assert!(green.contains("stop-opacity=\"0.59\""));
        assert!(green.contains("stop-opacity=\"0.32\""), "mid stop at 55% of peak");
        let amber = blob_svg("#ffcf7d", 0.50);
        assert!(amber.contains("stop-color=\"#ffcf7d\""));
    }
}
