//! Hover scale — grows its content while the pointer is over it, and lights a ring around it.
//!
//! The growth is a transformation, not an overlay. A scale above one drawn through `float` is
//! an overlay, and while an overlay reports a pointer interaction iced hides the pointer from the
//! widgets underneath. A context menu around a hovered card then never saw its right-press. This
//! widget adds no overlay, so the pointer reaches everything beneath it.
//!
//! The target comes from the view (`hovered`), so the content eases toward it on window frames
//! with no subscription. The easing is [`anim::lift`]. The hit areas follow the drawing: the
//! content receives the pointer where it is drawn.
//!
//! ```text
//! HoverScale::new(card, hovered).ring(14.0, color)
//! ```

use iced::advanced::layout::{Limits, Node};
use iced::advanced::overlay;
use iced::advanced::renderer::{self, Quad};
use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Renderer as _, Shell, Widget};
use iced::border::Radius;
use iced::mouse::{Cursor, Interaction};
use iced::{Background, Border, Color, Element, Event, Length, Rectangle, Renderer as BaseRenderer, Size, Theme, Transformation, Vector, window};
use iced_anim::Animated;

use super::anim;

/// How far a fully hovered widget grows, as a fraction of its size.
pub const GROWTH: f32 = 0.03;

/// The scale at an eased `progress`, which is clamped to `[0, 1]`.
pub(crate) fn scale_at(progress: f32, growth: f32) -> f32 {
    1.0 + growth * progress.clamp(0.0, 1.0)
}

/// The transformation that grows `bounds` by `scale` about their centre.
fn about_centre(bounds: Rectangle, scale: f32) -> Transformation {
    Transformation::translate(bounds.center_x(), bounds.center_y())
        * Transformation::scale(scale)
        * Transformation::translate(-bounds.center_x(), -bounds.center_y())
}

/// The eased progress from rest (0) to fully hovered (1).
#[derive(Debug)]
struct State {
    progress: Animated<f32>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            progress: Animated::new(0.0, anim::lift()),
        }
    }
}

/// The content's progress right now, in `[0, 1]`.
fn progress_of(tree: &Tree) -> f32 {
    tree.state.downcast_ref::<State>().progress.value().clamp(0.0, 1.0)
}

/// A widget that grows its content while `hovered`. Built with [`HoverScale::new`].
pub struct HoverScale<'a, Message> {
    content: Element<'a, Message>,
    hovered: bool,
    ring: Option<(f32, Color)>,
}

impl<'a, Message> HoverScale<'a, Message> {
    /// Wraps `content`. It grows while `hovered` is true.
    pub fn new(content: impl Into<Element<'a, Message>>, hovered: bool) -> Self {
        Self {
            content: content.into(),
            hovered,
            ring: None,
        }
    }

    /// Draws a ring of `color` around the content, at `radius`. The ring fades in with the growth.
    pub fn ring(mut self, radius: f32, color: Color) -> Self {
        self.ring = Some((radius, color));
        self
    }
}

impl<Message: 'static> Widget<Message, Theme, BaseRenderer> for HoverScale<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &BaseRenderer, limits: &Limits) -> Node {
        // Set here rather than in `diff`, because a freshly built tree is never diffed.
        let target = if self.hovered { 1.0 } else { 0.0 };
        tree.state.downcast_mut::<State>().progress.set_target(target);
        self.content.as_widget_mut().layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(&mut self, tree: &mut Tree, layout: Layout<'_>, renderer: &BaseRenderer, operation: &mut dyn Operation) {
        self.content.as_widget_mut().operate(&mut tree.children[0], layout, renderer, operation);
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
        // The content sees the pointer and the viewport where they are drawn.
        let inverse = about_centre(layout.bounds(), scale_at(progress_of(tree), GROWTH)).inverse();
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor * inverse,
            renderer,
            clipboard,
            shell,
            &(*viewport * inverse),
        );

        if let Event::Window(window::Event::RedrawRequested(now)) = event {
            let state = tree.state.downcast_mut::<State>();
            state.progress.tick(*now);
            if state.progress.is_animating() {
                shell.request_redraw();
            }
        }
    }

    fn mouse_interaction(&self, tree: &Tree, layout: Layout<'_>, cursor: Cursor, viewport: &Rectangle, renderer: &BaseRenderer) -> Interaction {
        let inverse = about_centre(layout.bounds(), scale_at(progress_of(tree), GROWTH)).inverse();
        self.content
            .as_widget()
            .mouse_interaction(&tree.children[0], layout, cursor * inverse, &(*viewport * inverse), renderer)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut BaseRenderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
    ) {
        let progress = progress_of(tree);
        let bounds = layout.bounds();
        let transformation = about_centre(bounds, scale_at(progress, GROWTH));
        let inverse = transformation.inverse();
        renderer.with_transformation(transformation, |renderer| {
            self.content.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                style,
                layout,
                cursor * inverse,
                &(*viewport * inverse),
            );
            if let Some((radius, color)) = self.ring.filter(|_| progress > 0.0) {
                renderer.fill_quad(
                    Quad {
                        bounds,
                        border: Border {
                            radius: Radius::from(radius),
                            width: 1.0,
                            color: anim::fade(color, progress),
                        },
                        ..Quad::default()
                    },
                    Background::Color(Color::TRANSPARENT),
                );
            }
        });
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &BaseRenderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, BaseRenderer>> {
        // The content's own overlays are not grown; this widget has none of its own.
        self.content
            .as_widget_mut()
            .overlay(&mut tree.children[0], layout, renderer, viewport, translation)
    }
}

impl<'a, Message: 'static + 'a> From<HoverScale<'a, Message>> for Element<'a, Message, Theme, BaseRenderer> {
    fn from(hover: HoverScale<'a, Message>) -> Self {
        Element::new(hover)
    }
}

#[cfg(test)]
#[path = "../../../tests/app/components/hover_scale.rs"]
mod tests;
