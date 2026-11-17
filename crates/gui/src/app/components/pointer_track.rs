//! Pointer tracking for a drag — reports where the pointer is, and the release that ends the drag, while `armed`.
//!
//! `mouse_area` reports a move only while the pointer is over it, and a release only inside it, so a drag that leaves
//! the area never ends. It also reports every move, armed or not, and each report rebuilds the view. This widget keeps
//! no state of its own: while the view says a drag is armed it reports each move, with the position relative to its
//! own top-left corner, and the release of the left button wherever the pointer is. Otherwise it reports nothing.
//!
//! ```text
//! PointerTrack::new(list, drag.is_armed(), Message::Moved, Message::Released)
//! ```

use iced::advanced::layout::{Limits, Node};
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget};
use iced::mouse::{self, Cursor, Interaction};
use iced::{Element, Event, Length, Point, Rectangle, Renderer as BaseRenderer, Size, Theme, Vector};

/// Wraps `content` and, while `armed`, publishes each pointer move and the left button's release. Built with
/// [`PointerTrack::new`].
pub struct PointerTrack<'a, Message> {
    content: Element<'a, Message>,
    armed: bool,
    on_move: Box<dyn Fn(Point) -> Message + 'a>,
    on_release: Message,
}

impl<'a, Message: Clone> PointerTrack<'a, Message> {
    /// Wraps `content`. `armed` is whether the view has a press it is following.
    pub fn new(content: impl Into<Element<'a, Message>>, armed: bool, on_move: impl Fn(Point) -> Message + 'a, on_release: Message) -> Self {
        Self {
            content: content.into(),
            armed,
            on_move: Box::new(on_move),
            on_release,
        }
    }
}

/// The pointer's position relative to the corner at `origin`.
fn relative(pointer: Point, origin: Point) -> Point {
    Point::new(pointer.x - origin.x, pointer.y - origin.y)
}

impl<Message: Clone> Widget<Message, Theme, BaseRenderer> for PointerTrack<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::stateless()
    }

    fn state(&self) -> tree::State {
        tree::State::None
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
        self.content
            .as_widget_mut()
            .update(&mut tree.children[0], event, layout, cursor, renderer, clipboard, shell, viewport);

        if !self.armed {
            return;
        }
        // Read from the event, not the cursor: a layer above may have hidden the cursor from this one, and the drag
        // still has to follow the pointer and end.
        match event {
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                shell.publish((self.on_move)(relative(*position, layout.bounds().position())));
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => shell.publish(self.on_release.clone()),
            _ => {}
        }
    }

    fn mouse_interaction(&self, tree: &Tree, layout: Layout<'_>, cursor: Cursor, viewport: &Rectangle, renderer: &BaseRenderer) -> Interaction {
        self.content
            .as_widget()
            .mouse_interaction(&tree.children[0], layout, cursor, viewport, renderer)
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
        self.content
            .as_widget()
            .draw(&tree.children[0], renderer, theme, style, layout, cursor, viewport);
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &BaseRenderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, BaseRenderer>> {
        self.content
            .as_widget_mut()
            .overlay(&mut tree.children[0], layout, renderer, viewport, translation)
    }
}

impl<'a, Message: Clone + 'a> From<PointerTrack<'a, Message>> for Element<'a, Message, Theme, BaseRenderer> {
    fn from(track: PointerTrack<'a, Message>) -> Self {
        Element::new(track)
    }
}

#[cfg(test)]
#[path = "../../../tests/app/components/pointer_track.rs"]
mod tests;
