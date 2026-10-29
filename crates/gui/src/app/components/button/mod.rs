//! Text buttons — the style guide's `.btn-primary` and `.btn-ghost` (§07).
//!
//! Primary is the one loud thing on screen: a Lode Green fill with on-green
//! ink and a glow shadow. Ghost is the quiet counterpart — white 4% fill with
//! a hairline border. Danger is the destructive counterpart: a red tint that
//! turns solid on hover. Hover lifts the button 1px and brightens it; press
//! settles it back down. All of it eases on the §08 timings (Micro/Lift).
//!
//! ```text
//! TextButton::primary("Play").on_press(Message::Play)
//! TextButton::ghost("Edit instance").on_press(Message::Edit)
//! ```
//!
//! For icon-only buttons prefer [`super::icon_button::IconButton`]; for an
//! icon plus label, build the row yourself and wrap it with
//! [`TextButton::content`].

mod style;
mod widget;

use crate::app::components::anim;
use crate::app::theme::{color, fonts, utility};
use iced::border::Radius;
use iced::{Color, Element, Length, Padding, Renderer as BaseRenderer, Theme};
use iced_anim::{AnimatedState, animated::Mode};

/// Default `.btn-primary` padding — 12px vertical, 20px horizontal.
pub const PRIMARY_PADDING: Padding = Padding {
    top: 12.0,
    bottom: 12.0,
    left: 20.0,
    right: 20.0,
};

/// Default `.btn-ghost` padding — 10px vertical, 16px horizontal.
const GHOST_PADDING: Padding = Padding {
    top: 10.0,
    bottom: 10.0,
    left: 16.0,
    right: 16.0,
};

/// The destructive red — the danger palette's hover fill.
const DANGER: Color = color::buttons::Danger::BACKGROUND_HOVER;

/// The visual treatment of a text button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Variant {
    /// `.btn-primary` — green fill, on-green ink, glow shadow.
    Primary,
    /// `.btn-ghost` — white 4% fill, hairline border, `--ink-1` label.
    Ghost,
    /// Chrome-free tab: no fill, no border, no lift — the label (and an
    /// external underline) carries the state.
    Tab,
    /// Destructive action — red ink on a red tint that turns solid on hover.
    Danger,
}

/// Widget-local interaction status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Idle,
    Hovered,
    Pressed,
    Disabled,
}

/// The animated appearance of a [`TextButton`].
#[derive(Debug, Clone, PartialEq, iced_anim::Animate)]
struct Visual {
    /// Fill.
    background: Color,
    /// Label ink.
    foreground: Color,
    /// Border color (hairline ring).
    border: Color,
    /// Hover lift in logical pixels — 0 or 1.
    lift: f32,
    /// Glow strength, `0.0..=1.0`, scaling the guide's green shadow.
    glow: f32,
}

#[derive(Debug)]
struct State {
    is_pressed: bool,
    animated_state: AnimatedState<Status, Visual>,
}

/// An animated text button (`.btn-primary` / `.btn-ghost`).
pub struct TextButton<'a, Message> {
    content: Element<'a, Message, Theme, BaseRenderer>,
    on_press: Option<Message>,
    variant: Variant,
    width: Length,
    height: Length,
    padding: Padding,
    radius: Radius,
    disabled: bool,
    animation: Mode,
}

impl<'a, Message: Clone + 'a> TextButton<'a, Message> {
    /// Creates a button wrapping arbitrary content.
    pub fn content(content: impl Into<Element<'a, Message, Theme, BaseRenderer>>) -> Self {
        Self {
            content: content.into(),
            on_press: None,
            variant: Variant::Ghost,
            width: Length::Shrink,
            height: Length::Shrink,
            padding: GHOST_PADDING,
            radius: Radius::from(12.0),
            disabled: false,
            animation: anim::lift().into(),
        }
    }

    /// `.btn-primary` — bold 14px label in on-green ink.
    pub fn primary(label: impl Into<String>) -> Self {
        Self::content(iced::widget::Text::new(label.into()).size(14.0).line_height(1.0).font(fonts::Inter::BOLD))
            .variant(Variant::Primary)
            .padding(PRIMARY_PADDING)
    }

    /// `.btn-ghost` — medium 13px label in `--ink-1`.
    pub fn ghost(label: impl Into<String>) -> Self {
        Self::content(
            iced::widget::Text::new(label.into())
                .size(13.0)
                .line_height(1.0)
                .font(fonts::Inter::MEDIUM),
        )
        .variant(Variant::Ghost)
        .padding(GHOST_PADDING)
    }

    /// An icon followed by a label — the Import and New world style buttons.
    pub fn labeled_icon(icon: &'static str, label: impl Into<String>, variant: Variant) -> Self {
        let (ink, padding) = match variant {
            Variant::Primary => (color::Primary::ON, PRIMARY_PADDING),
            _ => (color::Text::DEFAULT, GHOST_PADDING),
        };
        Self::content(
            iced::widget::row![
                utility::icon(icon, 14.0, Some((ink, ink))),
                iced::widget::Text::new(label.into())
                    .size(13.0)
                    .line_height(1.0)
                    .font(fonts::Inter::MEDIUM)
                    .color(ink),
            ]
            .spacing(8)
            .align_y(iced::alignment::Vertical::Center),
        )
        .variant(variant)
        .padding(padding)
    }

    /// Overrides the style variant — for custom content that needs the
    /// primary look (green fill, on-green ink) instead of the ghost default.
    pub fn variant(mut self, variant: Variant) -> Self {
        self.variant = variant;
        self
    }

    /// Sets the message produced when the button is pressed.
    ///
    /// Without an `on_press` message the button is disabled.
    pub fn on_press(mut self, on_press: Message) -> Self {
        self.on_press = Some(on_press);
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

    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = padding.into();
        self
    }

    pub fn radius(mut self, radius: impl Into<Radius>) -> Self {
        self.radius = radius.into();
        self
    }

    /// Overrides the easing driving the hover lift (§08 motion presets).
    pub fn animation(mut self, motion: impl Into<Mode>) -> Self {
        self.animation = motion.into();
        self
    }

    /// Disables interaction and fades the button to 40% (§07 spec).
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl<'a, Message: Clone + 'a> From<TextButton<'a, Message>> for Element<'a, Message, Theme, BaseRenderer> {
    fn from(button: TextButton<'a, Message>) -> Self {
        Element::new(button)
    }
}

#[cfg(test)]
#[path = "../../../../tests/app/components/button/mod.rs"]
mod tests;
