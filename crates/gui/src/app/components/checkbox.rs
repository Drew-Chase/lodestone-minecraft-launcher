//! The checkbox the export tree uses. It is a button with a glyph, because iced's checkbox has no
//! indeterminate state, and the tree's header shows one when only some of its rows are taken.

use aurora_leaf::Icon;
use iced::widget::{button, container, svg};
use iced::{Background, Border, Element, Length, alignment};
use iced_color_extension::rgb;

use crate::app::theme::color;

/// The side of the box.
pub const SIZE: f32 = 16.0;
/// The side of the glyph inside the box.
const GLYPH: f32 = 11.0;

/// Whether a box is ticked, half ticked, or empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Checked,
    Partial,
    Unchecked,
}

/// A checkbox in `state` that sends `on_press` when it is pressed.
pub fn checkbox<'a, Message: Clone + 'a>(state: State, on_press: Message) -> Element<'a, Message> {
    let glyph = match state {
        State::Checked => Some(Icon::lucide().check()),
        State::Partial => Some(Icon::lucide().minus()),
        State::Unchecked => None,
    };
    let filled = state != State::Unchecked;
    let mark = glyph.map(|markup| {
        svg(svg::Handle::from_memory(markup.as_bytes()))
            .width(GLYPH)
            .height(GLYPH)
            .style(|_, _| svg::Style {
                color: Some(color::primary_on()),
            })
    });
    let inner = container(mark)
        .width(SIZE)
        .height(SIZE)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center);
    button(inner)
        .width(Length::Fixed(SIZE))
        .height(Length::Fixed(SIZE))
        .padding(0)
        .style(move |_, _| button::Style {
            background: Some(Background::Color(if filled { color::primary() } else { rgb!(255, 255, 255, 4%) })),
            border: Border {
                radius: 4.0.into(),
                width: 1.0,
                color: if filled { color::primary() } else { color::surface_line_strong() },
            },
            ..button::Style::default()
        })
        .on_press(on_press)
        .into()
}
