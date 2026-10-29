//! Toggle — the style guide's `.toggle` switch (§07).
//!
//! A 34×20 pill track with a circular knob. Off is the guide's neutral track
//! with a white knob; on is Lode Green with the on-green ink knob (`#062814`).
//! Hover brightens the track, press settles toward the dim gradient stop
//! (§02 — "Gradient stops, pressed states"), and the knob slides on a 120ms
//! ease (§08 — Micro).
//!
//! Animation is driven by iced_anim's `AnimatedState`, the same pattern the
//! library's own animated button uses: the widget-local [`Status`] carries
//! *everything* the visual depends on (including the on/off state, so a
//! programmatic flip retargets exactly like a click), `current_value`
//! retargets lazily during draw, and `needs_redraw` + `tick` run the frames.
//! All state is self-contained in the widget's tree state:
//!
//! ```text
//! Toggle::new(settings.auto_update).on_toggle(Message::AutoUpdateToggled)
//! ```

use crate::app::theme::color;
use iced::advanced::Renderer as _;
use iced::advanced::layout::{self, Limits, Node};
use iced::advanced::renderer;
use iced::advanced::widget::{Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget};
use iced::border::Radius;
use iced::mouse::{Cursor, Interaction};
use iced::{Border, Color, Element, Event, Length, Point, Rectangle, Renderer as BaseRenderer, Size, Theme, mouse, window};
use iced_anim::AnimatedState;
use iced_color_extension::rgb;

use super::anim;

/// Knob inset from the track edge — 2px inside a 20px track leaves a 16px knob.
const KNOB_INSET: f32 = 2.0;

/// An animated, self-contained switch (`.toggle`).
pub struct Toggle<Message> {
    is_on: bool,
    on_toggle: Option<OnToggle<Message>>,
    width: Length,
    height: Length,
    disabled: bool,
}

/// How a flip produces its message — a fixed message, or a closure receiving
/// the new state (`!is_on`).
enum OnToggle<Message> {
    Direct(Message),
    WithState(Box<dyn Fn(bool) -> Message>),
}

impl<Message: Clone> OnToggle<Message> {
    fn get(&self, is_on: bool) -> Message {
        match self {
            OnToggle::Direct(message) => message.clone(),
            OnToggle::WithState(closure) => closure(!is_on),
        }
    }
}

/// Widget-local interaction status. The on/off state is folded in so any
/// programmatic flip changes the status — which is what triggers the
/// retarget-and-redraw loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Status {
    OnIdle,
    OnHovered,
    OnPressed,
    OffIdle,
    OffHovered,
    OffPressed,
    /// Carries the on/off state: disabled controls ignore hover/press, but a
    /// bound value can still change underneath them.
    Disabled(bool),
}

impl Status {
    fn is_on(self) -> bool {
        matches!(self, Status::OnIdle | Status::OnHovered | Status::OnPressed) || matches!(self, Status::Disabled(true))
    }
}

/// The animated appearance of a [`Toggle`].
#[derive(Debug, Clone, PartialEq, iced_anim::Animate)]
struct Visual {
    /// Knob travel: `0.0` = off, `1.0` = on.
    knob: f32,
    /// Track fill.
    track: Color,
    /// Knob fill.
    knob_color: Color,
}

#[derive(Debug)]
struct State {
    is_pressed: bool,
    animated_state: AnimatedState<Status, Visual>,
}

impl<Message> Toggle<Message> {
    /// Creates a toggle in the given state.
    pub fn new(is_on: bool) -> Self {
        Self {
            is_on,
            on_toggle: None,
            width: Length::Fixed(34.0),
            height: Length::Fixed(20.0),
            disabled: false,
        }
    }

    /// Sets the message produced when the toggle is flipped.
    ///
    /// Without an `on_toggle` message the control is disabled.
    pub fn on_toggle(mut self, on_toggle: Message) -> Self {
        self.on_toggle = Some(OnToggle::Direct(on_toggle));
        self
    }

    /// Sets a closure producing the message from the toggle's new state.
    pub fn on_toggle_with(mut self, on_toggle: impl Fn(bool) -> Message + 'static) -> Self {
        self.on_toggle = Some(OnToggle::WithState(Box::new(on_toggle)));
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

    /// Disables interaction and fades the control.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    fn status_for(&self, is_pressed: bool, is_over: bool) -> Status {
        if self.disabled {
            return Status::Disabled(self.is_on);
        }

        match (self.is_on, is_pressed && is_over, is_over) {
            (true, true, _) => Status::OnPressed,
            (true, _, true) => Status::OnHovered,
            (true, _, _) => Status::OnIdle,
            (false, true, _) => Status::OffPressed,
            (false, _, true) => Status::OffHovered,
            (false, _, _) => Status::OffIdle,
        }
    }

    /// The status the toggle mounts in — settled, no fly-in.
    fn initial_status(&self) -> Status {
        self.status_for(false, false)
    }

    /// The target appearance for the given status.
    fn visual(&self, status: Status) -> Visual {
        let knob = if status.is_on() { 1.0 } else { 0.0 };

        let (track, knob_color) = match status {
            // The guide fades the whole control when disabled.
            Status::Disabled(_) if self.is_on => (anim::fade(color::Primary::DEFAULT, 0.4), anim::fade(color::Primary::ON, 0.7)),
            Status::Disabled(_) => (rgb!(255, 255, 255, 5%), color::Text::DISABLED),
            Status::OnPressed => (color::Primary::DIM, color::Primary::ON),
            // `.btn-primary:hover` — brightness(1.06) ≈ `#2aff8c`.
            Status::OnHovered => (iced::color!(0x2AFF8C), color::Primary::ON),
            Status::OnIdle => (color::Primary::DEFAULT, color::Primary::ON),
            Status::OffPressed => (rgb!(255, 255, 255, 6%), rgb!(255, 255, 255, 80%)),
            Status::OffHovered => (rgb!(255, 255, 255, 14%), Color::WHITE),
            Status::OffIdle => (rgb!(255, 255, 255, 10%), Color::WHITE),
        };

        Visual { knob, track, knob_color }
    }
}

impl<Message: Clone> Widget<Message, Theme, BaseRenderer> for Toggle<Message> {
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
        // The animated value is created lazily at the current status, so the
        // toggle mounts settled — no fly-in.
        tree::State::new(State {
            is_pressed: false,
            animated_state: AnimatedState::new(self.initial_status(), anim::micro()),
        })
    }

    fn diff(&self, tree: &mut Tree) {
        let state = tree.state.downcast_mut::<State>();
        state.animated_state.diff(anim::micro());
    }

    fn layout(&mut self, _tree: &mut Tree, _renderer: &BaseRenderer, limits: &Limits) -> Node {
        layout::atomic(limits, self.width, self.height)
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: Cursor,
        _renderer: &BaseRenderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        if shell.is_event_captured() {
            return;
        }

        let is_over = cursor.is_over(layout.bounds());
        let state = tree.state.downcast_mut::<State>();

        // Redraw whenever the status changes or an animation is in flight.
        let status = self.status_for(state.is_pressed, is_over);
        if state.animated_state.needs_redraw(status) {
            shell.request_redraw();
        }

        match event {
            Event::Window(window::Event::RedrawRequested(now)) => {
                state.animated_state.tick(*now);
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if is_over && self.on_toggle.is_some() && !self.disabled {
                    state.is_pressed = true;
                    shell.request_redraw();
                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) if state.is_pressed => {
                state.is_pressed = false;
                shell.request_redraw();
                if is_over && let Some(on_toggle) = self.on_toggle.as_ref() {
                    shell.publish(on_toggle.get(self.is_on));
                }
                shell.capture_event();
            }
            _ => {}
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
        // Lazily retargets the animation when the computed visual differs —
        // this is what makes both click-driven and programmatic flips animate.
        let visual = state.animated_state.current_value(|status| self.visual(*status));
        let bounds = layout.bounds();

        // Track — fully rounded pill.
        renderer.fill_quad(
            renderer::Quad {
                bounds,
                border: Border {
                    radius: Radius::from(bounds.height / 2.0),
                    ..Border::default()
                },
                ..renderer::Quad::default()
            },
            visual.track,
        );

        // Knob — travels from the left inset to the right inset.
        let knob_diameter = bounds.height - KNOB_INSET * 2.0;
        let knob_x = bounds.x + KNOB_INSET + visual.knob * (bounds.width - bounds.height);
        let knob_bounds = Rectangle::new(Point::new(knob_x, bounds.y + KNOB_INSET), Size::new(knob_diameter, knob_diameter));
        renderer.fill_quad(
            renderer::Quad {
                bounds: knob_bounds,
                border: Border {
                    radius: Radius::from(knob_diameter / 2.0),
                    ..Border::default()
                },
                ..renderer::Quad::default()
            },
            visual.knob_color,
        );
    }

    fn mouse_interaction(&self, _tree: &Tree, layout: Layout<'_>, cursor: Cursor, _viewport: &Rectangle, _renderer: &BaseRenderer) -> Interaction {
        if cursor.is_over(layout.bounds()) && self.on_toggle.is_some() && !self.disabled {
            Interaction::Pointer
        } else {
            Interaction::default()
        }
    }
}

impl<'a, Message: Clone + 'a> From<Toggle<Message>> for Element<'a, Message, Theme, BaseRenderer> {
    fn from(toggle: Toggle<Message>) -> Self {
        Element::new(toggle)
    }
}

#[cfg(test)]
#[path = "../../../tests/app/components/toggle.rs"]
mod tests;
