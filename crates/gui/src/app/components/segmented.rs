//! Segmented control — mutually exclusive text options in one hairline group.
//!
//! ```text
//! segmented(&LAUNCH_MODES, mode, Message::LaunchMode)
//! ```

use crate::app::components::TextButton;
use crate::app::components::button::Variant;
use crate::app::theme::{color, fonts};
use iced::widget::{Text, container, row};
use iced::{Background, Border, Element, Padding};
use iced_color_extension::rgb;

/// Builds one group: a cell per `(value, label)` option, with the selected
/// cell lifted onto a brighter fill.
pub fn segmented<'a, T, M>(options: &[(T, &'static str)], selected: T, on_select: impl Fn(T) -> M) -> Element<'a, M>
where
    T: Copy + PartialEq,
    M: Clone + 'a,
{
    let cells: Vec<Element<'a, M>> = options
        .iter()
        .map(|&(value, label)| {
            let active = value == selected;
            let ink = if active { color::Text::HEADLINE } else { color::Text::MUTED };
            let button = TextButton::content(Text::new(label).size(12).font(fonts::Inter::MEDIUM).color(ink))
                .variant(Variant::Tab)
                .padding(Padding::new(0.0).left(12).right(12).top(6).bottom(6))
                .on_press(on_select(value));

            container(button)
                .style(move |_| container::Style {
                    background: active.then_some(Background::Color(rgb!(255, 255, 255, 10%))),
                    border: Border {
                        radius: 8.0.into(),
                        ..Border::default()
                    },
                    ..container::Style::default()
                })
                .into()
        })
        .collect();

    container(row(cells).spacing(2))
        .padding(2)
        .style(|_| container::Style {
            background: Some(Background::Color(rgb!(0, 0, 0, 20%))),
            border: Border {
                radius: 10.0.into(),
                width: 1.0,
                color: color::Surface::LINE,
            },
            ..container::Style::default()
        })
        .into()
}
