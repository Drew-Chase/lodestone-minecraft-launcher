//! A tile: a clickable block of artwork, such as an icon, a swatch, a symbol or a banner.
//! It shows the pointer over it, a faint outline and lifted fill while hovered, and a green
//! ring when it is the chosen one.

use iced::widget::button;
use iced::{Background, Border, Element, Length};
use iced_color_extension::rgb;

use crate::app::theme::color;

/// The gap between a tile's frame and its content. The frame is drawn beneath the content, so
/// the content sits this far inside it and the ring stays visible around the artwork.
pub const INSET: f32 = 2.0;

/// A clickable tile: `content` in a frame with `radius` corners. The content fills the frame
/// less [`INSET`] on each side. `on_press` is sent when the tile is clicked, and the frame rings
/// in green while `selected`.
pub fn tile<'a, M: Clone + 'a>(
    content: impl Into<Element<'a, M>>,
    width: impl Into<Length>,
    height: impl Into<Length>,
    radius: f32,
    selected: bool,
    on_press: M,
) -> Element<'a, M> {
    button(content)
        .width(width)
        .height(height)
        .padding(INSET)
        .style(move |_, status| style(status, radius, selected))
        .on_press(on_press)
        .into()
}

/// The frame in each state. A chosen tile rings in green. A hovered or pressed one gets a
/// faint white outline and a lifted fill. At rest it keeps the hairline.
fn style(status: button::Status, radius: f32, selected: bool) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    let (width, edge, fill) = if selected {
        (2.0, color::Primary::DEFAULT, None)
    } else if hovered {
        (1.0, rgb!(255, 255, 255, 30%), Some(rgb!(255, 255, 255, 4%)))
    } else {
        (1.0, color::Surface::LINE, None)
    };
    button::Style {
        background: fill.map(Background::Color),
        text_color: color::Text::DEFAULT,
        border: Border {
            radius: radius.into(),
            width,
            color: edge,
        },
        ..button::Style::default()
    }
}

#[cfg(test)]
#[path = "../../../tests/app/components/tile.rs"]
mod tests;
