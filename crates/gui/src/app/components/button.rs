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

use crate::app::theme::{color, fonts, utility};
use iced::advanced::Renderer as _;
use iced::advanced::layout::{Limits, Node};
use iced::advanced::renderer;
use iced::advanced::widget::{Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget};
use iced::border::Radius;
use iced::mouse::{Cursor, Interaction};
use iced::{Border, Color, Element, Event, Length, Padding, Rectangle, Renderer as BaseRenderer, Shadow, Size, Theme, Vector, mouse, window};
use iced_anim::{AnimatedState, animated::Mode};
use iced_color_extension::rgb;

use super::anim;

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

    fn status_for(&self, is_pressed: bool, is_over: bool) -> Status {
        if self.disabled || self.on_press.is_none() {
            Status::Disabled
        } else if is_pressed && is_over {
            Status::Pressed
        } else if is_over {
            Status::Hovered
        } else {
            Status::Idle
        }
    }

    /// The target appearance for the given status.
    fn visual(&self, status: Status) -> Visual {
        if self.disabled || status == Status::Disabled {
            // The guide fades the whole button to 40% opacity.
            return match self.variant {
                Variant::Primary => Visual {
                    background: rgb!(34, 255, 132, 40%),
                    foreground: anim::fade(color::Primary::ON, 0.4),
                    border: rgb!(34, 255, 132, 12%),
                    lift: 0.0,
                    glow: 0.0,
                },
                Variant::Ghost => Visual {
                    background: rgb!(255, 255, 255, 2%),
                    foreground: color::Text::DISABLED,
                    border: rgb!(255, 255, 255, 3%),
                    lift: 0.0,
                    glow: 0.0,
                },
                Variant::Danger => Visual {
                    background: anim::fade(DANGER, 0.04),
                    foreground: color::Text::DISABLED,
                    border: anim::fade(DANGER, 0.08),
                    lift: 0.0,
                    glow: 0.0,
                },
                Variant::Tab => Visual {
                    background: Color::TRANSPARENT,
                    foreground: color::Text::DISABLED,
                    border: Color::TRANSPARENT,
                    lift: 0.0,
                    glow: 0.0,
                },
            };
        }

        match self.variant {
            Variant::Primary => match status {
                // Hover — `brightness(1.06)`, lift, stronger glow (§07 spec).
                Status::Hovered => Visual {
                    background: iced::color!(0x2AFF8C),
                    foreground: color::Primary::ON,
                    border: rgb!(34, 255, 132, 50%),
                    lift: 1.0,
                    glow: 1.0,
                },
                // Press — translateY(0); the glow hugs the surface.
                Status::Pressed => Visual {
                    background: color::Primary::DIM,
                    foreground: color::Primary::ON,
                    border: rgb!(34, 255, 132, 30%),
                    lift: 0.0,
                    glow: 0.6,
                },
                _ => Visual {
                    background: color::Primary::DEFAULT,
                    foreground: color::Primary::ON,
                    border: rgb!(34, 255, 132, 30%),
                    lift: 0.0,
                    glow: 0.75,
                },
            },
            Variant::Ghost => match status {
                // Hover — white 8%, emphasized border (§07 spec).
                Status::Hovered => Visual {
                    background: rgb!(255, 255, 255, 8%),
                    foreground: color::Text::HEADLINE,
                    border: color::Surface::LINE_STRONG,
                    lift: 1.0,
                    glow: 0.0,
                },
                Status::Pressed => Visual {
                    background: rgb!(255, 255, 255, 6%),
                    foreground: color::Text::HEADLINE,
                    border: color::Surface::LINE_STRONG,
                    lift: 0.0,
                    glow: 0.0,
                },
                _ => Visual {
                    background: rgb!(255, 255, 255, 4%),
                    foreground: color::Text::DEFAULT,
                    border: color::Surface::LINE,
                    lift: 0.0,
                    glow: 0.0,
                },
            },
            // Destructive — red ink on a red tint at rest; hover turns the
            // fill solid red with headline ink.
            Variant::Danger => match status {
                Status::Hovered => Visual {
                    background: DANGER,
                    foreground: color::Text::HEADLINE,
                    border: DANGER,
                    lift: 1.0,
                    glow: 0.0,
                },
                Status::Pressed => Visual {
                    background: anim::fade(DANGER, 0.8),
                    foreground: color::Text::HEADLINE,
                    border: DANGER,
                    lift: 0.0,
                    glow: 0.0,
                },
                _ => Visual {
                    background: anim::fade(DANGER, 0.12),
                    foreground: DANGER,
                    border: anim::fade(DANGER, 0.3),
                    lift: 0.0,
                    glow: 0.0,
                },
            },
            // Tabs are chrome-free: only the label (and the page's
            // underline) changes. Hover brightens the ink and leaves a
            // barely-there tint so the row still feels alive.
            Variant::Tab => match status {
                Status::Hovered => Visual {
                    background: rgb!(255, 255, 255, 3%),
                    foreground: color::Text::HEADLINE,
                    border: Color::TRANSPARENT,
                    lift: 0.0,
                    glow: 0.0,
                },
                Status::Pressed => Visual {
                    background: rgb!(255, 255, 255, 5%),
                    foreground: color::Text::HEADLINE,
                    border: Color::TRANSPARENT,
                    lift: 0.0,
                    glow: 0.0,
                },
                _ => Visual {
                    background: Color::TRANSPARENT,
                    foreground: color::Text::MUTED,
                    border: Color::TRANSPARENT,
                    lift: 0.0,
                    glow: 0.0,
                },
            },
        }
    }

    fn status(&self, state: &State, cursor: Cursor, layout: Layout<'_>) -> Status {
        self.status_for(state.is_pressed, cursor.is_over(layout.bounds()))
    }
}

impl<Message: Clone> Widget<Message, Theme, BaseRenderer> for TextButton<'_, Message> {
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
            is_pressed: false,
            animated_state: AnimatedState::new(Status::Idle, self.animation),
        })
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        let state = tree.state.downcast_mut::<State>();
        state.animated_state.diff(self.animation);
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &BaseRenderer, limits: &Limits) -> Node {
        iced::advanced::layout::padded(limits, self.width, self.height, self.padding, |limits| {
            self.content.as_widget_mut().layout(&mut tree.children[0], renderer, limits)
        })
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: Cursor,
        renderer: &BaseRenderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout.children().next().unwrap(),
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        if shell.is_event_captured() {
            return;
        }

        let state = tree.state.downcast_mut::<State>();
        let status = self.status(state, cursor, layout);
        let needs_redraw = state.animated_state.needs_redraw(status);

        if needs_redraw {
            shell.request_redraw();
        }

        match event {
            Event::Window(window::Event::RedrawRequested(now)) => {
                state.animated_state.tick(*now);
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if self.on_press.is_some() && cursor.is_over(layout.bounds()) {
                    state.is_pressed = true;
                    shell.request_redraw();
                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) if state.is_pressed => {
                state.is_pressed = false;
                shell.request_redraw();
                if cursor.is_over(layout.bounds())
                    && let Some(on_press) = self.on_press.clone()
                {
                    shell.publish(on_press);
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
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State>();
        let visual = state.animated_state.current_value(|status| self.visual(*status));
        let bounds = layout.bounds();
        let content_layout = layout.children().next().unwrap();

        // The whole surface — fill, border, glow and content — lifts together.
        renderer.with_translation(Vector::new(0.0, -visual.lift), |renderer| {
            let shadow = match self.variant {
                Variant::Primary => {
                    // §07 — `0 8px 24px -6px var(--mc-green-glow)`; hover pulls
                    // it closer (12px/32px), press hugs the surface.
                    let (offset_y, blur_radius) = match visual.lift > 0.5 {
                        true => (10.0, 30.0),
                        false if visual.glow < 0.5 => (6.0, 18.0),
                        false => (8.0, 24.0),
                    };
                    Shadow {
                        color: rgb!(34, 255, 132, 35%),
                        offset: Vector::new(0.0, offset_y),
                        blur_radius,
                    }
                }
                Variant::Ghost | Variant::Danger => Shadow {
                    color: rgb!(0, 0, 0, 30%),
                    offset: Vector::new(0.0, 4.0 + visual.lift * 4.0),
                    blur_radius: 12.0 + visual.lift * 8.0,
                },
                Variant::Tab => Shadow {
                    color: Color::TRANSPARENT,
                    offset: Vector::ZERO,
                    blur_radius: 0.0,
                },
            };

            renderer.fill_quad(
                renderer::Quad {
                    bounds,
                    border: Border {
                        radius: self.radius,
                        width: 1.0,
                        color: visual.border,
                    },
                    shadow,
                    ..renderer::Quad::default()
                },
                visual.background,
            );

            self.content.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                &renderer::Style {
                    text_color: visual.foreground,
                },
                content_layout,
                cursor,
                viewport,
            );
        });
    }

    fn mouse_interaction(&self, _tree: &Tree, layout: Layout<'_>, cursor: Cursor, _viewport: &Rectangle, _renderer: &BaseRenderer) -> Interaction {
        if cursor.is_over(layout.bounds()) && self.on_press.is_some() && !self.disabled {
            Interaction::Pointer
        } else {
            Interaction::default()
        }
    }
}

impl<'a, Message: Clone + 'a> From<TextButton<'a, Message>> for Element<'a, Message, Theme, BaseRenderer> {
    fn from(button: TextButton<'a, Message>) -> Self {
        Element::new(button)
    }
}

#[cfg(test)]
#[path = "../../../tests/app/components/button.rs"]
mod tests;
