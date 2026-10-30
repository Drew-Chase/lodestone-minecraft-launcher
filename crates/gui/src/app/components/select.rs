//! Select — the style guide's `.select` dropdown (§07).
//!
//! The trigger follows the `.select` spec — white 4% fill, hairline border,
//! 12px radius, 13px label — with a chevron that rotates 180° while open.
//! Options render in the shared floating menu ([`super::menu`]): `bg-3`
//! surface, `bg-4` hover rows, Lode Green highlight on the selected row.
//!
//! ```text
//! Select::new(vec!["Vanilla", "Fabric", "Quilt"], Some(self.loader))
//!     .on_select(|i| Message::LoaderSelected(i))
//! ```

use crate::app::theme::{color, fonts};
use aurora_leaf::Icon;
use iced::Vector;
use iced::advanced::Renderer as _;
use iced::advanced::layout::{Limits, Node};
use iced::advanced::renderer;
use iced::advanced::svg::Renderer as _;
use iced::advanced::svg::{Handle, Svg};
use iced::advanced::text;
use iced::advanced::text::Renderer as _;
use iced::advanced::widget::{Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget};
use iced::border::Radius;
use iced::mouse::{Cursor, Interaction};
use iced::{
    Border, Element, Event, Length, Pixels, Point, Radians, Rectangle, Renderer as BaseRenderer, Size, Theme, alignment, mouse, overlay, window,
};
use iced_anim::Animated;
use iced_color_extension::rgb;

use super::anim;
use super::menu::{self, Anchor, MenuItem, MenuState};

/// Horizontal padding inside the trigger.
const H_PADDING: f32 = 12.0;
/// Trigger height — 13px text plus the `.select` padding.
const TRIGGER_HEIGHT: f32 = 36.0;
/// Gap between the label and the chevron.
const CHEVRON_GAP: f32 = 8.0;
/// Chevron icon size.
const CHEVRON_SIZE: f32 = 16.0;

/// An animated select dropdown (`.select` + floating menu).
pub struct Select<'a, Message> {
    options: Vec<String>,
    /// Menu entries, derived from `options` once at construction so the
    /// overlay can borrow them for the frame without rebuilding.
    items: Vec<MenuItem>,
    selected: Option<usize>,
    on_select: Option<Box<dyn Fn(usize) -> Message + 'a>>,
    placeholder: String,
    width: Length,
    height: Length,
    disabled: bool,
    /// Whether the menu carries a filter field.
    searchable: bool,
}

#[derive(Debug)]
struct State {
    menu: MenuState,
    chevron: Animated<f32>,
    is_pressed: bool,
    hovered: bool,
}

impl<'a, Message: 'a> Select<'a, Message> {
    /// Creates a select with the given options and selected index.
    pub fn new<I: IntoIterator<Item = impl Into<String>>>(options: I, selected: Option<usize>) -> Self {
        let options: Vec<String> = options.into_iter().map(Into::into).collect();
        let items = options
            .iter()
            .map(|option| MenuItem::Action {
                label: option.clone(),
                icon: None,
                danger: false,
            })
            .collect();
        Self {
            options,
            items,
            selected,
            on_select: None,
            placeholder: String::from("Select…"),
            width: Length::Fill,
            height: Length::Fixed(TRIGGER_HEIGHT),
            disabled: false,
            searchable: false,
        }
    }

    /// Sets the message produced when an option is selected.
    ///
    /// The index refers to the option list given to [`Select::new`].
    /// Without it the control is not interactive.
    pub fn on_select(mut self, on_select: impl Fn(usize) -> Message + 'a) -> Self {
        self.on_select = Some(Box::new(on_select));
        self
    }

    /// Text shown when nothing is selected.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
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

    /// Adds a filter field to the menu. Typing narrows the options to those whose labels
    /// contain the text, ignoring case, and Enter picks the highlighted one. The field sits
    /// on the side of the trigger the menu opens from. The query clears each time the menu
    /// opens.
    pub fn searchable(mut self) -> Self {
        self.searchable = true;
        self
    }

    fn interactive(&self) -> bool {
        !self.disabled && self.on_select.is_some() && !self.options.is_empty()
    }

    /// The trigger's label — the selected option, else the placeholder.
    fn label(&self) -> &str {
        self.selected
            .and_then(|index| self.options.get(index))
            .map(String::as_str)
            .unwrap_or(&self.placeholder)
    }

    /// Whether the label is a real value (ink) or a placeholder (muted ink).
    fn has_value(&self) -> bool {
        self.selected.is_some_and(|index| index < self.options.len())
    }

    /// Syncs the chevron rotation with the menu phase.
    fn sync_chevron(&self, state: &mut State) -> bool {
        let target = if state.menu.is_open() { std::f32::consts::PI } else { 0.0 };
        if state.chevron.target() != &target {
            state.chevron.set_target(target);
            true
        } else {
            false
        }
    }

    /// Toggle the menu on trigger press. `Closing` counts as closed: the
    /// overlay's outside-click already dismissed it, and a second press
    /// must not re-open it mid-animation.
    fn toggle_menu(&self, state: &mut State) {
        if state.menu.is_closed() {
            state.menu.open();
        } else {
            state.menu.close();
        }
    }
}

impl<Message: 'static> Widget<Message, Theme, BaseRenderer> for Select<'_, Message> {
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

        let bounds = layout.bounds();
        let is_over = cursor.is_over(bounds);
        let state = tree.state.downcast_mut::<State>();

        match event {
            Event::Window(window::Event::RedrawRequested(now)) => {
                // The menu's open animation is ticked by the overlay while it
                // is mounted; here we only drive the chevron.
                state.chevron.tick(*now);
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if is_over && self.interactive() {
                    state.is_pressed = true;
                    self.toggle_menu(state);
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

        if self.sync_chevron(state) || state.chevron.is_animating() {
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

        // Trigger — `.select`: white 4% fill, hairline border; hover and open
        // states lift to white 6% with a `--line-strong` border (§07 spec).
        let (background, border) = if !self.interactive() {
            (rgb!(255, 255, 255, 2%), color::Surface::LINE)
        } else if !state.menu.is_closed() || state.hovered {
            (rgb!(255, 255, 255, 6%), color::Surface::LINE_STRONG)
        } else {
            (rgb!(255, 255, 255, 4%), color::Surface::LINE)
        };
        renderer.fill_quad(
            renderer::Quad {
                bounds,
                border: Border {
                    radius: Radius::from(12.0),
                    width: 1.0,
                    color: border,
                },
                ..renderer::Quad::default()
            },
            background,
        );

        // Label — `fill_text` with owned content: a paragraph built locally
        // during draw is only weakly held by the render pass and is silently
        // skipped once dropped (see the menu engine note).
        let label = self.label();
        renderer.fill_text(
            text::Text {
                content: label.to_owned(),
                bounds: Size::new(bounds.width - H_PADDING * 2.0 - CHEVRON_SIZE - CHEVRON_GAP, bounds.height),
                size: Pixels::from(13.0),
                line_height: text::LineHeight::Relative(1.4),
                font: fonts::Inter::MEDIUM,
                align_x: text::Alignment::Default,
                align_y: alignment::Vertical::Center,
                shaping: text::Shaping::Basic,
                wrapping: text::Wrapping::default(),
            },
            Point::new(bounds.x + H_PADDING, bounds.center_y()),
            if self.has_value() {
                color::Text::DEFAULT
            } else {
                color::Text::PLACEHOLDER
            },
            *viewport,
        );

        // Chevron — rotates 180° while the menu is open.
        let chevron_svg = Svg {
            handle: Handle::from_memory(Icon::lucide().chevron_down().as_bytes()),
            opacity: 1.0,
            rotation: Radians(*state.chevron.value()),
            color: Some(color::Text::MUTED),
        };
        let chevron_bounds = Rectangle::new(
            Point::new(bounds.x + bounds.width - H_PADDING - CHEVRON_SIZE, bounds.center_y() - CHEVRON_SIZE / 2.0),
            Size::new(CHEVRON_SIZE, CHEVRON_SIZE),
        );
        renderer.draw_svg(chevron_svg, chevron_bounds, bounds);
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
            self.selected,
            self.on_select.as_deref(),
            Anchor {
                origin: Point::new(bounds.x + translation.x, bounds.y + translation.y),
                trigger_height: bounds.height,
                width: bounds.width,
            },
            self.searchable,
        )
    }
}

impl<'a, Message: 'a + 'static> From<Select<'a, Message>> for Element<'a, Message, Theme, BaseRenderer> {
    fn from(select: Select<'a, Message>) -> Self {
        Element::new(select)
    }
}
