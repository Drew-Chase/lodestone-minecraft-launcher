//! The [`Widget`] implementation for [`TextButton`]: layout, input handling,
//! and drawing, plus the interaction status those share.

use crate::app::theme::color;
use iced::advanced::Renderer as _;
use iced::advanced::layout::{Limits, Node};
use iced::advanced::renderer;
use iced::advanced::widget::{Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget};
use iced::mouse::{Cursor, Interaction};
use iced::{Border, Color, Event, Length, Rectangle, Renderer as BaseRenderer, Shadow, Size, Theme, Vector, mouse, window};
use iced_anim::AnimatedState;
use iced_color_extension::rgb;

use super::{State, Status, TextButton, Variant};

/// The strength of a primary button's glow at full [`Visual::glow`]. The glow colour is
/// `--mc-green-glow` at 35%; this scales it down to the reference's slight halo.
const PRIMARY_GLOW_ALPHA: f32 = 0.30;

impl<'a, Message: Clone + 'a> TextButton<'a, Message> {
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
                    // §07 — `0 8px 24px -6px var(--mc-green-glow)`. iced has no spread, so the
                    // blur stays short to keep the halo as slight as the reference. `glow`
                    // scales the strength, so the halo eases with hover and press.
                    let strength = visual.glow;
                    Shadow {
                        color: Color {
                            a: PRIMARY_GLOW_ALPHA * strength,
                            ..color::Primary::GLOW
                        },
                        offset: Vector::new(0.0, 2.0 + 4.0 * strength),
                        blur_radius: 4.0 + 12.0 * strength,
                    }
                }
                Variant::Ghost | Variant::Danger | Variant::Destructive => Shadow {
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
