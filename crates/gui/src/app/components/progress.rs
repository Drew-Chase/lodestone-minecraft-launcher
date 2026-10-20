//! Progress bar — the style guide's `.progress` control (§07).
//!
//! A 6px pill track with a Lode Green gradient fill (`--mc-green-dim →
//! --mc-green`). The fill width rides a spring so value changes read as motion
//! rather than jumps, and an [`ProgressBar::indeterminate`] mode sweeps a
//! highlight band for operations with unknown duration (the guide's shimmer,
//! repurposed as an honest spinner).
//!
//! ```text
//! ProgressBar::new(0.68)
//! ProgressBar::new(0.0).indeterminate(true)
//! ```

use std::time::{Duration, Instant};

use crate::app::theme::color;
use iced::advanced::Renderer as _;
use iced::advanced::layout::{self, Limits, Node};
use iced::advanced::renderer;
use iced::advanced::widget::{Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget};
use iced::border::Radius;
use iced::gradient;
use iced::mouse::{Cursor, Interaction};
use iced::{Color, Element, Event, Gradient, Length, Rectangle, Renderer as BaseRenderer, Size, Theme, window};
use iced_anim::Animated;
use iced_color_extension::rgb;

use super::anim;

/// Period of the indeterminate sweep — long and looping (§08 "Ambient").
const SWEEP_PERIOD: Duration = Duration::from_millis(1600);
/// Width of the sweeping band, as a fraction of the track.
const SWEEP_BAND: f32 = 0.3;

/// An animated progress bar (`.progress`).
pub struct ProgressBar {
    /// Target fill, `0.0..=1.0`. Ignored in indeterminate mode.
    fraction: f32,
    indeterminate: bool,
    width: Length,
    height: Length,
}

#[derive(Debug)]
struct State {
    /// Animated fill, `0.0..=1.0` of the track width.
    fill: Animated<f32>,
    /// Indeterminate sweep phase, `0.0..=1.0` of one period.
    sweep: f32,
    last_tick: Option<Instant>,
}

impl ProgressBar {
    /// Creates a progress bar with the given fill, `0.0..=1.0`.
    pub fn new(fraction: f32) -> Self {
        Self {
            fraction: fraction.clamp(0.0, 1.0),
            indeterminate: false,
            width: Length::Fill,
            height: Length::Fixed(6.0),
        }
    }

    /// Sweeps a highlight band instead of showing a determinate fill.
    pub fn indeterminate(mut self, indeterminate: bool) -> Self {
        self.indeterminate = indeterminate;
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// The guide's fill — `--mc-green-dim → --mc-green`, left to right.
    fn fill_gradient() -> Gradient {
        Gradient::Linear(
            gradient::Linear::new(0.0)
                .add_stop(0.0, color::Primary::DIM)
                .add_stop(1.0, color::Primary::DEFAULT),
        )
    }

    /// The sweeping highlight — the guide's shimmer gradient.
    fn sweep_gradient() -> Gradient {
        Gradient::Linear(
            gradient::Linear::new(0.0)
                .add_stop(0.0, Color::TRANSPARENT)
                .add_stop(0.5, rgb!(255, 255, 255, 35%))
                .add_stop(1.0, Color::TRANSPARENT),
        )
    }
}

impl<Message> Widget<Message, Theme, BaseRenderer> for ProgressBar {
    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: self.height,
        }
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State {
            fill: Animated::new(self.fraction.clamp(0.0, 1.0), anim::glide()),
            sweep: 0.0,
            last_tick: None,
        })
    }

    fn layout(&mut self, _tree: &mut Tree, _renderer: &BaseRenderer, limits: &Limits) -> Node {
        layout::atomic(limits, self.width, self.height)
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

        let state = tree.state.downcast_mut::<State>();

        if self.indeterminate {
            // Advance the sweep; loop forever while the mode is active.
            let dt = state.last_tick.map(|last| now.saturating_duration_since(last)).unwrap_or_default();
            state.last_tick = Some(*now);
            let progress = dt.as_secs_f32() / SWEEP_PERIOD.as_secs_f32();
            state.sweep = (state.sweep + progress) % 1.0;
            shell.request_redraw();
        } else {
            state.last_tick = None;
            state.fill.tick(*now);
            if state.fill.is_animating() {
                shell.request_redraw();
            }
        }
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
        let radius = Radius::from(bounds.height / 2.0);

        // Track — white 6%.
        renderer.fill_quad(
            renderer::Quad {
                bounds,
                border: iced::Border {
                    radius,
                    ..iced::Border::default()
                },
                ..renderer::Quad::default()
            },
            rgb!(255, 255, 255, 6%),
        );

        if self.indeterminate {
            // Band enters from the left and exits on the right.
            let band_width = bounds.width * SWEEP_BAND;
            let travel = bounds.width + band_width;
            let x = bounds.x + (travel * state.sweep) - band_width;
            let band = Rectangle::new(iced::Point::new(x, bounds.y), Size::new(band_width, bounds.height));
            renderer.fill_quad(
                renderer::Quad {
                    bounds: band,
                    border: iced::Border {
                        radius,
                        ..iced::Border::default()
                    },
                    ..renderer::Quad::default()
                },
                Self::sweep_gradient(),
            );
        } else {
            let fill_width = bounds.width * state.fill.value();
            if fill_width < 0.5 {
                return;
            }
            let fill = Rectangle::new(bounds.position(), Size::new(fill_width, bounds.height));
            renderer.fill_quad(
                renderer::Quad {
                    bounds: fill,
                    border: iced::Border {
                        radius,
                        ..iced::Border::default()
                    },
                    ..renderer::Quad::default()
                },
                Self::fill_gradient(),
            );
        }
    }

    fn mouse_interaction(&self, _tree: &Tree, _layout: Layout<'_>, _cursor: Cursor, _viewport: &Rectangle, _renderer: &BaseRenderer) -> Interaction {
        Interaction::default()
    }
}

impl<'a, Message> From<ProgressBar> for Element<'a, Message, Theme, BaseRenderer> {
    fn from(bar: ProgressBar) -> Self {
        Element::new(bar)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fraction_is_clamped() {
        let bar = ProgressBar::new(1.4);
        assert_eq!(bar.fraction, 1.0);
        let bar = ProgressBar::new(-0.2);
        assert_eq!(bar.fraction, 0.0);
    }
}
