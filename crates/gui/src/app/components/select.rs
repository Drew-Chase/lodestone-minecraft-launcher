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

use crate::app::lang;
use crate::app::theme::{color, fonts};
use aurora_leaf::Icon;
use iced::Vector;
use iced::advanced::Renderer as _;
use iced::advanced::layout::{Limits, Node};
use iced::advanced::renderer;
use iced::advanced::svg::Renderer as _;
use iced::advanced::svg::{Handle, Svg};
use iced::advanced::text;
use iced::advanced::text::Paragraph as _;
use iced::advanced::text::Renderer as _;
use iced::advanced::widget::{Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget};
use iced::border::Radius;
use iced::mouse::{Cursor, Interaction};
use iced::{
    Border, Color, Element, Event, Font, Length, Pixels, Point, Radians, Rectangle, Renderer as BaseRenderer, Size, Theme, alignment, mouse, overlay,
    window,
};
use iced_anim::Animated;
use iced_color_extension::rgb;

use super::anim;
use super::menu::{self, Anchor, GAP, MenuItem, MenuState};

/// Horizontal padding inside the trigger.
const H_PADDING: f32 = 12.0;
/// Trigger height — 13px text plus the `.select` padding.
const TRIGGER_HEIGHT: f32 = 36.0;
/// Gap between the label and the chevron.
const CHEVRON_GAP: f32 = 8.0;
/// Chevron icon size.
const CHEVRON_SIZE: f32 = 16.0;
/// The chip's insets and chevron. The chip is the language picker's look.
const CHIP_LEFT: f32 = 16.0;
const CHIP_RIGHT: f32 = 12.0;
const CHIP_CHEVRON_SIZE: f32 = 12.0;
/// Size of a leading icon, and the gap between it and the label.
const LEADING_SIZE: f32 = 14.0;
const LEADING_GAP: f32 = 8.0;
/// With a caption, the label sits this far above the trigger's middle and the caption this far below it.
const CAPTION_OFFSET: f32 = 9.0;
/// The badge's height, its text size, its side padding, and the gap between it and the label.
const BADGE_HEIGHT: f32 = 18.0;
const BADGE_TEXT: f32 = 10.0;
const BADGE_PADDING: f32 = 6.0;
const BADGE_GAP: f32 = 8.0;

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
    /// Whether the trigger draws as the dark chip (see [`Select::chip`]).
    chip: bool,
    /// Markup of an icon drawn before the label, when set.
    leading_icon: Option<&'static str>,
    /// A line under the label, in mono muted type, when set. The trigger should be taller to fit it.
    caption: Option<String>,
    /// A pill drawn right after the label, when set, such as a "latest" marker.
    badge: Option<String>,
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
            placeholder: lang::t("common.select").to_owned(),
            width: Length::Fill,
            height: Length::Fixed(TRIGGER_HEIGHT),
            disabled: false,
            searchable: false,
            chip: false,
            leading_icon: None,
            caption: None,
            badge: None,
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

    /// Draws the trigger as a chip: a dark fill with no border, deepening on hover and
    /// while the menu is open. The language picker's chip style draws it.
    pub fn chip(mut self) -> Self {
        self.chip = true;
        self
    }

    /// Draws `markup` as an icon before the label.
    pub fn leading_icon(mut self, markup: &'static str) -> Self {
        self.leading_icon = Some(markup);
        self
    }

    /// Adds a caption under the label, such as what the selected option means. Give the trigger a height
    /// that fits both lines.
    pub fn caption(mut self, caption: impl Into<String>) -> Self {
        self.caption = Some(caption.into());
        self
    }

    /// Draws `badge` as a small pill right after the label, such as a "latest" marker.
    pub fn badge(mut self, badge: impl Into<String>) -> Self {
        self.badge = Some(badge.into());
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
        let lifted = !state.menu.is_closed() || state.hovered;
        let (background, border, border_width) = if self.chip {
            let fill = if self.interactive() && lifted {
                rgb!(0, 0, 0, 55%)
            } else {
                rgb!(0, 0, 0, 40%)
            };
            (fill, Color::TRANSPARENT, 0.0)
        } else if !self.interactive() {
            (rgb!(255, 255, 255, 2%), color::Surface::LINE, 1.0)
        } else if lifted {
            (rgb!(255, 255, 255, 6%), color::Surface::LINE_STRONG, 1.0)
        } else {
            (rgb!(255, 255, 255, 4%), color::Surface::LINE, 1.0)
        };
        renderer.fill_quad(
            renderer::Quad {
                bounds,
                border: Border {
                    radius: Radius::from(12.0),
                    width: border_width,
                    color: border,
                },
                ..renderer::Quad::default()
            },
            background,
        );

        // Label — `fill_text` with owned content: a paragraph built locally
        // during draw is only weakly held by the render pass and is silently
        // skipped once dropped (see the menu engine note).
        // The chip keeps its own insets and smaller type; the field keeps the `.select` spec.
        let (left, right, text_size, chevron_size) = if self.chip {
            (CHIP_LEFT, CHIP_RIGHT, 12.0, CHIP_CHEVRON_SIZE)
        } else {
            (H_PADDING, H_PADDING, 13.0, CHEVRON_SIZE)
        };
        // A leading icon sits at the left inset, and the label follows it.
        let leading = if self.leading_icon.is_some() { LEADING_SIZE + LEADING_GAP } else { 0.0 };
        if let Some(markup) = self.leading_icon {
            let icon_svg = Svg {
                handle: Handle::from_memory(markup.as_bytes()),
                opacity: 0.68,
                rotation: Radians(0.0),
                color: Some(color::Text::MUTED),
            };
            let icon_bounds = Rectangle::new(
                Point::new(bounds.x + left, bounds.center_y() - LEADING_SIZE / 2.0),
                Size::new(LEADING_SIZE, LEADING_SIZE),
            );
            renderer.draw_svg(icon_svg, icon_bounds, bounds);
        }

        // With a caption the label rises above the middle, and the caption sits below it.
        let label_y = if self.caption.is_some() {
            bounds.center_y() - CAPTION_OFFSET
        } else {
            bounds.center_y()
        };
        let label = self.label();
        let label_font = fonts::Inter::MEDIUM;
        let label_width = text_width(label, text_size, label_font);
        renderer.fill_text(
            text::Text {
                content: label.to_owned(),
                bounds: Size::new(bounds.width - left - right - chevron_size - CHEVRON_GAP - leading, bounds.height),
                size: Pixels::from(text_size),
                line_height: text::LineHeight::Relative(1.4),
                font: label_font,
                align_x: text::Alignment::Default,
                align_y: alignment::Vertical::Center,
                shaping: text::Shaping::Basic,
                wrapping: text::Wrapping::default(),
            },
            Point::new(bounds.x + left + leading, label_y),
            if self.has_value() {
                color::Text::DEFAULT
            } else {
                color::Text::PLACEHOLDER
            },
            *viewport,
        );

        if let Some(badge) = &self.badge {
            let font = fonts::JetBrainsMono::SEMI_BOLD;
            let width = text_width(badge, BADGE_TEXT, font) + 2.0 * BADGE_PADDING;
            let x = bounds.x + left + leading + label_width + BADGE_GAP;
            renderer.fill_quad(
                renderer::Quad {
                    bounds: Rectangle::new(Point::new(x, label_y - BADGE_HEIGHT / 2.0), Size::new(width, BADGE_HEIGHT)),
                    border: Border {
                        radius: 999.0.into(),
                        ..Border::default()
                    },
                    ..renderer::Quad::default()
                },
                color::Primary::TINT,
            );
            // iced centres aligned text on its anchor point, so the badge text starts at its inset rather than
            // being centred on the pill's left edge, where it drew over the number.
            renderer.fill_text(
                text::Text {
                    content: badge.clone(),
                    bounds: Size::new(width, BADGE_HEIGHT),
                    size: Pixels::from(BADGE_TEXT),
                    line_height: text::LineHeight::Relative(1.4),
                    font,
                    align_x: text::Alignment::Default,
                    align_y: alignment::Vertical::Center,
                    shaping: text::Shaping::Basic,
                    wrapping: text::Wrapping::None,
                },
                Point::new(x + BADGE_PADDING, label_y),
                color::Primary::DEFAULT,
                *viewport,
            );
        }

        if let Some(caption) = &self.caption {
            renderer.fill_text(
                text::Text {
                    content: caption.clone(),
                    bounds: Size::new(bounds.width - left - right - chevron_size - CHEVRON_GAP - leading, bounds.height),
                    size: Pixels::from(11.0),
                    line_height: text::LineHeight::Relative(1.4),
                    font: fonts::JetBrainsMono::REGULAR,
                    align_x: text::Alignment::Default,
                    align_y: alignment::Vertical::Center,
                    shaping: text::Shaping::Basic,
                    wrapping: text::Wrapping::default(),
                },
                Point::new(bounds.x + left + leading, bounds.center_y() + CAPTION_OFFSET),
                color::Text::MUTED,
                *viewport,
            );
        }

        // Chevron — rotates 180° while the menu is open.
        let chevron_svg = Svg {
            handle: Handle::from_memory(Icon::lucide().chevron_down().as_bytes()),
            opacity: 1.0,
            rotation: Radians(*state.chevron.value()),
            color: Some(color::Text::MUTED),
        };
        let chevron_bounds = Rectangle::new(
            Point::new(bounds.x + bounds.width - right - chevron_size, bounds.center_y() - chevron_size / 2.0),
            Size::new(chevron_size, chevron_size),
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
                gap: GAP,
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

/// The natural width of `content` at `size` in `font`, so a badge can sit right after the text.
fn text_width(content: &str, size: f32, font: Font) -> f32 {
    <BaseRenderer as text::Renderer>::Paragraph::with_text(text::Text {
        content,
        bounds: Size::INFINITE,
        size: Pixels::from(size),
        line_height: text::LineHeight::Relative(1.4),
        font,
        align_x: text::Alignment::Default,
        align_y: alignment::Vertical::Center,
        shaping: text::Shaping::Basic,
        wrapping: text::Wrapping::None,
    })
    .min_width()
}

#[cfg(test)]
#[path = "../../../tests/app/components/select.rs"]
mod tests;
