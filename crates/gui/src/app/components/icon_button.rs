use crate::app::theme::color;
use iced::advanced::layout::{Limits, Node};
use iced::advanced::renderer::Style;
use iced::advanced::widget::{Tree, tree};
use iced::advanced::{Clipboard, Layout, Renderer, Shell, Widget, layout, renderer};
use iced::border::Radius;
use iced::mouse::{Cursor, Interaction};
use iced::{Border, Color, Element, Event, Length, Point, Radians, Rectangle, Renderer as BaseRenderer, Size, Theme, mouse};

pub fn secondary_button_style() -> IconButtonStyle {
    IconButtonStyle {
        foreground: color::buttons::secondary_foreground(),
        foreground_hover: color::buttons::secondary_foreground(),
        background: color::buttons::secondary_background(),
        border: Color::TRANSPARENT,
        background_hover: color::buttons::secondary_background_hover(),
        border_hover: Color::TRANSPARENT,
        // Disabled — the chip vanishes; only a dimmed icon remains.
        foreground_disabled: color::text_disabled(),
        background_disabled: Color::TRANSPARENT,
        foreground_pressed: color::buttons::secondary_foreground(),
        background_pressed: color::buttons::secondary_background_hover(),
    }
}
pub fn danger_button_style() -> IconButtonStyle {
    IconButtonStyle {
        foreground: color::buttons::danger_foreground(),
        foreground_hover: color::buttons::danger_foreground_hover(),
        background: color::buttons::danger_background(),
        border: Color::TRANSPARENT,
        background_hover: color::buttons::danger_background_hover(),
        border_hover: Color::TRANSPARENT,
        background_disabled: color::buttons::danger_background_hover(),
        foreground_disabled: color::buttons::danger_foreground_hover(),
        background_pressed: color::buttons::danger_background_hover(),
        foreground_pressed: color::buttons::danger_foreground_hover(),
    }
}
/// `.btn-icon` — white 4% fill inside a hairline ring, `--ink-2` icon. Hover
/// raises the fill to white 8%, the ring to `--line-strong` and the icon to `--ink-0`.
pub fn ghost_button_style() -> IconButtonStyle {
    IconButtonStyle {
        foreground: color::text_muted(),
        foreground_hover: color::text_headline(),
        background: color::buttons::secondary_background(),
        border: color::surface_line(),
        background_hover: color::buttons::secondary_background_hover(),
        border_hover: color::surface_line_strong(),
        foreground_disabled: color::text_disabled(),
        background_disabled: Color::TRANSPARENT,
        foreground_pressed: color::text_headline(),
        background_pressed: color::buttons::secondary_background_hover(),
    }
}

pub struct IconButton<Message> {
    icon: &'static str,
    width: Length,
    height: Length,
    icon_size: Length,
    style: IconButtonStyle,
    radius: Radius,
    on_press: Option<Message>,
    disabled: bool,
}
#[derive(Default)]
pub struct IconButtonState {
    is_hovered: bool,
    is_pressed: bool,
}

/// `border` and `border_hover` draw a 1px ring; a transparent ring draws none.
pub struct IconButtonStyle {
    pub foreground: Color,
    pub background: Color,
    pub border: Color,
    pub foreground_hover: Color,
    pub background_hover: Color,
    pub border_hover: Color,
    pub background_disabled: Color,
    pub foreground_disabled: Color,
    pub background_pressed: Color,
    pub foreground_pressed: Color,
}

impl<Message> Default for IconButton<Message> {
    fn default() -> Self {
        Self {
            icon: Default::default(),
            width: Length::Fixed(50.0),
            height: Length::Fixed(50.0),
            icon_size: Length::Fixed(16.0),
            // The `.btn-icon` radius (`--r-md`): an icon button is rounded unless it sets its own.
            radius: Radius::from(12.0),
            on_press: None,
            style: Default::default(),
            disabled: false,
        }
    }
}

impl Default for IconButtonStyle {
    fn default() -> Self {
        secondary_button_style()
    }
}

impl<Message> IconButton<Message> {
    pub fn new(icon: &'static str) -> Self {
        Self {
            icon,
            ..IconButton::default()
        }
    }
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }
    pub fn size(mut self, size: impl Into<Length>) -> Self {
        self.width = size.into();
        self.height = self.width;
        self
    }
    pub fn icon_size(mut self, icon_size: impl Into<Length>) -> Self {
        self.icon_size = icon_size.into();
        self
    }
    pub fn style(mut self, style: impl Into<IconButtonStyle>) -> Self {
        self.style = style.into();
        self
    }
    pub fn radius(mut self, radius: impl Into<Radius>) -> Self {
        self.radius = radius.into();
        self
    }
    pub fn on_press(mut self, on_press: Message) -> Self {
        self.on_press = Some(on_press);
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl<Message: Clone> Widget<Message, Theme, BaseRenderer> for IconButton<Message> {
    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: self.height,
        }
    }

    fn layout(&mut self, _tree: &mut Tree, _renderer: &BaseRenderer, limits: &Limits) -> Node {
        layout::atomic(limits, self.width, self.height)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut BaseRenderer,
        _theme: &Theme,
        _style: &Style,
        layout: Layout<'_>,
        _cursor: Cursor,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<IconButtonState>();
        let (fg, bg, border_color) = if self.disabled || self.on_press.is_none() {
            (self.style.foreground_disabled, self.style.background_disabled, Color::TRANSPARENT)
        } else if state.is_pressed && state.is_hovered {
            (self.style.foreground_pressed, self.style.background_pressed, self.style.border_hover)
        } else if state.is_hovered {
            (self.style.foreground_hover, self.style.background_hover, self.style.border_hover)
        } else {
            (self.style.foreground, self.style.background, self.style.border)
        };
        let bounds = layout.bounds();
        renderer.fill_quad(
            renderer::Quad {
                bounds,
                border: Border {
                    // No ring unless the style sets one, so borderless buttons draw as before.
                    width: if border_color.a > 0.0 { 1.0 } else { 0.0 },
                    color: border_color,
                    radius: self.radius,
                },
                ..Default::default()
            },
            bg,
        );
        use iced::advanced::svg::Renderer;
        use iced::advanced::svg::{Handle, Svg};
        let icon_svg = Svg {
            handle: Handle::from_memory(self.icon.as_bytes()),
            opacity: 1.0,
            rotation: Radians(0.0),
            color: Some(fg),
        };
        let icon_size: f32 = match self.icon_size {
            Length::Fixed(size) => size,
            _ => 16.0, // Provide a default fallback value.
        };
        let icon_position = Point::new(bounds.center_x() - icon_size / 2.0, bounds.center_y() - icon_size / 2.0);
        let icon_bounds: Rectangle = Rectangle::new(icon_position, Size::new(icon_size, icon_size));
        renderer.draw_svg(icon_svg, icon_bounds, bounds);
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<IconButtonState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(IconButtonState::default())
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
        let state = tree.state.downcast_mut::<IconButtonState>();

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if self.on_press.is_some() && is_over && !self.disabled {
                    state.is_pressed = true;
                    shell.capture_event();
                    shell.request_redraw();
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) if state.is_pressed => {
                state.is_pressed = false;
                if is_over && let Some(on_press) = self.on_press.clone() {
                    shell.publish(on_press);
                }
                shell.capture_event();
            }

            _ => {}
        }

        if state.is_hovered != is_over {
            state.is_hovered = is_over;
            shell.request_redraw()
        }
    }

    fn mouse_interaction(&self, _tree: &Tree, layout: Layout<'_>, cursor: Cursor, _viewport: &Rectangle, _renderer: &BaseRenderer) -> Interaction {
        if cursor.is_over(layout.bounds()) {
            Interaction::Pointer
        } else {
            Interaction::default()
        }
    }
}

impl<'a, Message: Clone + 'a> From<IconButton<Message>> for Element<'a, Message, Theme, BaseRenderer> {
    fn from(button: IconButton<Message>) -> Self {
        Element::new(button)
    }
}

#[cfg(test)]
#[path = "../../../tests/app/components/icon_button.rs"]
mod tests;
