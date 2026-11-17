//! Size tracking — reports the size its content was laid out at, judged against the size the view already knows.
//!
//! A view cannot ask how big a widget is, but some state needs to know: an editor that keeps its caret on screen has
//! to know how tall its scroll is. This widget keeps no state of its own. It compares the size of its laid-out bounds
//! with `known`, the size the view passes in, and reports the new size whenever the two differ. The first event after
//! a resize therefore brings the state up to date.
//!
//! ```text
//! SizeTrack::new(scroll, self.viewport, Message::Resized)
//! ```

use iced::advanced::layout::{Limits, Node};
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget};
use iced::mouse::{Cursor, Interaction};
use iced::{Element, Event, Length, Rectangle, Renderer as BaseRenderer, Size, Theme, Vector};

/// Wraps `content` and publishes its laid-out size whenever it is not `known`. Built with [`SizeTrack::new`].
pub struct SizeTrack<'a, Message> {
    content: Element<'a, Message>,
    known: Size,
    on_resize: Box<dyn Fn(Size) -> Message + 'a>,
}

impl<'a, Message> SizeTrack<'a, Message> {
    /// Wraps `content`. `known` is the size the view has for it now.
    pub fn new(content: impl Into<Element<'a, Message>>, known: Size, on_resize: impl Fn(Size) -> Message + 'a) -> Self {
        Self {
            content: content.into(),
            known,
            on_resize: Box::new(on_resize),
        }
    }
}

/// The size to report, given the size laid out and the size the view knows: the laid-out size when the two differ by
/// more than a rounding error, and `None` while the view is already right.
fn changed(laid_out: Size, known: Size) -> Option<Size> {
    let differs = (laid_out.width - known.width).abs() > 0.5 || (laid_out.height - known.height).abs() > 0.5;
    differs.then_some(laid_out)
}

impl<Message> Widget<Message, Theme, BaseRenderer> for SizeTrack<'_, Message> {
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

        // Checked on every event, so the first one after a resize reports the new size.
        if let Some(size) = changed(layout.bounds().size(), self.known) {
            shell.publish((self.on_resize)(size));
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

impl<'a, Message: 'a> From<SizeTrack<'a, Message>> for Element<'a, Message, Theme, BaseRenderer> {
    fn from(track: SizeTrack<'a, Message>) -> Self {
        Element::new(track)
    }
}

#[cfg(test)]
#[path = "../../../tests/app/components/size_track.rs"]
mod tests;
