//! Status dot — the style guide's `.pulse-dot` (§07).
//!
//! An 8px Lode Green dot with a glow, wrapped by a ring that expands and
//! fades on a 1.8s loop (§07 `pulse-ring`, §08 "Pulse · 1.8s ease-out ∞").
//! Ambient motion — the widget requests a redraw every frame while pulsing,
//! so [`PulseDot::pulsing`] can freeze it when that cost matters.
//!
//! ```text
//! PulseDot::new()                       // online
//! PulseDot::with_color(color::Accents::AMBER).pulsing(false)
//! ```

use std::time::{Duration, Instant};

use crate::app::theme::color;
use iced::advanced::Renderer as _;
use iced::advanced::layout::{self, Limits, Node};
use iced::advanced::renderer;
use iced::advanced::widget::{Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget};
use iced::border::Radius;
use iced::mouse::{Cursor, Interaction};
use iced::{Border, Color, Element, Event, Length, Point, Rectangle, Renderer as BaseRenderer, Size, Theme, window};

use super::anim;

/// One pulse cycle — §07 `animation: pulse-ring 1.8s ease-out infinite`.
const PULSE_PERIOD: Duration = Duration::from_millis(1800);
/// The ring starts just outside the dot and expands to 2.2× (§07 keyframes).
const RING_START: f32 = 0.8;
const RING_END: f32 = 2.2;

/// A pulsing status dot (`.pulse-dot`).
pub struct PulseDot {
    size: f32,
    color: Color,
    pulsing: bool,
}

#[derive(Debug)]
struct State {
    /// Pulse phase, `0.0..=1.0` of one period.
    phase: f32,
    last_tick: Option<Instant>,
}

impl PulseDot {
    /// Creates a Lode Green status dot — "online".
    pub fn new() -> Self {
        Self::with_color(color::Primary::DEFAULT)
    }

    /// Creates a status dot in an arbitrary color.
    pub fn with_color(color: Color) -> Self {
        Self {
            size: 8.0,
            color,
            pulsing: true,
        }
    }

    /// Diameter of the dot, in logical pixels.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Whether the ring animates. Freezing saves the per-frame redraw.
    pub fn pulsing(mut self, pulsing: bool) -> Self {
        self.pulsing = pulsing;
        self
    }
}

impl Default for PulseDot {
    fn default() -> Self {
        Self::new()
    }
}

impl<Message> Widget<Message, Theme, BaseRenderer> for PulseDot {
    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Fixed(self.size),
            height: Length::Fixed(self.size),
        }
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State { phase: 0.0, last_tick: None })
    }

    fn layout(&mut self, _tree: &mut Tree, _renderer: &BaseRenderer, limits: &Limits) -> Node {
        layout::atomic(limits, Length::Fixed(self.size), Length::Fixed(self.size))
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        _layout: Layout<'_>,
        _cursor: Cursor,
        _renderer: &BaseRenderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let Event::Window(window::Event::RedrawRequested(now)) = event else {
            return;
        };

        if !self.pulsing {
            return;
        }

        let state = tree.state.downcast_mut::<State>();
        let dt = state.last_tick.map(|last| now.saturating_duration_since(last)).unwrap_or_default();
        state.last_tick = Some(*now);
        state.phase = (state.phase + dt.as_secs_f32() / PULSE_PERIOD.as_secs_f32()) % 1.0;
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
        let center = Point::new(bounds.center_x(), bounds.center_y());

        // Expanding, fading ring.
        let phase = if self.pulsing { state.phase } else { 0.0 };
        let ring_diameter = self.size * (RING_START + (RING_END - RING_START) * phase);
        let ring_alpha = (1.0 - phase) * 0.9;
        if ring_alpha > 0.01 {
            let ring_bounds = Rectangle::new(
                Point::new(center.x - ring_diameter / 2.0, center.y - ring_diameter / 2.0),
                Size::new(ring_diameter, ring_diameter),
            );
            renderer.fill_quad(
                renderer::Quad {
                    bounds: ring_bounds,
                    border: Border {
                        width: 1.5,
                        color: anim::fade(self.color, ring_alpha),
                        radius: Radius::from(ring_diameter / 2.0),
                    },
                    ..renderer::Quad::default()
                },
                Color::TRANSPARENT,
            );
        }

        // The dot itself, glowing (§07 `box-shadow: 0 0 8px var(--mc-green-glow)`).
        renderer.fill_quad(
            renderer::Quad {
                bounds,
                border: Border {
                    radius: Radius::from(self.size / 2.0),
                    ..Border::default()
                },
                shadow: iced::Shadow {
                    color: anim::fade(self.color, 0.35),
                    offset: iced::Vector::new(0.0, 0.0),
                    blur_radius: 8.0,
                },
                ..renderer::Quad::default()
            },
            self.color,
        );
    }

    fn mouse_interaction(&self, _tree: &Tree, _layout: Layout<'_>, _cursor: Cursor, _viewport: &Rectangle, _renderer: &BaseRenderer) -> Interaction {
        Interaction::default()
    }
}

impl<'a, Message> From<PulseDot> for Element<'a, Message, Theme, BaseRenderer> {
    fn from(dot: PulseDot) -> Self {
        Element::new(dot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_the_guide() {
        let dot = PulseDot::new();
        assert_eq!(dot.size, 8.0);
        assert_eq!(dot.color, color::Primary::DEFAULT);
        assert!(dot.pulsing);
    }
}
