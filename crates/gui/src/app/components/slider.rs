//! Slider style — the style guide's green range track and round handle.
//!
//! ```text
//! slider(2.0..=16.0, memory, Message::Memory).style(slider::style)
//! ```

use crate::app::theme::color;
use iced::widget::slider;
use iced::{Background, Border, Theme};
use iced_color_extension::rgb;

/// The rail fills green up to the value and stays white 8% beyond it. The
/// handle grows slightly while it is hovered or dragged.
pub fn style(_theme: &Theme, status: slider::Status) -> slider::Style {
    let radius = match status {
        slider::Status::Active => 7.0,
        slider::Status::Hovered => 8.0,
        slider::Status::Dragged => 9.0,
    };

    slider::Style {
        rail: slider::Rail {
            backgrounds: (Background::Color(color::Primary::DEFAULT), Background::Color(rgb!(255, 255, 255, 8%))),
            width: 4.0,
            border: Border {
                radius: 2.0.into(),
                ..Border::default()
            },
        },
        handle: slider::Handle {
            shape: slider::HandleShape::Circle { radius },
            background: Background::Color(color::Primary::DEFAULT),
            border_width: 3.0,
            border_color: color::Surface::APPLICATION,
        },
    }
}
