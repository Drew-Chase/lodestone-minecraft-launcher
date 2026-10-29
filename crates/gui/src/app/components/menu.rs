//! Shared floating-menu engine behind [`Select`](super::select) and
//! [`Dropdown`](super::dropdown).
//!
//! A hand-rolled `Overlay` in the spirit of iced's own `combo_box` menu, but
//! styled straight from the guide: `bg-3` surface with a hairline border and
//! the elevation-3 shadow (§06), `bg-4` hover rows, Lode Green selected rows,
//! and an open/close ease — fade plus a 6px slide (§08 Lift).
//!
//! The engine is deliberately internal; the public surface lives in
//! [`select`](super::select) and [`dropdown`](super::dropdown).

use crate::app::theme::{color, fonts};
use iced::advanced::layout::Node;
use iced::advanced::overlay::Element as OverlayElement;
use iced::advanced::renderer;
use iced::advanced::svg::Renderer as _;
use iced::advanced::svg::{Handle, Svg};
use iced::advanced::text;
use iced::advanced::text::Renderer as _;
use iced::advanced::{Clipboard, Layout, Overlay, Renderer as _, Shell};
use iced::border::Radius;
use iced::mouse::{Cursor, Interaction};
use iced::{
    Background, Border, Event, Pixels, Point, Radians, Rectangle, Renderer as BaseRenderer, Shadow, Size, Theme, Vector, alignment, keyboard, mouse,
    window,
};
use iced_anim::Animated;
use iced_color_extension::rgb;

use super::anim;

/// Vertical gap between the trigger and the menu.
const GAP: f32 = 6.0;
/// Margin kept from the window edge.
const SCREEN_MARGIN: f32 = 8.0;
/// Height of one action row — 13px text at 1.4 plus 8px vertical padding.
const ROW_HEIGHT: f32 = 34.0;
/// Height of a separator, including its breathing room.
const SEPARATOR_HEIGHT: f32 = 9.0;
/// Horizontal padding inside a row.
const H_PADDING: f32 = 12.0;
/// Inset applied to row highlight quads.
const ROW_INSET: f32 = 4.0;
/// Hard cap on menu height; taller menus scroll.
pub const MAX_MENU_HEIGHT: f32 = 320.0;

/// One entry of a floating menu.
#[derive(Debug, Clone)]
pub enum MenuItem {
    /// A selectable action row.
    Action {
        label: String,
        icon: Option<&'static str>,
        /// Destructive action — `--pink` ink (§02: errors, destructive).
        danger: bool,
    },
    /// A hairline divider between groups.
    Separator,
}

impl MenuItem {
    /// Height of this entry.
    fn height(&self) -> f32 {
        match self {
            MenuItem::Action { .. } => ROW_HEIGHT,
            MenuItem::Separator => SEPARATOR_HEIGHT,
        }
    }

    fn is_selectable(&self) -> bool {
        matches!(self, MenuItem::Action { .. })
    }
}

/// The open/close lifecycle of the menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Phase {
    /// Not mounted — the widget produces no overlay.
    Closed,
    /// Fully open.
    Open,
    /// Playing the close animation; the overlay stays mounted until it ends.
    Closing,
}

/// Persistent state of the floating menu, owned by the host widget's tree
/// state.
#[derive(Debug)]
pub struct MenuState {
    pub(crate) scroll: f32,
    pub(crate) max_scroll: f32,
    pub(crate) hovered: Option<usize>,
    pub(crate) phase: Phase,
    pub(crate) open: Animated<f32>,
}

impl MenuState {
    pub(crate) fn new() -> Self {
        Self {
            scroll: 0.0,
            max_scroll: 0.0,
            hovered: None,
            phase: Phase::Closed,
            open: Animated::new(0.0, anim::menu()),
        }
    }

    pub(crate) fn is_closed(&self) -> bool {
        self.phase == Phase::Closed
    }

    pub(crate) fn is_open(&self) -> bool {
        self.phase == Phase::Open
    }

    pub(crate) fn open(&mut self) {
        self.phase = Phase::Open;
        self.open.set_target(1.0);
        self.scroll = 0.0;
        self.hovered = None;
    }

    pub(crate) fn close(&mut self) {
        if self.phase != Phase::Closed {
            self.phase = Phase::Closing;
            self.open.set_target(0.0);
        }
    }
}

impl Default for MenuState {
    fn default() -> Self {
        Self::new()
    }
}

/// The floating menu overlay. Created fresh each frame by the host widget's
/// `Widget::overlay` while the menu is not [`Phase::Closed`].
pub(crate) struct MenuOverlay<'a, Message> {
    state: &'a mut MenuState,
    items: &'a [MenuItem],
    /// Highlighted row (the host's current value), if any.
    selected: Option<usize>,
    on_select: &'a dyn Fn(usize) -> Message,
    /// Top-left of the trigger, already translated into window space.
    origin: Point,
    trigger_height: f32,
    menu_width: f32,
}

impl<Message> MenuOverlay<'_, Message> {
    /// Total unclipped height of all rows.
    fn total_height(&self) -> f32 {
        self.items.iter().map(MenuItem::height).sum()
    }

    /// The row under the given window-space point, if any.
    fn item_at(&self, menu_bounds: Rectangle, position: Point) -> Option<usize> {
        if !menu_bounds.contains(position) {
            return None;
        }
        let mut y = menu_bounds.y - self.state.scroll;
        for (index, item) in self.items.iter().enumerate() {
            let height = item.height();
            if position.y >= y && position.y < y + height {
                return item.is_selectable().then_some(index);
            }
            y += height;
        }
        None
    }

    /// Window-space bounds of one row, adjusted for scroll.
    fn row_bounds(&self, menu_bounds: Rectangle, index: usize) -> Option<Rectangle> {
        let mut y = menu_bounds.y - self.state.scroll;
        for (i, item) in self.items.iter().enumerate() {
            let height = item.height();
            if i == index {
                return Some(Rectangle::new(Point::new(menu_bounds.x, y), Size::new(menu_bounds.width, height)));
            }
            y += height;
        }
        None
    }
}

impl<Message> Overlay<Message, Theme, BaseRenderer> for MenuOverlay<'_, Message> {
    fn layout(&mut self, _renderer: &BaseRenderer, bounds: Size) -> Node {
        let total = self.total_height();

        let space_below = bounds.height - (self.origin.y + self.trigger_height) - GAP - SCREEN_MARGIN;
        let space_above = self.origin.y - GAP - SCREEN_MARGIN;
        let (place_below, available) = if space_below >= space_above {
            (true, space_below)
        } else {
            (false, space_above)
        };

        let height = total.min(available).clamp(0.0, MAX_MENU_HEIGHT);
        let width = self.menu_width.min(bounds.width - self.origin.x).max(0.0);
        let size = Size::new(width, height);

        let position = if place_below {
            Point::new(self.origin.x, self.origin.y + self.trigger_height + GAP)
        } else {
            Point::new(self.origin.x, self.origin.y - height - GAP)
        };

        // Keep scroll in range for the (possibly changed) viewport height.
        self.state.max_scroll = (total - height).max(0.0);
        self.state.scroll = self.state.scroll.clamp(0.0, self.state.max_scroll);

        Node::new(size).move_to(position)
    }

    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: Cursor,
        _renderer: &BaseRenderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) {
        let menu_bounds = layout.bounds();

        match event {
            Event::Window(window::Event::RedrawRequested(now)) => {
                self.state.open.tick(*now);
                // The overlay unmounts once the close animation has finished.
                if self.state.phase == Phase::Closing && !self.state.open.is_animating() {
                    self.state.phase = Phase::Closed;
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let hovered = cursor.position().and_then(|position| self.item_at(menu_bounds, position));
                if self.state.hovered != hovered {
                    self.state.hovered = hovered;
                    shell.request_redraw();
                }
            }
            Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                if cursor.is_over(menu_bounds) && self.state.max_scroll > 0.0 {
                    let amount = match delta {
                        mouse::ScrollDelta::Lines { y, .. } => *y * 14.0,
                        mouse::ScrollDelta::Pixels { y, .. } => *y,
                    };
                    self.state.scroll = (self.state.scroll - amount).clamp(0.0, self.state.max_scroll);
                    shell.request_redraw();
                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if cursor.is_over(menu_bounds) {
                    // Click inside the menu: select the hovered row, or simply
                    // dismiss when the click landed on a separator.
                    if let Some(index) = self.state.hovered
                        && self.items.get(index).is_some_and(MenuItem::is_selectable)
                    {
                        shell.publish((self.on_select)(index));
                    }
                    self.state.close();
                    shell.capture_event();
                } else {
                    // Click outside the menu: dismiss, but let the click
                    // fall through to whatever was actually under the cursor.
                    self.state.close();
                }
            }
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(keyboard::key::Named::Escape),
                ..
            }) => {
                self.state.close();
                shell.capture_event();
            }
            _ => {}
        }

        if self.state.open.is_animating() {
            shell.request_redraw();
        }
    }

    fn mouse_interaction(&self, layout: Layout<'_>, cursor: Cursor, _renderer: &BaseRenderer) -> Interaction {
        let menu_bounds = layout.bounds();
        let over_row = cursor.position().and_then(|position| self.item_at(menu_bounds, position)).is_some();
        if over_row { Interaction::Pointer } else { Interaction::default() }
    }

    fn draw(&self, renderer: &mut BaseRenderer, _theme: &Theme, _style: &renderer::Style, layout: Layout<'_>, cursor: Cursor) {
        let menu_bounds = layout.bounds();
        // The transition keeps the value in [0, 1]; the clamp is belt and
        // braces so a stray value can never produce out-of-range alphas.
        let alpha = self.state.open.value().clamp(0.0, 1.0);
        if alpha <= 0.001 {
            return;
        }

        // Open/close — fade plus a 6px slide up into place (§08 Lift).
        let offset = (1.0 - alpha) * 6.0;
        let menu_bounds = Rectangle::new(Point::new(menu_bounds.x, menu_bounds.y + offset), menu_bounds.size());

        // Surface — elevation 3 (§06): bg-3, hairline border, deep shadow.
        renderer.fill_quad(
            renderer::Quad {
                bounds: menu_bounds,
                border: Border {
                    radius: Radius::from(12.0),
                    width: 1.0,
                    color: anim::fade(color::Surface::LINE_STRONG, alpha),
                },
                shadow: Shadow {
                    color: rgb!(0, 0, 0, 80%),
                    offset: Vector::new(0.0, 30.0),
                    blur_radius: 80.0,
                },
                ..renderer::Quad::default()
            },
            Background::Color(anim::fade(color::Surface::BG3, alpha)),
        );

        // Rows, clipped to the menu surface.
        renderer.with_layer(menu_bounds, |renderer| {
            for (index, item) in self.items.iter().enumerate() {
                let Some(row) = self.row_bounds(menu_bounds, index) else {
                    continue;
                };
                if row.intersection(&menu_bounds).is_none() {
                    continue;
                }

                match item {
                    MenuItem::Separator => {
                        let line = Rectangle::new(
                            Point::new(row.x + H_PADDING, row.y + row.height / 2.0 - 0.5),
                            Size::new(row.width - H_PADDING * 2.0, 1.0),
                        );
                        renderer.fill_quad(
                            renderer::Quad {
                                bounds: line,
                                ..renderer::Quad::default()
                            },
                            Background::Color(anim::fade(color::Surface::LINE, alpha)),
                        );
                    }
                    MenuItem::Action { label, icon, danger } => {
                        let is_hovered =
                            self.state.hovered == Some(index) && cursor.position().is_some_and(|position| menu_bounds.contains(position));
                        let is_selected = self.selected == Some(index);

                        // Row highlight — hover bg-4, selected green tint.
                        let highlight = if is_selected {
                            Some(color::Primary::TINT)
                        } else if is_hovered {
                            Some(color::Surface::BG4)
                        } else {
                            None
                        };
                        if let Some(background) = highlight {
                            renderer.fill_quad(
                                renderer::Quad {
                                    bounds: Rectangle::new(Point::new(row.x + ROW_INSET, row.y), Size::new(row.width - ROW_INSET * 2.0, row.height)),
                                    border: Border {
                                        radius: Radius::from(8.0),
                                        ..Border::default()
                                    },
                                    ..renderer::Quad::default()
                                },
                                Background::Color(anim::fade(background, alpha)),
                            );
                        }

                        // Ink — danger rows are pink; selected rows read green.
                        let ink = if *danger {
                            color::Accents::PINK
                        } else if is_selected {
                            color::Primary::DEFAULT
                        } else {
                            color::Text::DEFAULT
                        };

                        // Leading icon.
                        let text_x = row.x + H_PADDING;
                        if let Some(icon) = icon {
                            let icon_size = 16.0;
                            let icon_svg = Svg {
                                handle: Handle::from_memory(icon.as_bytes()),
                                opacity: 1.0,
                                rotation: Radians(0.0),
                                color: Some(anim::fade(ink, alpha)),
                            };
                            let icon_bounds = Rectangle::new(
                                Point::new(text_x, row.y + row.height / 2.0 - icon_size / 2.0),
                                Size::new(icon_size, icon_size),
                            );
                            renderer.draw_svg(icon_svg, icon_bounds, row);
                        }

                        // Label — drawn via `fill_text` with owned content.
                        // A paragraph built locally during draw only downgrades
                        // to a weak reference in the render pass and is silently
                        // skipped once dropped; the cached path keys the buffer
                        // on the content instead (the same reason iced's own
                        // combo-box menu uses `fill_text`).
                        renderer.fill_text(
                            text::Text {
                                content: label.clone(),
                                bounds: Size::new(f32::INFINITY, row.height),
                                size: Pixels::from(13.0),
                                line_height: text::LineHeight::Relative(1.4),
                                font: fonts::Inter::MEDIUM,
                                align_x: text::Alignment::Default,
                                align_y: alignment::Vertical::Center,
                                shaping: text::Shaping::Basic,
                                wrapping: text::Wrapping::default(),
                            },
                            Point::new(text_x + if icon.is_some() { 18.0 + 8.0 } else { 0.0 }, row.center_y()),
                            anim::fade(ink, alpha),
                            menu_bounds,
                        );
                    }
                }
            }
        });
    }
}

/// Builds the overlay element for a host widget, if the menu is mounted.
pub(crate) fn overlay<'a, 'b, Message>(
    state: &'a mut MenuState,
    items: &'a [MenuItem],
    selected: Option<usize>,
    on_select: Option<&'a dyn Fn(usize) -> Message>,
    origin: Point,
    trigger_height: f32,
    menu_width: f32,
) -> Option<OverlayElement<'b, Message, Theme, BaseRenderer>>
where
    'a: 'b,
{
    if state.is_closed() || items.is_empty() {
        return None;
    }
    let on_select = on_select?;
    Some(OverlayElement::new(Box::new(MenuOverlay {
        state,
        items,
        selected,
        on_select,
        origin,
        trigger_height,
        menu_width,
    })))
}

#[cfg(test)]
#[path = "../../../tests/app/components/menu.rs"]
mod tests;
