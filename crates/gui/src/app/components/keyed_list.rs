//! A vertical list whose rows keep their widget state by key, not by position.
//!
//! iced's `keyed_column` reads its keys only when the number of children changes. A windowed list that scrolls by one
//! row builds as many children as before, so every slot keeps the state of the row that sat there: a switch shows its
//! neighbour's position, and an open menu moves to another row. This list matches each child to the state stored under
//! its key on every rebuild, so a row's state goes wherever the row goes, and a row that is new starts fresh.
//!
//! ```text
//! KeyedList::new(rows.iter().map(|row| (row.key(), row.view()))).width(Fill)
//! ```

use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::ops::Range;

use iced::advanced::layout::{self, Limits, Node};
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget};
use iced::mouse::{Cursor, Interaction};
use iced::{Alignment, Element, Event, Length, Padding, Rectangle, Renderer as BaseRenderer, Size, Theme, Vector};

/// The key of one row. A hash of whatever names the row is enough.
pub type Key = u64;

/// The key of the row that `name` names. A hash of the name is stable for as long as the row keeps it.
pub fn key_of(name: &str) -> Key {
    let mut hasher = DefaultHasher::new();
    name.hash(&mut hasher);
    hasher.finish()
}

/// The rows a windowed list builds when it is scrolled to `scroll_y` in a viewport `height` tall: the rows on screen and
/// `overscan` more past each edge, so a scroll never reveals a gap before the next frame. Every row is `pitch` tall, and
/// the list has `total` of them. A scroll past the end, left by a list that shrank, builds the last rows.
pub fn visible_rows(scroll_y: f32, height: f32, total: usize, pitch: f32, overscan: usize) -> Range<usize> {
    let top = (scroll_y.max(0.0) / pitch).floor() as usize;
    let shown = (height.max(0.0) / pitch).ceil() as usize;
    let first = top.saturating_sub(overscan).min(total.saturating_sub(1));
    let end = (top + shown + overscan).min(total).max(first);
    first..end
}

/// Rows stacked top to bottom, each with the key its state is kept under. Built with [`KeyedList::new`].
pub struct KeyedList<'a, Message> {
    keys: Vec<Key>,
    children: Vec<Element<'a, Message>>,
    width: Length,
}

impl<'a, Message> KeyedList<'a, Message> {
    /// A list of `children`, each paired with its key. Two children with one key are a mistake: the second starts fresh
    /// on every rebuild.
    pub fn new(children: impl IntoIterator<Item = (Key, Element<'a, Message>)>) -> Self {
        let (keys, children) = children.into_iter().unzip();
        Self {
            keys,
            children,
            width: Length::Shrink,
        }
    }

    /// Sets the list's width.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }
}

/// The keys the list was last built with, in order, so the next build can find each row's state.
struct State {
    keys: Vec<Key>,
}

/// Arranges the `old` items, stored in the order of `old_keys`, into the order of `new_keys`. An item moves to wherever
/// its key now sits, a key that was not there before gets `fresh(index)`, and an item whose key is gone is dropped.
fn rematch<T>(old_keys: &[Key], old: Vec<T>, new_keys: &[Key], mut fresh: impl FnMut(usize) -> T) -> Vec<T> {
    let mut kept: HashMap<Key, T> = HashMap::with_capacity(old.len());
    for (key, item) in old_keys.iter().copied().zip(old) {
        kept.entry(key).or_insert(item);
    }
    new_keys
        .iter()
        .enumerate()
        .map(|(index, key)| kept.remove(key).unwrap_or_else(|| fresh(index)))
        .collect()
}

impl<Message> Widget<Message, Theme, BaseRenderer> for KeyedList<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State { keys: self.keys.clone() })
    }

    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        let state = tree.state.downcast_mut::<State>();
        // A rebuild with the same rows in the same order is the common case, and needs no matching.
        if state.keys != self.keys || tree.children.len() != self.children.len() {
            let old = std::mem::take(&mut tree.children);
            tree.children = rematch(&state.keys, old, &self.keys, |index| Tree::new(&self.children[index]));
            state.keys.clone_from(&self.keys);
        }
        for (child, tree) in self.children.iter().zip(&mut tree.children) {
            tree.diff(child);
        }
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: Length::Shrink,
        }
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &BaseRenderer, limits: &Limits) -> Node {
        let limits = limits.width(self.width);
        layout::flex::resolve(
            layout::flex::Axis::Vertical,
            renderer,
            &limits,
            self.width,
            Length::Shrink,
            Padding::ZERO,
            0.0,
            Alignment::Start,
            &mut self.children,
            &mut tree.children,
        )
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
        for ((child, tree), layout) in self.children.iter_mut().zip(&mut tree.children).zip(layout.children()) {
            child
                .as_widget_mut()
                .update(tree, event, layout, cursor, renderer, clipboard, shell, viewport);
        }
    }

    fn mouse_interaction(&self, tree: &Tree, layout: Layout<'_>, cursor: Cursor, viewport: &Rectangle, renderer: &BaseRenderer) -> Interaction {
        self.children
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
            .map(|((child, tree), layout)| child.as_widget().mouse_interaction(tree, layout, cursor, viewport, renderer))
            .max()
            .unwrap_or_default()
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
        for ((child, state), layout) in self.children.iter().zip(&tree.children).zip(layout.children()) {
            child.as_widget().draw(state, renderer, theme, style, layout, cursor, viewport);
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

impl<'a, Message: 'a> From<KeyedList<'a, Message>> for Element<'a, Message> {
    fn from(list: KeyedList<'a, Message>) -> Self {
        Element::new(list)
    }
}

#[cfg(test)]
#[path = "../../../tests/app/components/keyed_list.rs"]
mod tests;
