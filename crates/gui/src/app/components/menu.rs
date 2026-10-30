//! Shared floating-menu engine behind [`Select`](super::select) and
//! [`Dropdown`](super::dropdown).
//!
//! A hand-rolled `Overlay` in the spirit of iced's own `combo_box` menu, but
//! styled straight from the guide: `bg-3` surface with a hairline border and
//! the elevation-3 shadow (§06), `bg-4` hover rows, Lode Green selected rows,
//! and an open/close ease — fade plus a 6px slide (§08 Lift).
//!
//! A searchable menu adds a filter field on the side nearest its trigger: the
//! top edge when the menu opens below, the bottom edge when it opens above.
//! Typing narrows the rows, the arrow keys move the highlight that Enter picks,
//! and Left, Right, Home, End, Backspace and Delete edit the query at a blinking
//! caret. The query clears each time the menu opens.
//!
//! The engine is deliberately internal; the public surface lives in
//! [`select`](super::select) and [`dropdown`](super::dropdown).

use crate::app::lang;
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
use iced::keyboard::key::{Named, Physical};
use iced::mouse::{Cursor, Interaction};
use iced::{
    Background, Border, Event, Pixels, Point, Radians, Rectangle, Renderer as BaseRenderer, Shadow, Size, Theme, Vector, alignment, keyboard, mouse,
    window,
};
use iced_anim::Animated;
use iced_color_extension::rgb;
use std::time::Instant;

use super::anim;
use super::search_field::{self, Query};

/// Vertical gap between the trigger and the menu.
pub(crate) const GAP: f32 = 6.0;
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
/// Height of the search band, and the height of the field inside it. The field
/// sits centred in the band, so the band's edge is the same on every side.
const SEARCH_BAND: f32 = 46.0;
const SEARCH_FIELD: f32 = 32.0;
/// Hard cap on menu height; taller menus scroll.
pub const MAX_MENU_HEIGHT: f32 = 320.0;

/// One entry of a floating menu.
#[derive(Debug, Clone)]
pub enum MenuItem {
    /// A selectable action row.
    Action {
        label: String,
        icon: Option<&'static str>,
        /// Destructive action — the red danger ink (the same red as the danger buttons).
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
    /// What the search field holds, and where its caret sits. Cleared each time the menu opens.
    pub(crate) query: Query,
    /// When the caret last moved or the text changed. Its blink counts from here.
    pub(crate) caret_since: Instant,
    /// The indices of the items the menu lists for `query`, in order.
    pub(crate) visible: Vec<usize>,
    /// Whether the menu opened below its trigger, which puts the search field on top.
    pub(crate) below: bool,
    /// The height of the rows window: the menu, less the search band.
    pub(crate) window_height: f32,
}

impl MenuState {
    pub(crate) fn new() -> Self {
        Self {
            scroll: 0.0,
            max_scroll: 0.0,
            hovered: None,
            phase: Phase::Closed,
            open: Animated::new(0.0, anim::menu()),
            query: Query::default(),
            caret_since: Instant::now(),
            visible: Vec::new(),
            below: true,
            window_height: 0.0,
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
        self.query.clear();
        self.caret_since = Instant::now();
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

/// The indices of the items a query leaves listed, in order. An empty query lists every
/// item. Any other query lists only the actions whose labels contain it, ignoring case.
pub(crate) fn listed(items: &[MenuItem], query: &str) -> Vec<usize> {
    let needle = query.to_lowercase();
    items
        .iter()
        .enumerate()
        .filter(|(_, item)| match item {
            MenuItem::Action { label, .. } => needle.is_empty() || label.to_lowercase().contains(&needle),
            MenuItem::Separator => needle.is_empty(),
        })
        .map(|(index, _)| index)
        .collect()
}

/// Where the menu attaches, in window space: a trigger's top-left corner and height, or a bare
/// point with no height (the pointer, for a context menu).
#[derive(Debug, Clone, Copy)]
pub(crate) struct Anchor {
    /// Top-left of the trigger, or the point the menu opens at.
    pub(crate) origin: Point,
    pub(crate) trigger_height: f32,
    pub(crate) width: f32,
    /// Space between the anchor and the menu: [`GAP`] for a trigger, none at the pointer.
    pub(crate) gap: f32,
}

/// Where a floating menu sits: its top-left corner, its size, and whether it opened below its
/// anchor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Placement {
    pub(crate) position: Point,
    pub(crate) size: Size,
    pub(crate) below: bool,
}

/// Places a menu `content` pixels tall against `anchor` in a `window`. The menu opens below the
/// anchor when that side has the room, otherwise above it. It is held to the window's width and
/// shifts left when it would run off the right edge, so it keeps its width.
pub(crate) fn place(anchor: Anchor, window: Size, content: f32) -> Placement {
    let origin = anchor.origin;
    let bottom = origin.y + anchor.trigger_height;
    let space_below = window.height - bottom - anchor.gap - SCREEN_MARGIN;
    let space_above = origin.y - anchor.gap - SCREEN_MARGIN;
    let (below, available) = if space_below >= space_above {
        (true, space_below)
    } else {
        (false, space_above)
    };

    let height = content.min(available).clamp(0.0, MAX_MENU_HEIGHT);
    let width = anchor.width.min(window.width - SCREEN_MARGIN * 2.0).max(0.0);
    let x = origin.x.min(window.width - SCREEN_MARGIN - width).max(0.0);
    let y = if below { bottom + anchor.gap } else { origin.y - height - anchor.gap };
    Placement {
        position: Point::new(x, y),
        size: Size::new(width, height),
        below,
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
    anchor: Anchor,
    searchable: bool,
}

impl<Message> MenuOverlay<'_, Message> {
    /// The height the search band takes, or nothing for a plain menu.
    fn chrome(&self) -> f32 {
        if self.searchable { SEARCH_BAND } else { 0.0 }
    }

    /// Total unclipped height of the listed rows.
    fn total_height(&self) -> f32 {
        self.state.visible.iter().map(|&index| self.items[index].height()).sum()
    }

    /// The part of the menu the rows scroll in: the menu less the search band, which
    /// sits on the side nearest the trigger.
    fn rows_area(&self, menu_bounds: Rectangle) -> Rectangle {
        let chrome = self.chrome();
        let y = if self.state.below { menu_bounds.y + chrome } else { menu_bounds.y };
        Rectangle::new(
            Point::new(menu_bounds.x, y),
            Size::new(menu_bounds.width, (menu_bounds.height - chrome).max(0.0)),
        )
    }

    /// The search band, on the side nearest the trigger. `None` for a plain menu.
    fn search_area(&self, menu_bounds: Rectangle) -> Option<Rectangle> {
        if !self.searchable {
            return None;
        }
        let y = if self.state.below {
            menu_bounds.y
        } else {
            menu_bounds.y + menu_bounds.height - SEARCH_BAND
        };
        Some(Rectangle::new(Point::new(menu_bounds.x, y), Size::new(menu_bounds.width, SEARCH_BAND)))
    }

    /// The field inside the search band, inset evenly on every side.
    fn field_area(band: Rectangle) -> Rectangle {
        let edge = (SEARCH_BAND - SEARCH_FIELD) / 2.0;
        Rectangle::new(
            Point::new(band.x + edge, band.y + edge),
            Size::new((band.width - edge * 2.0).max(0.0), SEARCH_FIELD),
        )
    }

    /// The row under the window-space point, if any.
    fn item_at(&self, rows: Rectangle, position: Point) -> Option<usize> {
        if !rows.contains(position) {
            return None;
        }
        let mut y = rows.y - self.state.scroll;
        for &index in &self.state.visible {
            let item = &self.items[index];
            let height = item.height();
            if position.y >= y && position.y < y + height {
                return item.is_selectable().then_some(index);
            }
            y += height;
        }
        None
    }

    /// The actions the query leaves listed, in order: the rows a highlight can land on.
    fn selectable(&self) -> impl Iterator<Item = usize> + '_ {
        self.state.visible.iter().copied().filter(|&index| self.items[index].is_selectable())
    }

    /// Scrolls the rows so the listed row for `index` is inside the window.
    fn reveal(&mut self, index: usize) {
        let mut top = 0.0;
        for &listed in &self.state.visible {
            let height = self.items[listed].height();
            if listed == index {
                let bottom = top + height;
                if top < self.state.scroll {
                    self.state.scroll = top;
                } else if bottom > self.state.scroll + self.state.window_height {
                    self.state.scroll = bottom - self.state.window_height;
                }
                break;
            }
            top += height;
        }
        self.state.scroll = self.state.scroll.clamp(0.0, self.state.max_scroll);
    }

    /// Moves the highlight `step` actions along, starting from the first when none is
    /// highlighted, and keeps the highlighted row in view.
    fn move_highlight(&mut self, step: isize) {
        let candidates: Vec<usize> = self.selectable().collect();
        if candidates.is_empty() {
            self.state.hovered = None;
            return;
        }
        let current = self
            .state
            .hovered
            .and_then(|hovered| candidates.iter().position(|&index| index == hovered));
        let next = match current {
            Some(at) => (at as isize + step).clamp(0, candidates.len() as isize - 1) as usize,
            None if step >= 0 => 0,
            None => candidates.len() - 1,
        };
        self.state.hovered = Some(candidates[next]);
        self.reveal(candidates[next]);
    }

    /// Applies a text edit to the query. An edit that changed the text refilters the rows,
    /// returns to the top, and highlights the first match.
    fn edit(&mut self, edit: impl FnOnce(&mut Query) -> bool) {
        if edit(&mut self.state.query) {
            self.state.visible = listed(self.items, self.state.query.text());
            self.state.scroll = 0.0;
            let first = self.selectable().next();
            self.state.hovered = first;
        }
    }

    /// Applies a key press to an open searchable menu. Returns whether the menu used the key.
    fn press(
        &mut self,
        key: &keyboard::Key,
        physical: Physical,
        modifiers: keyboard::Modifiers,
        text: Option<&str>,
        shell: &mut Shell<'_, Message>,
    ) -> bool {
        // Typed text is read first, so a keypad key types even when NumLock is off and the key
        // sends a navigation key instead.
        if let Some(typed) = search_field::typed_text(key, physical, modifiers, text) {
            self.edit(|query| query.insert(typed));
        } else {
            match key {
                keyboard::Key::Named(Named::ArrowDown) => self.move_highlight(1),
                keyboard::Key::Named(Named::ArrowUp) => self.move_highlight(-1),
                keyboard::Key::Named(Named::ArrowLeft) => self.state.query.left(),
                keyboard::Key::Named(Named::ArrowRight) => self.state.query.right(),
                keyboard::Key::Named(Named::Home) => self.state.query.home(),
                keyboard::Key::Named(Named::End) => self.state.query.end(),
                keyboard::Key::Named(Named::Backspace) => self.edit(Query::backspace),
                keyboard::Key::Named(Named::Delete) => self.edit(Query::delete),
                keyboard::Key::Named(Named::Enter) => {
                    if let Some(index) = self.state.hovered.or_else(|| self.selectable().next()) {
                        shell.publish((self.on_select)(index));
                        self.state.close();
                    }
                }
                _ => return false,
            }
        }
        // Using the field restarts the caret's blink, so the caret reads as solid while in use.
        self.state.caret_since = Instant::now();
        true
    }
}

impl<Message> Overlay<Message, Theme, BaseRenderer> for MenuOverlay<'_, Message> {
    fn layout(&mut self, _renderer: &BaseRenderer, bounds: Size) -> Node {
        // The listed rows follow the query, so refresh them before measuring.
        self.state.visible = listed(self.items, self.state.query.text());
        let chrome = self.chrome();
        let placed = place(self.anchor, bounds, self.total_height() + chrome);

        // Keep scroll in range for the (possibly changed) viewport height.
        self.state.below = placed.below;
        self.state.window_height = (placed.size.height - chrome).max(0.0);
        self.state.max_scroll = (self.total_height() - self.state.window_height).max(0.0);
        self.state.scroll = self.state.scroll.clamp(0.0, self.state.max_scroll);

        Node::new(placed.size).move_to(placed.position)
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
        let rows = self.rows_area(menu_bounds);

        match event {
            Event::Window(window::Event::RedrawRequested(now)) => {
                self.state.open.tick(*now);
                // The overlay unmounts once the close animation has finished.
                if self.state.phase == Phase::Closing && !self.state.open.is_animating() {
                    self.state.phase = Phase::Closed;
                }
                // An open search field keeps its caret blinking: wake again at the next flip.
                if self.searchable && self.state.is_open() {
                    shell.request_redraw_at(*now + search_field::next_blink(self.state.caret_since, *now));
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                // Only the rows take the pointer's highlight, so moving over the field or the
                // rest of the menu leaves a keyboard highlight where it is.
                if cursor.is_over(rows) {
                    let hovered = cursor.position().and_then(|position| self.item_at(rows, position));
                    if self.state.hovered != hovered {
                        self.state.hovered = hovered;
                        shell.request_redraw();
                    }
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
                    let on_field = self.search_area(menu_bounds).is_some_and(|band| cursor.is_over(band));
                    if on_field {
                        // The field keeps the menu open so the query can be typed.
                        shell.capture_event();
                    } else {
                        // Click inside the menu: pick the row under the pointer, or simply
                        // dismiss when the click landed on a separator.
                        if let Some(index) = cursor.position().and_then(|position| self.item_at(rows, position)) {
                            shell.publish((self.on_select)(index));
                        }
                        self.state.close();
                        shell.capture_event();
                    }
                } else {
                    // Click outside the menu: dismiss, but let the click
                    // fall through to whatever was actually under the cursor.
                    self.state.close();
                }
            }
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(Named::Escape),
                ..
            }) => {
                self.state.close();
                shell.capture_event();
            }
            Event::Keyboard(keyboard::Event::KeyPressed {
                key,
                physical_key,
                modifiers,
                text,
                ..
            }) if self.searchable && self.state.is_open() => {
                let used = self.press(key, *physical_key, *modifiers, text.as_deref(), shell);
                if used {
                    shell.request_redraw();
                    shell.capture_event();
                }
            }
            _ => {}
        }

        if self.state.open.is_animating() {
            shell.request_redraw();
        }
    }

    fn mouse_interaction(&self, layout: Layout<'_>, cursor: Cursor, _renderer: &BaseRenderer) -> Interaction {
        let menu_bounds = layout.bounds();
        if self.search_area(menu_bounds).is_some_and(|band| cursor.is_over(band)) {
            return Interaction::Text;
        }
        let rows = self.rows_area(menu_bounds);
        let over_row = cursor.position().and_then(|position| self.item_at(rows, position)).is_some();
        if over_row { Interaction::Pointer } else { Interaction::default() }
    }

    fn draw(&self, renderer: &mut BaseRenderer, _theme: &Theme, _style: &renderer::Style, layout: Layout<'_>, _cursor: Cursor) {
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

        let rows = self.rows_area(menu_bounds);
        renderer.with_layer(menu_bounds, |renderer| {
            if let Some(band) = self.search_area(menu_bounds) {
                let caret_on = search_field::caret_visible(self.state.caret_since, Instant::now());
                search_field::draw(renderer, Self::field_area(band), &self.state.query, caret_on, alpha);
            }

            // Rows, clipped to the rows window.
            renderer.with_layer(rows, |renderer| {
                let mut y = rows.y - self.state.scroll;
                for &index in &self.state.visible {
                    let item = &self.items[index];
                    let row = Rectangle::new(Point::new(rows.x, y), Size::new(rows.width, item.height()));
                    y += item.height();
                    if row.intersection(&rows).is_none() {
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
                            let is_hovered = self.state.hovered == Some(index);
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
                                        bounds: Rectangle::new(
                                            Point::new(row.x + ROW_INSET, row.y),
                                            Size::new(row.width - ROW_INSET * 2.0, row.height),
                                        ),
                                        border: Border {
                                            radius: Radius::from(8.0),
                                            ..Border::default()
                                        },
                                        ..renderer::Quad::default()
                                    },
                                    Background::Color(anim::fade(background, alpha)),
                                );
                            }

                            // Ink — danger rows are red; selected rows read green.
                            let ink = if *danger {
                                color::buttons::Danger::BACKGROUND_HOVER
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
                                rows,
                            );
                        }
                    }
                }

                // A search that matches nothing says so in the empty window.
                if self.searchable && self.state.visible.is_empty() {
                    renderer.fill_text(
                        text::Text {
                            content: lang::t("common.no_matches").to_owned(),
                            bounds: Size::new(rows.width, rows.height),
                            size: Pixels::from(12.0),
                            line_height: text::LineHeight::Relative(1.4),
                            font: fonts::Inter::MEDIUM,
                            align_x: text::Alignment::Center,
                            align_y: alignment::Vertical::Center,
                            shaping: text::Shaping::Basic,
                            wrapping: text::Wrapping::default(),
                        },
                        Point::new(rows.center_x(), rows.center_y()),
                        anim::fade(color::Text::MUTED, alpha),
                        rows,
                    );
                }
            });
        });
    }
}

/// Builds the overlay element for a host widget, if the menu is mounted.
pub(crate) fn overlay<'a, 'b, Message>(
    state: &'a mut MenuState,
    items: &'a [MenuItem],
    selected: Option<usize>,
    on_select: Option<&'a dyn Fn(usize) -> Message>,
    anchor: Anchor,
    searchable: bool,
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
        anchor,
        searchable,
    })))
}

#[cfg(test)]
#[path = "../../../tests/app/components/menu.rs"]
mod tests;
