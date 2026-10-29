//! The modal's full-window layer: scrim, card chrome, and the event wall.
//!
//! The layer lays the card out itself instead of using a `column!`: iced lays
//! a shrinking column's children out in order, so a tall body would take all
//! the height and squash the footer to nothing. Here the header and footer
//! are measured first and the (scrollable) body gets what is left.
//!
//! iced's `opaque` only captures mouse presses, and `Stack` stops delivering
//! an event once a layer captures it (`iced_widget` `stack.rs`). So after its
//! children have had their turn, the layer captures every remaining mouse,
//! keyboard and touch event: a focused input underneath gets no keystrokes,
//! the wheel scrolls nothing, and the history buttons do not navigate.

use crate::app::theme::color;
use iced::advanced::Renderer as _;
use iced::advanced::layout::{Limits, Node};
use iced::advanced::widget::{Operation, Tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, overlay, renderer};
use iced::mouse::{Cursor, Interaction};
use iced::{Border, Color, Element, Event, Length, Point, Rectangle, Renderer as BaseRenderer, Shadow, Size, Theme, Vector, keyboard, mouse, touch};
use iced_color_extension::rgb;

/// The smallest gap kept between the card and the window edge.
const MARGIN: f32 = 40.0;
/// `--r-xl` — modals and the hero.
const RADIUS: f32 = 24.0;
/// The dimming over the app behind the modal.
const SCRIM: Color = rgb!(0, 0, 0, 55%);
/// The style guide's popover/modal shadow: `0 30px 80px -20px rgba(0,0,0,.8)`.
const SHADOW: Shadow = Shadow {
    color: rgb!(0, 0, 0, 80%),
    offset: Vector::new(0.0, 30.0),
    blur_radius: 80.0,
};
/// The hairline between the header, body and footer.
const RULE: f32 = 1.0;

/// The layer widget: `[header, body, footer?]` inside a centred card.
pub(super) struct Layer<'a, Message> {
    children: Vec<Element<'a, Message>>,
    width: f32,
    on_close: Option<Message>,
    close_on_scrim: bool,
}

impl<'a, Message> Layer<'a, Message> {
    pub(super) fn new(
        header: Element<'a, Message>,
        body: Element<'a, Message>,
        footer: Option<Element<'a, Message>>,
        width: f32,
        on_close: Option<Message>,
        close_on_scrim: bool,
    ) -> Self {
        let mut children = vec![header, body];
        children.extend(footer);
        Self {
            children,
            width,
            on_close,
            close_on_scrim,
        }
    }

    fn has_footer(&self) -> bool {
        self.children.len() == 3
    }
}

/// The card's rectangle: the span of its laid-out sections.
fn card_bounds(layout: Layout<'_>) -> Rectangle {
    let mut sections = layout.children().map(|section| section.bounds());
    let first = sections.next().unwrap_or_default();
    sections.fold(first, |card, section| card.union(&section))
}

/// Card height for sections of the given heights, rules included.
pub(super) fn card_height(header: f32, body: f32, footer: Option<f32>) -> f32 {
    match footer {
        Some(footer) => header + RULE + body + RULE + footer,
        None => header + RULE + body,
    }
}

/// Room left for the body inside a card at most `max_height` tall.
pub(super) fn body_room(max_height: f32, header: f32, footer: Option<f32>) -> f32 {
    (max_height - card_height(header, 0.0, footer)).max(0.0)
}

impl<Message: Clone> Widget<Message, Theme, BaseRenderer> for Layer<'_, Message> {
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.children);
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &BaseRenderer, limits: &Limits) -> Node {
        let size = limits.resolve(Length::Fill, Length::Fill, Size::ZERO);
        let card_width = self.width.min(size.width - 2.0 * MARGIN).max(0.0);
        let max_height = (size.height - 2.0 * MARGIN).max(0.0);
        // Every section spans the card's full width.
        let section = |height: f32| Limits::new(Size::new(card_width, 0.0), Size::new(card_width, height));
        let has_footer = self.has_footer();

        let header = self.children[0]
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, &section(max_height));
        let header_height = header.size().height;
        let footer = has_footer.then(|| {
            let room = (max_height - header_height - 2.0 * RULE).max(0.0);
            self.children[2].as_widget_mut().layout(&mut tree.children[2], renderer, &section(room))
        });
        let footer_height = footer.as_ref().map(|footer| footer.size().height);
        let body = self.children[1].as_widget_mut().layout(
            &mut tree.children[1],
            renderer,
            &section(body_room(max_height, header_height, footer_height)),
        );
        let body_height = body.size().height;

        let height = card_height(header_height, body_height, footer_height);
        let origin = Point::new((size.width - card_width) / 2.0, ((size.height - height) / 2.0).max(0.0));
        let body_y = origin.y + header_height + RULE;
        let mut sections = vec![header.move_to(origin), body.move_to(Point::new(origin.x, body_y))];
        if let Some(footer) = footer {
            sections.push(footer.move_to(Point::new(origin.x, body_y + body_height + RULE)));
        }
        Node::with_children(size, sections)
    }

    fn operate(&mut self, tree: &mut Tree, layout: Layout<'_>, renderer: &BaseRenderer, operation: &mut dyn Operation) {
        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            self.children
                .iter_mut()
                .zip(&mut tree.children)
                .zip(layout.children())
                .for_each(|((child, state), layout)| child.as_widget_mut().operate(state, layout, renderer, operation));
        });
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
        for ((child, state), layout) in self.children.iter_mut().zip(&mut tree.children).zip(layout.children()) {
            child
                .as_widget_mut()
                .update(state, event, layout, cursor, renderer, clipboard, shell, viewport);
        }
        if shell.is_event_captured() {
            return;
        }

        match event {
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(keyboard::key::Named::Escape),
                ..
            }) => {
                if let Some(message) = &self.on_close {
                    shell.publish(message.clone());
                }
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) | Event::Touch(touch::Event::FingerPressed { .. })
                if self.close_on_scrim =>
            {
                let outside = cursor.position().is_some_and(|point| !card_bounds(layout).contains(point));
                if outside && let Some(message) = &self.on_close {
                    shell.publish(message.clone());
                }
            }
            _ => {}
        }

        // The wall: nothing the modal did not use reaches the app below.
        if matches!(event, Event::Mouse(_) | Event::Keyboard(_) | Event::Touch(_)) {
            shell.capture_event();
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
        renderer.fill_quad(
            renderer::Quad {
                bounds: layout.bounds(),
                ..renderer::Quad::default()
            },
            SCRIM,
        );

        let card = card_bounds(layout);
        renderer.fill_quad(
            renderer::Quad {
                bounds: card,
                border: Border {
                    radius: RADIUS.into(),
                    ..Border::default()
                },
                shadow: SHADOW,
                ..renderer::Quad::default()
            },
            color::Surface::BG3,
        );

        for ((child, state), layout) in self.children.iter().zip(&tree.children).zip(layout.children()) {
            child.as_widget().draw(state, renderer, theme, style, layout, cursor, viewport);
        }

        // Hairlines under the header and above the footer.
        let mut sections = layout.children();
        let header = sections.next().map(|header| header.bounds());
        let footer = if self.has_footer() {
            sections.nth(1).map(|footer| footer.bounds())
        } else {
            None
        };
        let rules = [header.map(|header| header.y + header.height), footer.map(|footer| footer.y - RULE)];
        for y in rules.into_iter().flatten() {
            renderer.fill_quad(
                renderer::Quad {
                    bounds: Rectangle::new(Point::new(card.x, y), Size::new(card.width, RULE)),
                    ..renderer::Quad::default()
                },
                color::Surface::LINE,
            );
        }

        // Border ring last, so no section paints over it.
        renderer.fill_quad(
            renderer::Quad {
                bounds: card,
                border: Border {
                    radius: RADIUS.into(),
                    width: 1.0,
                    color: color::Surface::LINE_STRONG,
                },
                ..renderer::Quad::default()
            },
            Color::TRANSPARENT,
        );
    }

    fn mouse_interaction(&self, tree: &Tree, layout: Layout<'_>, cursor: Cursor, viewport: &Rectangle, renderer: &BaseRenderer) -> Interaction {
        let interaction = self
            .children
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
            .map(|((child, state), layout)| child.as_widget().mouse_interaction(state, layout, cursor, viewport, renderer))
            .max()
            .unwrap_or_default();
        // Never `None`: the stack then levitates the cursor for the layers
        // below, so nothing underneath shows a hover state.
        if interaction == Interaction::None {
            Interaction::Idle
        } else {
            interaction
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &BaseRenderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, BaseRenderer>> {
        overlay::from_children(&mut self.children, tree, layout, renderer, viewport, translation)
    }
}

#[cfg(test)]
#[path = "../../../../tests/app/components/modal/layer.rs"]
mod tests;
