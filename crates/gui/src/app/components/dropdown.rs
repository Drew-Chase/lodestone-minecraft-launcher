//! Dropdown — an action menu attached to an icon-button trigger.
//!
//! The trigger is the guide's `.btn-icon` — a 36×36 rounded slot that
//! brightens on hover — and the menu reuses the shared floating engine
//! ([`super::menu`]): `bg-3` surface, elevation-3 shadow, `bg-4` hover rows,
//! `--pink` ink for destructive entries and hairline separators between
//! groups (§02, §06, §07).
//!
//! ```text
//! Dropdown::new(Icon::lucide().ellipsis_vertical())
//!     .item(MenuItem::Action { label: "Duplicate".into(), icon: None, danger: false })
//!     .separator()
//!     .item(MenuItem::Action { label: "Delete".into(), icon: None, danger: true })
//!     .on_select(|i| Message::InstanceAction(i))
//! ```

use crate::app::theme::color;
use iced::Vector;
use iced::advanced::Renderer as _;
use iced::advanced::layout::{Limits, Node};
use iced::advanced::renderer;
use iced::advanced::svg::Renderer as _;
use iced::advanced::svg::{Handle, Svg};
use iced::advanced::widget::{Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget};
use iced::border::Radius;
use iced::mouse::{Cursor, Interaction};
use iced::{Border, Element, Event, Length, Point, Radians, Rectangle, Renderer as BaseRenderer, Size, Theme, mouse, overlay, window};
use iced_anim::Animated;
use iced_color_extension::rgb;

use super::anim;
use super::menu::{self, MenuItem, MenuState};

/// Trigger size — the `.btn-icon` spec is 36×36 with a 12px radius.
const TRIGGER_SIZE: f32 = 36.0;
const TRIGGER_RADIUS: f32 = 12.0;
/// Trigger icon size.
const ICON_SIZE: f32 = 18.0;

/// An animated dropdown menu attached to an icon-button trigger.
pub struct Dropdown<Message> {
    icon: &'static str,
    items: Vec<MenuItem>,
    on_select: Option<Box<dyn Fn(usize) -> Message>>,
    width: Length,
    height: Length,
    disabled: bool,
}

#[derive(Debug)]
struct State {
    menu: MenuState,
    chevron: Animated<f32>,
    is_pressed: bool,
    hovered: bool,
}

impl<Message> Dropdown<Message> {
    /// Creates a dropdown with the given trigger icon and no items yet.
    pub fn new(icon: &'static str) -> Self {
        Self {
            icon,
            items: Vec::new(),
            on_select: None,
            width: Length::Fixed(TRIGGER_SIZE),
            height: Length::Fixed(TRIGGER_SIZE),
            disabled: false,
        }
    }

    /// Appends an action row.
    pub fn item(mut self, item: MenuItem) -> Self {
        self.items.push(item);
        self
    }

    /// Appends a hairline separator.
    pub fn separator(mut self) -> Self {
        self.items.push(MenuItem::Separator);
        self
    }

    /// Replaces the whole item list.
    pub fn items<I: IntoIterator<Item = MenuItem>>(mut self, items: I) -> Self {
        self.items = items.into_iter().collect();
        self
    }

    /// Sets the message produced when an action row is selected.
    ///
    /// The index refers to the item list, separators included.
    /// Without it the control is not interactive.
    pub fn on_select(mut self, on_select: impl Fn(usize) -> Message + 'static) -> Self {
        self.on_select = Some(Box::new(on_select));
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    fn interactive(&self) -> bool {
        !self.disabled && self.on_select.is_some() && !self.items.is_empty()
    }

    /// Syncs the (unused) chevron animation — kept for state symmetry; the
    /// trigger icon itself does not rotate.
    fn sync(&self, _state: &mut State) -> bool {
        false
    }
}

impl<Message: 'static> Widget<Message, Theme, BaseRenderer> for Dropdown<Message> {
    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: self.height,
        }
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State {
            menu: MenuState::new(),
            chevron: Animated::new(0.0, anim::micro()),
            is_pressed: false,
            hovered: false,
        })
    }

    fn layout(&mut self, _tree: &mut Tree, _renderer: &BaseRenderer, limits: &Limits) -> Node {
        iced::advanced::layout::atomic(limits, self.width, self.height)
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
        if shell.is_event_captured() {
            return;
        }

        let is_over = cursor.is_over(layout.bounds());
        let state = tree.state.downcast_mut::<State>();

        match event {
            Event::Window(window::Event::RedrawRequested(now)) => {
                state.chevron.tick(*now);
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if is_over && self.interactive() {
                    state.is_pressed = true;
                    if state.menu.is_closed() {
                        state.menu.open();
                    } else {
                        state.menu.close();
                    }
                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) if state.is_pressed => {
                state.is_pressed = false;
                shell.capture_event();
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                state.hovered = is_over;
            }
            _ => {}
        }

        if self.sync(state) || state.chevron.is_animating() {
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
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State>();
        let bounds = layout.bounds();

        // Trigger — `.btn-icon`: white 4% slot, hover white 8%, open keeps the
        // hover treatment while the menu is mounted.
        let is_open = !state.menu.is_closed();
        let background = if !self.interactive() {
            rgb!(255, 255, 255, 2%)
        } else if is_open || state.hovered {
            rgb!(255, 255, 255, 8%)
        } else {
            rgb!(255, 255, 255, 4%)
        };
        let border = if is_open || (state.hovered && self.interactive()) {
            color::Surface::LINE_STRONG
        } else {
            color::Surface::LINE
        };
        renderer.fill_quad(
            renderer::Quad {
                bounds,
                border: Border {
                    radius: Radius::from(TRIGGER_RADIUS),
                    width: 1.0,
                    color: border,
                },
                ..renderer::Quad::default()
            },
            background,
        );

        // Trigger icon.
        let ink = if self.interactive() && (is_open || state.hovered) {
            color::Text::HEADLINE
        } else {
            color::Text::MUTED
        };
        let icon_svg = Svg {
            handle: Handle::from_memory(self.icon.as_bytes()),
            opacity: 1.0,
            rotation: Radians(0.0),
            color: Some(ink),
        };
        let icon_bounds = Rectangle::new(
            Point::new(bounds.center_x() - ICON_SIZE / 2.0, bounds.center_y() - ICON_SIZE / 2.0),
            Size::new(ICON_SIZE, ICON_SIZE),
        );
        renderer.draw_svg(icon_svg, icon_bounds, bounds);
    }

    fn mouse_interaction(&self, _tree: &Tree, layout: Layout<'_>, cursor: Cursor, _viewport: &Rectangle, _renderer: &BaseRenderer) -> Interaction {
        if cursor.is_over(layout.bounds()) && self.interactive() {
            Interaction::Pointer
        } else {
            Interaction::default()
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'_>,
        _renderer: &BaseRenderer,
        _viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, BaseRenderer>> {
        let state = tree.state.downcast_mut::<State>();
        let bounds = layout.bounds();

        menu::overlay::<'b, 'b, Message>(
            &mut state.menu,
            &self.items,
            None,
            self.on_select.as_deref(),
            Point::new(bounds.x + translation.x, bounds.y + translation.y),
            bounds.height,
            bounds.width.max(180.0),
        )
    }
}

impl<'a, Message: 'static + 'a> From<Dropdown<Message>> for Element<'a, Message, Theme, BaseRenderer> {
    fn from(dropdown: Dropdown<Message>) -> Self {
        Element::new(dropdown)
    }
}

#[cfg(test)]
#[path = "../../../tests/app/components/dropdown.rs"]
mod tests;
