//! The scrollbar every scrolling area shares: a 6px white thumb inside a 16px
//! transparent track, so the thumb sits 5px in from each side. It rests at half
//! opacity and goes to full while the pointer is over its track, or while the
//! thumb is being dragged.
//!
//! The app runs on iced's built-in `Theme::Dark`, which takes no custom
//! scrollable style catalog. So each scrolling area is styled through [`scroll`]
//! rather than a bare `scrollable(...)`.

use iced::widget::scrollable::{Direction, Rail, Scrollbar, Scroller, Status, Style};
use iced::widget::{container, scrollable};
use iced::{Border, Element, Theme, border};
use iced_color_extension::rgb;

/// The width of the track, which is the room the bar takes beside the content.
pub const TRACK_WIDTH: f32 = 16.0;
/// The width of the thumb. It sits `(TRACK_WIDTH - THUMB_WIDTH) / 2` in from each side of the track.
const THUMB_WIDTH: f32 = 6.0;

/// A scrollable over `content` that shows the shared scrollbar.
pub fn scroll<'a, Message>(content: impl Into<Element<'a, Message>>) -> scrollable::Scrollable<'a, Message> {
    scrollable(content)
        .direction(Direction::Vertical(Scrollbar::new().width(TRACK_WIDTH).scroller_width(THUMB_WIDTH)))
        .style(style)
}

/// The style of a scrollable. The rails stay transparent, and only the bar under
/// the pointer, or being dragged, lights its thumb.
pub fn style(theme: &Theme, status: Status) -> Style {
    let (vertical, horizontal) = match status {
        Status::Active { .. } => (false, false),
        Status::Hovered {
            is_vertical_scrollbar_hovered,
            is_horizontal_scrollbar_hovered,
            ..
        } => (is_vertical_scrollbar_hovered, is_horizontal_scrollbar_hovered),
        Status::Dragged {
            is_vertical_scrollbar_dragged,
            is_horizontal_scrollbar_dragged,
            ..
        } => (is_vertical_scrollbar_dragged, is_horizontal_scrollbar_dragged),
    };

    Style {
        container: container::Style::default(),
        vertical_rail: rail(vertical),
        horizontal_rail: rail(horizontal),
        gap: None,
        // iced's own auto-scroll overlay, so middle-click scrolling looks as it did.
        auto_scroll: scrollable::default(theme, status).auto_scroll,
    }
}

/// A rail with no fill and no border, so the track stays transparent. Its thumb is
/// white: at half opacity, or full when `lit`.
fn rail(lit: bool) -> Rail {
    let thumb = if lit { rgb!(255, 255, 255, 100%) } else { rgb!(255, 255, 255, 50%) };

    Rail {
        background: None,
        border: Border::default(),
        scroller: Scroller {
            background: thumb.into(),
            // The renderer clamps this to half the thumb's shorter side, so both ends are round.
            border: border::rounded(u32::MAX),
        },
    }
}

#[cfg(test)]
#[path = "../../../tests/app/components/scrollbar.rs"]
mod tests;
