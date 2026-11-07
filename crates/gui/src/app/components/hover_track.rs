//! Hover tracking — reports the pointer entering and leaving its content, judged against the view's `hovered` flag.
//!
//! `mouse_area` reports an enter or an exit only as a change from the state it remembered at its last update. A list
//! that scrolls under a still pointer changes what sits beneath it without a move, so that remembered state goes
//! stale: a card the pointer is still over reports nothing. This widget keeps no state of its
//! own. It compares the pointer with `hovered`, the flag the view passes in, and reports a change whenever the two
//! disagree. The next event after a scroll therefore brings the page up to date, and a page never has to clear its
//! hover on other messages.
//!
//! ```text
//! HoverTrack::new(card, hovered, Message::Hover { id, hovered: true }, Message::Hover { id, hovered: false })
//! ```

use iced::advanced::layout::{Limits, Node};
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget};
use iced::mouse::{Cursor, Interaction};
use iced::{Element, Event, Length, Rectangle, Renderer as BaseRenderer, Size, Theme, Vector};

/// The change to report, given where the pointer is and what the view shows: `Some(true)` when the pointer is over
/// the content but the view does not show it hovered, `Some(false)` when it has left content the view shows hovered,
/// and `None` while the view already agrees with the pointer.
fn transition(over: bool, hovered: bool) -> Option<bool> {
    (over != hovered).then_some(over)
}

/// Wraps `content` and publishes `enter` or `exit` when the pointer's side of `hovered` changes. Built with
/// [`HoverTrack::new`].
pub struct HoverTrack<'a, Message> {
    content: Element<'a, Message>,
    hovered: bool,
    enter: Message,
    exit: Message,
}

impl<'a, Message: Clone> HoverTrack<'a, Message> {
    /// Wraps `content`. `hovered` is whether the view shows the pointer over it now.
    pub fn new(content: impl Into<Element<'a, Message>>, hovered: bool, enter: Message, exit: Message) -> Self {
        Self {
            content: content.into(),
            hovered,
            enter,
            exit,
        }
    }
}

impl<Message: Clone + 'static> Widget<Message, Theme, BaseRenderer> for HoverTrack<'_, Message> {
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

        // Checked on every event, whether or not the content took it: where the pointer is does not depend on who
        // takes a press.
        if let Some(entered) = transition(cursor.is_over(layout.bounds()), self.hovered) {
            shell.publish(if entered { self.enter.clone() } else { self.exit.clone() });
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

impl<'a, Message: Clone + 'static + 'a> From<HoverTrack<'a, Message>> for Element<'a, Message, Theme, BaseRenderer> {
    fn from(track: HoverTrack<'a, Message>) -> Self {
        Element::new(track)
    }
}

#[cfg(test)]
#[path = "../../../tests/app/components/hover_track.rs"]
mod tests;
