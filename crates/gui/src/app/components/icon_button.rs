use crate::app::theme::color;
use iced::advanced::layout::{Limits, Node};
use iced::advanced::renderer::Style;
use iced::advanced::widget::{Tree, tree};
use iced::advanced::{Clipboard, Layout, Renderer, Shell, Widget, layout, renderer};
use iced::border::Radius;
use iced::mouse::{Cursor, Interaction};
use iced::{
    Border, Color, Element, Event, Length, Point, Radians, Rectangle, Renderer as BaseRenderer,
    Size, Theme, mouse,
};

pub const PRIMARY_BUTTON_STYLE: IconButtonStyle = IconButtonStyle {
    foreground: color::buttons::Primary::FOREGROUND,
    foreground_hover: color::buttons::Primary::FOREGROUND,
    background: color::buttons::Primary::BACKGROUND,
    background_hover: color::buttons::Primary::BACKGROUND_HOVER,
    foreground_disabled: color::buttons::Primary::FOREGROUND,
    background_disabled: color::buttons::Primary::BACKGROUND_DISABLED,
    foreground_pressed: color::buttons::Primary::FOREGROUND,
    background_pressed: color::buttons::Primary::BACKGROUND_DISABLED,
};
pub const SECONDARY_BUTTON_STYLE: IconButtonStyle = IconButtonStyle {
    foreground: color::buttons::Secondary::FOREGROUND,
    foreground_hover: color::buttons::Secondary::FOREGROUND,
    background: color::buttons::Secondary::BACKGROUND,
    background_hover: color::buttons::Secondary::BACKGROUND_HOVER,
    foreground_disabled: color::buttons::Secondary::FOREGROUND,
    background_disabled: color::buttons::Secondary::BACKGROUND_HOVER,
    foreground_pressed: color::buttons::Secondary::FOREGROUND,
    background_pressed: color::buttons::Secondary::BACKGROUND_HOVER,
};
#[cfg(windows)]
pub const DANGER_BUTTON_STYLE: IconButtonStyle = IconButtonStyle {
    foreground: color::buttons::Danger::FOREGROUND,
    foreground_hover: color::buttons::Danger::FOREGROUND_HOVER,
    background: color::buttons::Danger::BACKGROUND,
    background_hover: color::buttons::Danger::BACKGROUND_HOVER,
    background_disabled: color::buttons::Danger::BACKGROUND_HOVER,
    foreground_disabled: color::buttons::Danger::FOREGROUND_HOVER,
    background_pressed: color::buttons::Danger::BACKGROUND_HOVER,
    foreground_pressed: color::buttons::Danger::FOREGROUND_HOVER,
};

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

pub struct IconButtonStyle {
    pub foreground: Color,
    pub background: Color,
    pub foreground_hover: Color,
    pub background_hover: Color,
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
            radius: Default::default(),
            on_press: None,
            style: Default::default(),
            disabled: false,
        }
    }
}

impl Default for IconButtonStyle {
    fn default() -> Self {
        SECONDARY_BUTTON_STYLE
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
        let (fg, bg) = if self.disabled || self.on_press.is_none() {
            (
                self.style.foreground_disabled,
                self.style.background_disabled,
            )
        } else if state.is_pressed && state.is_hovered {
            (self.style.foreground_pressed, self.style.background_pressed)
        } else if state.is_hovered {
            (self.style.foreground_hover, self.style.background_hover)
        } else {
            (self.style.foreground, self.style.background)
        };
        let bounds = layout.bounds();
        renderer.fill_quad(
            renderer::Quad {
                bounds,
                border: Border {
                    width: 0.0,
                    color: Color::TRANSPARENT,
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
        let icon_position = Point::new(
            bounds.center_x() - icon_size / 2.0,
            bounds.center_y() - icon_size / 2.0,
        );
        let icon_bounds: Rectangle = Rectangle::new(icon_position, Size::new(icon_size, icon_size));
        renderer.draw_svg(icon_svg, icon_bounds, bounds);
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

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        layout: Layout<'_>,
        cursor: Cursor,
        _viewport: &Rectangle,
        _renderer: &BaseRenderer,
    ) -> Interaction {
        if cursor.is_over(layout.bounds()) {
            Interaction::Pointer
        } else {
            Interaction::default()
        }
    }
}

impl<'a, Message: Clone + 'a> From<IconButton<Message>>
    for Element<'a, Message, Theme, BaseRenderer>
{
    fn from(button: IconButton<Message>) -> Self {
        Element::new(button)
    }
}
