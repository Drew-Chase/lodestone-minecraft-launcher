//! Panels — the static glass surfaces behind the instance page's cards.
//!
//! A panel is a white 4%→2% vertical gradient over a 1px hairline with an
//! 18px radius. [`divided_panel`] stacks rows with hairlines between them.
//!
//! ```text
//! panel(column![section_label("SESSION"), session_rows])
//! divided_panel(rows, Padding::new(0.0).left(18).right(18))
//! ```

use crate::app::theme::color;
use iced::widget::{Space, column, container};
use iced::{Background, Border, Color, Element, Fill, Gradient, Padding, gradient};
use iced_color_extension::rgb;

/// The default panel padding, per the reference.
pub const PADDING: f32 = 18.0;

/// A panel with the default padding and the hairline border.
pub fn panel<'a, M: 'a>(content: impl Into<Element<'a, M>>) -> Element<'a, M> {
    panel_with_padding(content, PADDING)
}

/// A panel with the given padding and the hairline border.
pub fn panel_with_padding<'a, M: 'a>(content: impl Into<Element<'a, M>>, padding: impl Into<Padding>) -> Element<'a, M> {
    bordered_panel(content, padding, color::Surface::LINE)
}

/// A panel whose border is `border` — the danger zone draws a red one.
pub fn bordered_panel<'a, M: 'a>(content: impl Into<Element<'a, M>>, padding: impl Into<Padding>, border: Color) -> Element<'a, M> {
    container(content)
        .width(Fill)
        .padding(padding)
        .style(move |_| container::Style {
            background: Some(Background::Gradient(Gradient::Linear(
                gradient::Linear::new(std::f32::consts::PI)
                    .add_stop(0.0, rgb!(255, 255, 255, 4%))
                    .add_stop(1.0, rgb!(255, 255, 255, 2%)),
            ))),
            border: Border {
                radius: 18.0.into(),
                width: 1.0,
                color: border,
            },
            ..container::Style::default()
        })
        .into()
}

/// A panel of rows separated by hairlines.
pub fn divided_panel<'a, M: 'a>(rows: Vec<Element<'a, M>>, padding: impl Into<Padding>) -> Element<'a, M> {
    panel_with_padding(divided(rows), padding)
}

/// Rows stacked with hairlines between them, without a surface of their own.
pub fn divided<'a, M: 'a>(rows: Vec<Element<'a, M>>) -> Element<'a, M> {
    let mut stacked = Vec::with_capacity(rows.len().saturating_mul(2));
    for (index, row) in rows.into_iter().enumerate() {
        if index > 0 {
            stacked.push(hairline());
        }
        stacked.push(row);
    }
    column(stacked).width(Fill).into()
}

/// A 1px full-width rule in the hairline color.
pub fn hairline<'a, M: 'a>() -> Element<'a, M> {
    container(Space::new().width(Fill).height(1))
        .width(Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(color::Surface::LINE)),
            ..container::Style::default()
        })
        .into()
}
