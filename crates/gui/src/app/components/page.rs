//! The width of a page's content: the window less the sidebar, capped at [`MAX_CONTENT_WIDTH`], less the
//! margin either side. Every page that lays out by width reads it here, so the cap holds on every page.

use crate::app::components::sidebar;

/// The widest a page's content column grows. Wider windows keep this width and centre the column, so
/// ultrawide and 4K screens do not stretch its cards and rows.
pub const MAX_CONTENT_WIDTH: f32 = 1920.0;

/// The margin either side of a page's content.
pub const PAGE_SIDE: f32 = 28.0;

/// The width of a page's content in a window `window_width` wide: the window less the sidebar, capped at
/// [`MAX_CONTENT_WIDTH`], less [`PAGE_SIDE`] on each side.
pub fn content_width(window_width: f32) -> f32 {
    (window_width - sidebar::WIDTH).min(MAX_CONTENT_WIDTH) - 2.0 * PAGE_SIDE
}

#[cfg(test)]
#[path = "../../../tests/app/components/page.rs"]
mod tests;
