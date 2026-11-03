//! Sidebar navigation — the style guide's `.nav-item` rail (§07).
//!
//! A vertical rail of 44×44 rounded icon slots. The active item wears a green
//! 8% tint and the signature 3px left indicator bar with a green glow; a faint
//! white highlight glides under the hovered item. The indicator *slides*
//! between items on an ease — the rail should feel like a compass needle
//! swinging home (§01, §08).
//!
//! ```text
//! NavRail::new(vec![NavItem::new(Icon::lucide().home()), ...])
//!     .active(self.section)
//!     .on_select(|i| Message::SectionSelected(i))
//! ```

use crate::app::theme::color;
use iced::advanced::Renderer as _;
use iced::advanced::layout::{Limits, Node};
use iced::advanced::renderer;
use iced::advanced::svg::Renderer as _;
use iced::advanced::svg::{Handle, Svg};
use iced::advanced::widget::{Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, overlay};
use iced::border::Radius;
use iced::mouse::{Cursor, Interaction};
use iced::{Border, Color, Element, Event, Length, Point, Radians, Rectangle, Renderer as BaseRenderer, Size, Theme, Vector, mouse, window};
use iced_anim::Animated;
use iced_color_extension::rgb;

use super::anim;
use super::hint::bubble;

/// Item slot size — `.nav-item` is 44×44 with a 12px radius.
const ITEM_SIZE: f32 = 44.0;
const ITEM_RADIUS: f32 = 12.0;
/// Gap between items.
const GAP: f32 = 6.0;
/// Left inset that leaves room for the 3px indicator bar.
const INDICATOR_INSET: f32 = 14.0;
/// Indicator bar geometry — 3px wide, 22px tall, centered on the item.
const INDICATOR_WIDTH: f32 = 3.0;
const INDICATOR_HEIGHT: f32 = 22.0;
/// The gap between a slot and the tooltip that names it, beside the rail.
const TOOLTIP_GAP: f32 = 10.0;

/// One slot in a [`NavRail`].
#[derive(Debug, Clone)]
pub struct NavItem {
    /// SVG markup of the icon.
    pub icon: &'static str,
    /// The name a tooltip shows beside the slot while the pointer rests on it. `None` shows no tooltip.
    pub label: Option<&'static str>,
}

impl NavItem {
    pub fn new(icon: &'static str) -> Self {
        Self { icon, label: None }
    }

    /// A slot whose tooltip names it.
    pub fn labelled(icon: &'static str, label: &'static str) -> Self {
        Self { icon, label: Some(label) }
    }
}

/// An animated sidebar navigation rail (`.nav-item`).
pub struct NavRail<Message> {
    items: Vec<NavItem>,
    active: usize,
    on_select: Option<Box<dyn Fn(usize) -> Message>>,
    icon_size: f32,
    disabled: bool,
    /// Whether the active item draws the 3px left indicator bar. The
    /// sidebar suppresses it (its active state is the filled slot alone);
    /// the style-guide showcase keeps it.
    indicator: bool,
}

#[derive(Debug)]
struct State {
    /// Item bounds relative to the rail, recorded during layout.
    item_bounds: Vec<Rectangle>,
    /// Animated indicator top (relative to the rail).
    indicator_y: Animated<f32>,
    /// Hovered item, if any.
    hovered: Option<usize>,
    hover_y: Animated<f32>,
    hover_alpha: Animated<f32>,
    is_pressed: Option<usize>,
    /// True until the first layout, so the indicator settles instead of flying.
    first_layout: bool,
    /// The tooltip's tree, kept between passes. Each pass builds a new overlay, and only a layout fills in the tooltip's
    /// text, so the tree must outlive the overlay that draws it.
    bubble: Option<Tree>,
}

impl<Message> NavRail<Message> {
    /// Creates a rail from the given items.
    pub fn new<I: IntoIterator<Item = NavItem>>(items: I) -> Self {
        Self {
            items: items.into_iter().collect(),
            active: 0,
            on_select: None,
            icon_size: 20.0,
            disabled: false,
            indicator: true,
        }
    }

    /// Sets the active item index.
    pub fn active(mut self, active: usize) -> Self {
        self.active = active;
        self
    }

    /// Sets the message produced when an item is selected.
    ///
    /// Without an `on_select` closure the rail is not interactive.
    pub fn on_select(mut self, on_select: impl Fn(usize) -> Message + 'static) -> Self {
        self.on_select = Some(Box::new(on_select));
        self
    }

    /// Size of the item icons, in logical pixels.
    pub fn icon_size(mut self, icon_size: f32) -> Self {
        self.icon_size = icon_size;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Shows or hides the active item's left indicator bar.
    pub fn indicator(mut self, indicator: bool) -> Self {
        self.indicator = indicator;
        self
    }

    /// Whether the rail can be interacted with at all.
    fn interactive(&self) -> bool {
        !self.disabled && self.on_select.is_some()
    }

    /// The index of the item under the cursor, if any.
    fn item_at(state: &State, cursor: Cursor, bounds: Rectangle) -> Option<usize> {
        // `position_in` is already relative to `bounds` — subtracting the
        // origin again shifts the whole hit test by the rail's position.
        let position = cursor.position_in(bounds)?;
        let y = position.y;
        state.item_bounds.iter().position(|item| y >= item.y && y <= item.y + item.height)
    }

    /// Idle icon color — `--ink-3`; hovered brightens; active is Lode Green.
    fn icon_color(&self, index: usize, hovered: Option<usize>) -> Color {
        if self.disabled {
            return color::text_disabled();
        }
        if index == self.active {
            color::primary()
        } else if hovered == Some(index) {
            color::text_headline()
        } else {
            color::text_placeholder()
        }
    }

    /// Syncs the animated indicator/hover targets against recorded bounds.
    fn sync_targets(&self, state: &mut State) -> bool {
        if state.item_bounds.is_empty() {
            return false;
        }

        let mut changed = false;

        let active_top = state
            .item_bounds
            .get(self.active)
            .map(|item| item.center_y() - INDICATOR_HEIGHT / 2.0)
            .unwrap_or(0.0);
        if state.indicator_y.target() != &active_top {
            if state.first_layout {
                state.indicator_y.settle_at(active_top);
                state.first_layout = false;
            } else {
                state.indicator_y.set_target(active_top);
            }
            changed = true;
        }

        let hover_top = state
            .hovered
            .and_then(|i| state.item_bounds.get(i))
            .map(|item| item.y)
            .unwrap_or_else(|| *state.hover_y.target());
        let hover_alpha_target = if state.hovered.is_some() { 1.0 } else { 0.0 };
        if state.hover_y.target() != &hover_top {
            state.hover_y.set_target(hover_top);
            changed = true;
        }
        if state.hover_alpha.target() != &hover_alpha_target {
            state.hover_alpha.set_target(hover_alpha_target);
            changed = true;
        }

        changed
    }

    fn intrinsic_size(&self) -> Size {
        let height = self.items.len().checked_sub(1).map(|gaps| gaps as f32 * GAP).unwrap_or(0.0) + self.items.len() as f32 * ITEM_SIZE;
        Size::new(INDICATOR_INSET + ITEM_SIZE, height)
    }
}

impl<Message> Widget<Message, Theme, BaseRenderer> for NavRail<Message>
where
    Message: 'static,
{
    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Shrink,
            height: Length::Shrink,
        }
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State {
            item_bounds: Vec::new(),
            indicator_y: Animated::new(0.0, anim::glide()),
            hovered: None,
            hover_y: Animated::new(0.0, anim::micro()),
            hover_alpha: Animated::new(0.0, anim::micro()),
            is_pressed: None,
            first_layout: true,
            bubble: None,
        })
    }

    fn layout(&mut self, tree: &mut Tree, _renderer: &BaseRenderer, limits: &Limits) -> Node {
        let state = tree.state.downcast_mut::<State>();
        let intrinsic = self.intrinsic_size();

        let mut item_bounds = Vec::with_capacity(self.items.len());
        for index in 0..self.items.len() {
            let y = index as f32 * (ITEM_SIZE + GAP);
            item_bounds.push(Rectangle::new(Point::new(INDICATOR_INSET, y), Size::new(ITEM_SIZE, ITEM_SIZE)));
        }

        state.item_bounds = item_bounds;
        let _ = self.sync_targets(state);

        let size = limits.resolve(Length::Shrink, Length::Shrink, intrinsic);
        Node::new(size)
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: Cursor,
        _renderer: &BaseRenderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let state = tree.state.downcast_mut::<State>();

        match event {
            Event::Window(window::Event::RedrawRequested(now)) => {
                state.indicator_y.tick(*now);
                state.hover_y.tick(*now);
                state.hover_alpha.tick(*now);
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if self.interactive()
                    && let Some(index) = Self::item_at(state, cursor, bounds)
                {
                    state.is_pressed = Some(index);
                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if let Some(pressed) = state.is_pressed.take() {
                    if let (Some(index), Some(on_select)) = (Self::item_at(state, cursor, bounds), self.on_select.as_ref())
                        && index == pressed
                        && !self.disabled
                    {
                        shell.publish(on_select(index));
                    }
                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let hovered = if self.interactive() {
                    Self::item_at(state, cursor, bounds)
                } else {
                    None
                };
                if state.hovered != hovered {
                    state.hovered = hovered;
                }
            }
            _ => {}
        }

        let mut needs_redraw = self.sync_targets(state);
        needs_redraw |= state.indicator_y.is_animating();
        needs_redraw |= state.hover_y.is_animating();
        needs_redraw |= state.hover_alpha.is_animating();

        if needs_redraw {
            shell.request_redraw();
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut BaseRenderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State>();
        let bounds = layout.bounds();
        let radius = Radius::from(ITEM_RADIUS);

        // Active item tint — Lode Green at 8% (§07 `.nav-item.active`).
        if let Some(active) = state.item_bounds.get(self.active) {
            let active_bounds = Rectangle::new(Point::new(bounds.x + active.x, bounds.y + active.y), active.size());
            if active_bounds.intersection(viewport).is_some() {
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: active_bounds,
                        border: Border { radius, ..Border::default() },
                        ..renderer::Quad::default()
                    },
                    rgb!(34, 255, 132, 8%),
                );
            }
        }

        // Sliding hover highlight — white 4%.
        let hover_alpha = *state.hover_alpha.value();
        if hover_alpha > 0.01 && state.hovered.is_some() {
            let hover_bounds = Rectangle::new(
                Point::new(bounds.x + INDICATOR_INSET, bounds.y + *state.hover_y.value()),
                Size::new(ITEM_SIZE, ITEM_SIZE),
            );
            renderer.fill_quad(
                renderer::Quad {
                    bounds: hover_bounds,
                    border: Border { radius, ..Border::default() },
                    ..renderer::Quad::default()
                },
                anim::fade(rgb!(255, 255, 255, 4%), hover_alpha),
            );
        }

        // Icons.
        for (index, item) in self.items.iter().enumerate() {
            let Some(slot) = state.item_bounds.get(index) else {
                continue;
            };
            let slot_bounds = Rectangle::new(Point::new(bounds.x + slot.x, bounds.y + slot.y), slot.size());
            if slot_bounds.intersection(viewport).is_none() {
                continue;
            }
            let icon_svg = Svg {
                handle: Handle::from_memory(item.icon.as_bytes()),
                opacity: 1.0,
                rotation: Radians(0.0),
                color: Some(self.icon_color(index, state.hovered)),
            };
            let icon_bounds = Rectangle::new(
                Point::new(
                    slot_bounds.center_x() - self.icon_size / 2.0,
                    slot_bounds.center_y() - self.icon_size / 2.0,
                ),
                Size::new(self.icon_size, self.icon_size),
            );
            renderer.draw_svg(icon_svg, icon_bounds, slot_bounds);
        }

        // Active indicator — 3px green bar with a glow (§07 `.nav-item.active::before`).
        if !self.indicator {
            return;
        }
        let indicator_bounds = Rectangle::new(
            Point::new(bounds.x, bounds.y + *state.indicator_y.value()),
            Size::new(INDICATOR_WIDTH, INDICATOR_HEIGHT),
        );
        renderer.fill_quad(
            renderer::Quad {
                bounds: indicator_bounds,
                border: Border {
                    radius: Radius::from(INDICATOR_WIDTH / 2.0),
                    ..Border::default()
                },
                shadow: iced::Shadow {
                    color: color::primary_glow(),
                    offset: iced::Vector::new(0.0, 0.0),
                    blur_radius: crate::app::theme::blur(12.0),
                },
                ..renderer::Quad::default()
            },
            color::primary(),
        );
    }

    fn mouse_interaction(&self, tree: &Tree, layout: Layout<'_>, cursor: Cursor, _viewport: &Rectangle, _renderer: &BaseRenderer) -> Interaction {
        let state = tree.state.downcast_ref::<State>();
        if self.interactive() && Self::item_at(state, cursor, layout.bounds()).is_some() {
            Interaction::Pointer
        } else {
            Interaction::default()
        }
    }

    /// The tooltip of the hovered slot, if that slot has a label. It is an overlay, so it draws above the page.
    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'_>,
        _renderer: &BaseRenderer,
        _viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, BaseRenderer>> {
        let state = tree.state.downcast_mut::<State>();
        let index = state.hovered?;
        let label = self.items.get(index)?.label?;
        let slot = *state.item_bounds.get(index)?;
        let bounds = layout.bounds();
        let anchor = Rectangle::new(Point::new(bounds.x + slot.x, bounds.y + slot.y), slot.size()) + translation;
        let content = bubble(label);
        let bubble_tree = state.bubble.get_or_insert_with(|| Tree::new(&content));
        Some(overlay::Element::new(Box::new(BubbleOverlay {
            content,
            tree: bubble_tree,
            anchor,
        })))
    }
}

impl<'a, Message: 'static + 'a> From<NavRail<Message>> for Element<'a, Message, Theme, BaseRenderer> {
    fn from(rail: NavRail<Message>) -> Self {
        Element::new(rail)
    }
}

/// Where a tooltip of `size` sits beside the slot `anchor`: to its right, centred on it, and kept inside `window`.
fn tooltip_origin(anchor: Rectangle, size: Size, window: Size) -> Vector {
    let x = (anchor.x + anchor.width + TOOLTIP_GAP).min(window.width - size.width).max(0.0);
    let y = (anchor.center_y() - size.height / 2.0).min(window.height - size.height).max(0.0);
    Vector::new(x, y)
}

/// The bubble that names a hovered slot. It draws above the page, so it is an overlay rather than part of the rail.
/// Its tree is kept in the rail's state, so the text its layout fills in is still there when the bubble draws.
struct BubbleOverlay<'b, Message> {
    content: Element<'static, Message, Theme, BaseRenderer>,
    tree: &'b mut Tree,
    /// The slot's bounds in window coordinates.
    anchor: Rectangle,
}

impl<Message: 'static> overlay::Overlay<Message, Theme, BaseRenderer> for BubbleOverlay<'_, Message> {
    fn layout(&mut self, renderer: &BaseRenderer, bounds: Size) -> Node {
        let node = self.content.as_widget_mut().layout(self.tree, renderer, &Limits::new(Size::ZERO, bounds));
        let size = node.size();
        Node::with_children(size, vec![node]).translate(tooltip_origin(self.anchor, size, bounds))
    }

    fn draw(&self, renderer: &mut BaseRenderer, theme: &Theme, style: &renderer::Style, layout: Layout<'_>, cursor: Cursor) {
        if let Some(node) = layout.children().next() {
            self.content
                .as_widget()
                .draw(self.tree, renderer, theme, style, node, cursor, &Rectangle::with_size(Size::INFINITE));
        }
    }
}

#[cfg(test)]
#[path = "../../../tests/app/components/nav.rs"]
mod tests;
