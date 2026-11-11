//! The small rounded labels the pages share: a tag, a loader or a Minecraft version, and a loader's name as it is shown.

use iced::widget::{Row, Text, container};
use iced::{Background, Border, Element, Fill, Padding};

use crate::app::theme::{color, fonts};

/// The gap between chips along a line, and between the lines they wrap onto.
const GAP: f32 = 6.0;

/// Chips laid out left to right, onto a new line when the next one would run past the width the row is given. Each chip
/// keeps its own width, so its label stays inside its outline; a plain row squeezes its last chips instead.
pub fn chip_row<'a, M: 'a>(chips: Vec<Element<'a, M>>) -> Element<'a, M> {
    Row::with_children(chips).spacing(GAP).width(Fill).wrap().vertical_spacing(GAP).into()
}

/// A small rounded label: a tag, a loader or a Minecraft version.
pub fn chip<'a, M: 'a>(label: impl Into<String>) -> Element<'a, M> {
    container(
        Text::new(label.into())
            .size(11)
            .font(fonts::JetBrainsMono::REGULAR)
            .color(color::text_muted()),
    )
    .padding(Padding::new(0.0).left(8).right(8).top(3).bottom(3))
    .style(|_| container::Style {
        background: Some(Background::Color(color::surface_bg3())),
        border: Border {
            radius: 999.0.into(),
            width: 1.0,
            color: color::surface_line(),
        },
        ..container::Style::default()
    })
    .into()
}

/// A loader's name as it is shown: `fabric` is `Fabric`, and `neoforge` is `NeoForge`.
pub fn loader_label(name: &str) -> String {
    match name {
        "neoforge" => "NeoForge".to_string(),
        other => {
            let mut chars = other.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect(),
                None => String::new(),
            }
        }
    }
}
