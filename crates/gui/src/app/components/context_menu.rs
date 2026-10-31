//! Context menu — a floating action menu that opens at the pointer on a right-click.
//!
//! The menu wraps any content. A right-press over the content, one the content has not
//! already captured, opens the shared floating engine ([`super::menu`]) at the cursor, with
//! the same surface, hover rows and separators as [`super::dropdown`]. It closes on Esc, on a
//! press outside it, on a wheel scroll and on a window resize.
//!
//! The menu remembers the `key` it opened for. When the content in that tree slot now has a
//! different key (a scroll or a reload reshuffled a virtualised list), the menu drops rather
//! than showing another target's actions.
//!
//! ```text
//! ContextMenu::new(card, instance.id.clone())
//!     .items(instance_menu::menu(false, true).items)
//!     .on_select(move |index| Message::InstanceMenu { instance: id.clone(), index })
//! ```

use iced::advanced::layout::{Limits, Node};
use iced::advanced::overlay::{self, Group};
use iced::advanced::renderer;
use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget};
use iced::mouse::{self, Cursor, Interaction};
use iced::{Element, Event, Length, Point, Rectangle, Renderer as BaseRenderer, Size, Theme, Vector, window};

use super::menu::{self, Anchor, MenuItem, MenuState};

/// The menu's width. Its rows are short labels, so a fixed width is enough.
const MENU_WIDTH: f32 = 200.0;

/// A context menu around `content`, opened by a right-click. See the module docs.
pub struct ContextMenu<'a, Message> {
    content: Element<'a, Message>,
    key: String,
    items: Vec<MenuItem>,
    on_select: Option<Box<dyn Fn(usize) -> Message + 'a>>,
}

/// The menu's state, kept in the widget tree.
#[derive(Debug)]
struct State {
    menu: MenuState,
    /// Where the pointer was at the right-press, in the content's space.
    anchor: Point,
    /// The key the menu opened for.
    key: Option<String>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            menu: MenuState::new(),
            anchor: Point::ORIGIN,
            key: None,
        }
    }
}

/// Whether an open menu belongs to a different target than `key`. A menu that opened for one
/// key must not show another key's actions.
fn is_stale(opened_for: Option<&str>, key: &str) -> bool {
    opened_for != Some(key)
}

impl<'a, Message> ContextMenu<'a, Message> {
    /// Wraps `content`. `key` names what the menu is for; see the module docs.
    pub fn new(content: impl Into<Element<'a, Message>>, key: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            key: key.into(),
            items: Vec::new(),
            on_select: None,
        }
    }

    /// Sets the menu's rows, top to bottom.
    pub fn items<I: IntoIterator<Item = MenuItem>>(mut self, items: I) -> Self {
        self.items = items.into_iter().collect();
        self
    }

    /// Sets the message for a picked row. The index counts every row, separators included.
    pub fn on_select(mut self, on_select: impl Fn(usize) -> Message + 'a) -> Self {
        self.on_select = Some(Box::new(on_select));
        self
    }

    /// The menu opens only when it has rows to show and a message for a picked one.
    fn interactive(&self) -> bool {
        self.on_select.is_some() && !self.items.is_empty()
    }
}

impl<Message: 'static> Widget<Message, Theme, BaseRenderer> for ContextMenu<'_, Message> {
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
        // The content goes first, so a control that captures the press (Play) keeps it.
        self.content
            .as_widget_mut()
            .update(&mut tree.children[0], event, layout, cursor, renderer, clipboard, shell, viewport);

        let state = tree.state.downcast_mut::<State>();
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) => {
                // A right-press the content has not claimed opens the menu at the pointer. Any
                // other right-press closes it, including one on another card.
                match cursor.position() {
                    Some(position) if self.interactive() && !shell.is_event_captured() && cursor.is_over(layout.bounds()) => {
                        state.anchor = position;
                        state.key = Some(self.key.clone());
                        state.menu.open();
                        shell.capture_event();
                    }
                    _ => state.menu.close(),
                }
                shell.request_redraw();
            }
            Event::Window(window::Event::Resized(_)) => state.menu.close(),
            // A wheel scroll the open menu did not take closes it: the rows it showed have moved.
            Event::Mouse(mouse::Event::WheelScrolled { .. }) if !shell.is_event_captured() => state.menu.close(),
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
        let ContextMenu {
            content,
            key,
            items,
            on_select,
            ..
        } = self;
        let state = tree.state.downcast_mut::<State>();
        if !state.menu.is_closed() && is_stale(state.key.as_deref(), key.as_str()) {
            // The content in this slot is another target now, so the menu drops rather than
            // showing its actions.
            *state = State::default();
        }

        // The anchor is in the content's space; the scroll translation puts it in window space.
        let anchor = Anchor {
            origin: Point::new(state.anchor.x + translation.x, state.anchor.y + translation.y),
            trigger_height: 0.0,
            width: MENU_WIDTH,
            gap: 0.0,
        };
        let menu = menu::overlay::<'b, 'b, Message>(&mut state.menu, items.as_slice(), None, on_select.as_deref(), anchor, false);
        let content = content
            .as_widget_mut()
            .overlay(&mut tree.children[0], layout, renderer, viewport, translation);

        match (content, menu) {
            (Some(content), Some(menu)) => Some(Group::with_children(vec![content, menu]).overlay()),
            (content, menu) => content.or(menu),
        }
    }
}

impl<'a, Message: 'static + 'a> From<ContextMenu<'a, Message>> for Element<'a, Message, Theme, BaseRenderer> {
    fn from(menu: ContextMenu<'a, Message>) -> Self {
        Element::new(menu)
    }
}

#[cfg(test)]
#[path = "../../../tests/app/components/context_menu.rs"]
mod tests;
