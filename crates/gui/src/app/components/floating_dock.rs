//! The floating navigation dock: the page, with the navigation's two segments laid over its bottom edge.
//!
//! The segments float, so the page keeps its full height and shows through the gaps. The pair is centred in the window.
//! When the side segment would run past the right edge, the pair shifts left, and when the pair cannot keep [`EDGE`] either
//! side, the sidebar takes its place beside the page until the window is wide enough again.
//!
//! The dock lays out all four children every time, so a segment's measured width decides the placement. Events reach the
//! segments first, as iced's `Stack` delivers them, so a press on a bar button does not also press the page beneath it.

use iced::advanced::Renderer as _;
use iced::advanced::layout::{Limits, Node};
use iced::advanced::renderer;
use iced::advanced::widget::{Operation, Tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, overlay};
use iced::mouse::{Cursor, Interaction};
use iced::{Element, Event, Length, Point, Rectangle, Renderer as BaseRenderer, Size, Theme, Vector};

/// The least space kept between the bar and each side of the window, in pixels.
pub const EDGE: f32 = 15.0;
/// The gap between the two segments, in pixels.
pub const GAP: f32 = 15.0;
/// The distance from the bar to the window's bottom edge, in pixels.
pub const BOTTOM: f32 = 18.0;

/// The child indices: the page, the sidebar, the main segment and the side segment.
const PAGE: usize = 0;
const RAIL: usize = 1;
const MAIN: usize = 2;
const SIDE: usize = 3;

/// Where the main segment's left edge goes in a window `window` wide, for segments `main` and `side` wide. It is centred when
/// the pair fits, moved left just enough to keep the side segment inside the right edge, and `None` when the pair cannot keep
/// [`EDGE`] either side.
pub fn main_left(window: f32, main: f32, side: f32) -> Option<f32> {
    let total = main + GAP + side;
    if total + 2.0 * EDGE > window {
        return None;
    }
    let centred = (window - main) / 2.0;
    let right_limit = window - EDGE - total;
    Some(centred.min(right_limit).max(EDGE))
}

/// Whether the sidebar is showing, which the dock reads from its layout: the sidebar takes its width only when the bar
/// does not fit.
fn rail_shown(layout: Layout<'_>) -> bool {
    layout.children().nth(RAIL).is_some_and(|rail| rail.bounds().width > 0.0)
}

/// Whether the child at `index` is hidden: the sidebar while the bar shows, or the bar while the sidebar does.
fn hidden(index: usize, rail: bool) -> bool {
    match index {
        RAIL => !rail,
        MAIN | SIDE => rail,
        _ => false,
    }
}

/// The dock widget: `[page, sidebar, main segment, side segment]`.
pub struct FloatingDock<'a, Message> {
    children: Vec<Element<'a, Message>>,
}

impl<'a, Message> FloatingDock<'a, Message> {
    /// A dock over `page`. `rail` is the sidebar, shown in place of the bar when the window is too narrow for it, and
    /// `main` and `side` are the two segments.
    pub fn new(page: Element<'a, Message>, rail: Element<'a, Message>, main: Element<'a, Message>, side: Element<'a, Message>) -> Self {
        Self {
            children: vec![page, rail, main, side],
        }
    }
}

impl<Message: Clone + 'static> Widget<Message, Theme, BaseRenderer> for FloatingDock<'_, Message> {
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
        let whole = Limits::new(Size::ZERO, size);
        let main = self.children[MAIN].as_widget_mut().layout(&mut tree.children[MAIN], renderer, &whole);
        let side = self.children[SIDE].as_widget_mut().layout(&mut tree.children[SIDE], renderer, &whole);
        let (main_width, side_width) = (main.size().width, side.size().width);
        let (main_height, side_height) = (main.size().height, side.size().height);

        match main_left(size.width, main_width, side_width) {
            Some(left) => {
                let page = self.children[PAGE].as_widget_mut().layout(&mut tree.children[PAGE], renderer, &whole);
                // The segments sit on the same bottom edge.
                let bottom = size.height - BOTTOM;
                let main = main.move_to(Point::new(left, bottom - main_height));
                let side = side.move_to(Point::new(left + main_width + GAP, bottom - side_height));
                Node::with_children(size, vec![page, Node::new(Size::ZERO), main, side])
            }
            None => {
                let rail = self.children[RAIL].as_widget_mut().layout(&mut tree.children[RAIL], renderer, &whole);
                let rail_width = rail.size().width;
                let page_limits = Limits::new(Size::ZERO, Size::new((size.width - rail_width).max(0.0), size.height));
                let page = self.children[PAGE]
                    .as_widget_mut()
                    .layout(&mut tree.children[PAGE], renderer, &page_limits)
                    .move_to(Point::new(rail_width, 0.0));
                Node::with_children(size, vec![page, rail, Node::new(Size::ZERO), Node::new(Size::ZERO)])
            }
        }
    }

    fn operate(&mut self, tree: &mut Tree, layout: Layout<'_>, renderer: &BaseRenderer, operation: &mut dyn Operation) {
        operation.container(None, layout.bounds());
        // A hidden child has an empty layout, which some widgets cannot be operated on with (iced's container unwraps its
        // child's layout), so only the shown children take part.
        let rail = rail_shown(layout);
        operation.traverse(&mut |operation| {
            self.children
                .iter_mut()
                .zip(&mut tree.children)
                .zip(layout.children())
                .enumerate()
                .filter(|(index, _)| !hidden(*index, rail))
                .for_each(|(_, ((child, state), layout))| child.as_widget_mut().operate(state, layout, renderer, operation));
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
        let rail = rail_shown(layout);
        let nodes: Vec<Layout<'_>> = layout.children().collect();
        // Top first: once a child captures the event, the ones beneath it do not see it.
        for index in (0..self.children.len()).rev() {
            if hidden(index, rail) {
                continue;
            }
            self.children[index].as_widget_mut().update(
                &mut tree.children[index],
                event,
                nodes[index],
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );
            if shell.is_event_captured() {
                break;
            }
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
        let rail = rail_shown(layout);
        for (index, node) in layout.children().enumerate() {
            if hidden(index, rail) {
                continue;
            }
            let child = &self.children[index];
            let state = &tree.children[index];
            // The segments draw in their own layer, as iced's `Stack` draws the layers above its base. Otherwise the
            // page's images and text, which iced draws after the quads, would paint over the bar.
            if index == MAIN || index == SIDE {
                renderer.with_layer(node.bounds(), |renderer| {
                    child.as_widget().draw(state, renderer, theme, style, node, cursor, viewport);
                });
            } else {
                child.as_widget().draw(state, renderer, theme, style, node, cursor, viewport);
            }
        }
    }

    fn mouse_interaction(&self, tree: &Tree, layout: Layout<'_>, cursor: Cursor, viewport: &Rectangle, renderer: &BaseRenderer) -> Interaction {
        let rail = rail_shown(layout);
        self.children
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
            .enumerate()
            .filter(|(index, _)| !hidden(*index, rail))
            .map(|(_, ((child, state), node))| child.as_widget().mouse_interaction(state, node, cursor, viewport, renderer))
            .max()
            .unwrap_or_default()
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &BaseRenderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, BaseRenderer>> {
        // A hidden child's open panel must not draw where its zero-size layout sits, so only the shown children are asked.
        let rail = rail_shown(layout);
        let nodes: Vec<Layout<'b>> = layout.children().collect();
        let overlays = self
            .children
            .iter_mut()
            .zip(&mut tree.children)
            .zip(nodes)
            .enumerate()
            .filter(|(index, _)| !hidden(*index, rail))
            .filter_map(|(_, ((child, state), node))| child.as_widget_mut().overlay(state, node, renderer, viewport, translation))
            .collect::<Vec<_>>();
        (!overlays.is_empty()).then(|| overlay::Group::with_children(overlays).overlay())
    }
}

impl<'a, Message: Clone + 'static> From<FloatingDock<'a, Message>> for Element<'a, Message, Theme, BaseRenderer> {
    fn from(dock: FloatingDock<'a, Message>) -> Self {
        Element::new(dock)
    }
}

#[cfg(test)]
#[path = "../../../tests/app/components/floating_dock.rs"]
mod tests;
