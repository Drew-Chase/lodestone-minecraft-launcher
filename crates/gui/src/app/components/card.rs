//! Card — the style guide's `.card` surface (§07), with the hover treatment
//! baked in.
//!
//! A card is a glass panel — white 3.5%→1.5% vertical gradient over a 1px
//! hairline — with optional edge-to-edge cover art above padded content.
//! Hovering lifts the whole card 2px, fades its border to Lode Green 25% and
//! blooms the deep shadow (§07 `.card:hover`, §08 Lift).
//!
//! ```text
//! Card::new(cover_element, content_element)
//! ```

use crate::app::theme::color;
use iced::advanced::Renderer as _;
use iced::advanced::layout::{Limits, Node};
use iced::advanced::renderer;
use iced::advanced::widget::{Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget};
use iced::border::Radius;
use iced::mouse::{Cursor, Interaction};
use iced::{
    Border, Color, Element, Event, Gradient, Length, Padding, Point, Rectangle, Renderer as BaseRenderer, Shadow, Size, Theme, Vector, gradient,
    window,
};
use iced_anim::Animated;
use iced_color_extension::rgb;

use super::anim;

/// Hover lift — `.card:hover { transform: translateY(-2px) }`.
const HOVER_LIFT: f32 = 2.0;

/// An animated card (`.card`).
pub struct Card<'a, Message> {
    cover: Option<Element<'a, Message, Theme, BaseRenderer>>,
    content: Element<'a, Message, Theme, BaseRenderer>,
    padding: Padding,
    radius: Radius,
    width: Length,
    height: Length,
    /// Whether hover effects apply — static surfaces sit still.
    interactive: bool,
}

#[derive(Debug)]
struct State {
    hovered: bool,
    /// Animated hover progress: `0.0` at rest, `1.0` fully hovered.
    hover: Animated<f32>,
}

impl<'a, Message> Card<'a, Message> {
    /// Creates a card with optional cover art and its content.
    pub fn new(
        cover: impl Into<Option<Element<'a, Message, Theme, BaseRenderer>>>,
        content: impl Into<Element<'a, Message, Theme, BaseRenderer>>,
    ) -> Self {
        Self {
            cover: cover.into(),
            content: content.into(),
            padding: Padding::new(14.0),
            radius: Radius::from(18.0),
            width: Length::Fill,
            height: Length::Shrink,
            interactive: true,
        }
    }

    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = padding.into();
        self
    }

    pub fn radius(mut self, radius: impl Into<Radius>) -> Self {
        self.radius = radius.into();
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

    /// Disables the hover lift/glow — for static, non-clickable surfaces.
    pub fn interactive(mut self, interactive: bool) -> Self {
        self.interactive = interactive;
        self
    }

    fn has_cover(&self) -> bool {
        self.cover.is_some()
    }

    /// The glass fill — white 3.5% fading to 1.5%, top to bottom (§07 `.card`).
    fn surface_gradient() -> Gradient {
        Gradient::Linear(
            gradient::Linear::new(std::f32::consts::FRAC_PI_2)
                .add_stop(0.0, rgb!(255, 255, 255, 4%))
                .add_stop(1.0, rgb!(255, 255, 255, 2%)),
        )
    }

    /// Border color — hairline at rest, Lode Green 25% on hover (§07 spec).
    fn border_color(hover: f32) -> Color {
        anim::mix(color::Surface::LINE, rgb!(34, 255, 132, 25%), hover)
    }

    /// Shadow — deep and soft; the offset grows slightly on hover.
    fn shadow(hover: f32) -> Shadow {
        Shadow {
            color: rgb!(0, 0, 0, 60%),
            offset: Vector::new(0.0, 20.0 + hover * 4.0),
            blur_radius: 48.0,
        }
    }
}

impl<Message> Widget<Message, Theme, BaseRenderer> for Card<'_, Message> {
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
            hovered: false,
            hover: Animated::new(0.0, anim::lift()),
        })
    }

    fn children(&self) -> Vec<Tree> {
        match &self.cover {
            Some(cover) => vec![Tree::new(cover), Tree::new(&self.content)],
            None => vec![Tree::new(&self.content)],
        }
    }

    fn diff(&self, tree: &mut Tree) {
        match &self.cover {
            Some(cover) => tree.diff_children(&[cover, &self.content]),
            None => tree.diff_children(std::slice::from_ref(&self.content)),
        }
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &BaseRenderer, limits: &Limits) -> Node {
        let max = limits.max();

        // The cover is edge to edge; the content respects the padding.
        let content_limits = Limits::new(
            Size::ZERO,
            Size::new((max.width - self.padding.x()).max(0.0), (max.height - self.padding.y()).max(0.0)),
        );

        let mut children = Vec::new();
        let mut y = 0.0;
        let mut max_width: f32 = 0.0;

        if let Some(cover) = &mut self.cover {
            let node = cover.as_widget_mut().layout(&mut tree.children[0], renderer, limits);
            let size = node.size();
            max_width = max_width.max(size.width);
            let positioned = node.move_to(Point::new(0.0, 0.0));
            y += size.height;
            children.push(positioned);
        }

        let content_index = if self.has_cover() { 1 } else { 0 };
        let content_node = self
            .content
            .as_widget_mut()
            .layout(&mut tree.children[content_index], renderer, &content_limits);
        let content_size = content_node.size();
        max_width = max_width.max(content_size.width + self.padding.x());
        let positioned = content_node.move_to(Point::new(self.padding.left, y + self.padding.top));
        y += content_size.height + self.padding.y();
        children.push(positioned);

        let intrinsic = Size::new(max_width, y);
        let size = limits.resolve(self.width, self.height, intrinsic);
        Node::with_children(size, children)
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
        let content_index = if self.has_cover() { 1 } else { 0 };

        // Let the children see events first (buttons inside the card, etc.).
        self.content.as_widget_mut().update(
            &mut tree.children[content_index],
            event,
            layout.children().nth(content_index).unwrap(),
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
        if self.has_cover() {
            let cover = self.cover.as_mut().unwrap();
            cover.as_widget_mut().update(
                &mut tree.children[0],
                event,
                layout.children().next().unwrap(),
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );
        }

        if shell.is_event_captured() {
            return;
        }

        let state = tree.state.downcast_mut::<State>();
        let hovered = self.interactive && cursor.is_over(layout.bounds());
        if state.hovered != hovered {
            state.hovered = hovered;
        }
        let target = if hovered { 1.0 } else { 0.0 };
        if state.hover.target() != &target {
            state.hover.set_target(target);
            shell.request_redraw();
        }

        if let Event::Window(window::Event::RedrawRequested(now)) = event {
            state.hover.tick(*now);
            if state.hover.is_animating() {
                shell.request_redraw();
            }
        } else if state.hover.is_animating() {
            shell.request_redraw();
        }
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
        let state = tree.state.downcast_ref::<State>();
        let hover = *state.hover.value();
        let bounds = layout.bounds();
        let content_index = if self.has_cover() { 1 } else { 0 };

        // The whole card lifts together — fill, shadow and content.
        renderer.with_translation(Vector::new(0.0, -HOVER_LIFT * hover), |renderer| {
            renderer.fill_quad(
                renderer::Quad {
                    bounds,
                    border: Border {
                        radius: self.radius,
                        width: 1.0,
                        color: Color::TRANSPARENT,
                    },
                    shadow: Self::shadow(hover),
                    ..renderer::Quad::default()
                },
                Self::surface_gradient(),
            );

            // Children, clipped to the card's rectangle so oversized cover art
            // cannot bleed onto neighbors.
            renderer.with_layer(bounds, |renderer| {
                let mut children = layout.children();
                if self.has_cover() {
                    self.cover.as_ref().unwrap().as_widget().draw(
                        &tree.children[0],
                        renderer,
                        theme,
                        style,
                        children.next().unwrap(),
                        cursor,
                        viewport,
                    );
                }
                self.content.as_widget().draw(
                    &tree.children[content_index],
                    renderer,
                    theme,
                    style,
                    children.next().unwrap(),
                    cursor,
                    viewport,
                );
            });

            // Border ring last — cover art can never paint over it.
            renderer.fill_quad(
                renderer::Quad {
                    bounds,
                    border: Border {
                        radius: self.radius,
                        width: 1.0,
                        color: Self::border_color(hover),
                    },
                    ..renderer::Quad::default()
                },
                Color::TRANSPARENT,
            );
        });
    }

    fn mouse_interaction(&self, tree: &Tree, layout: Layout<'_>, cursor: Cursor, viewport: &Rectangle, renderer: &BaseRenderer) -> Interaction {
        let content_index = if self.has_cover() { 1 } else { 0 };
        let content_interaction = self.content.as_widget().mouse_interaction(
            &tree.children[content_index],
            layout.children().nth(content_index).unwrap(),
            cursor,
            viewport,
            renderer,
        );
        if content_interaction != Interaction::None {
            return content_interaction;
        }
        if let Some(cover) = &self.cover {
            return cover
                .as_widget()
                .mouse_interaction(&tree.children[0], layout.children().next().unwrap(), cursor, viewport, renderer);
        }
        Interaction::default()
    }
}

impl<'a, Message: 'a> From<Card<'a, Message>> for Element<'a, Message, Theme, BaseRenderer> {
    fn from(card: Card<'a, Message>) -> Self {
        Element::new(card)
    }
}
