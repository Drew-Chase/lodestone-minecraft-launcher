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

/// The content width under which a page takes its narrow layout. Toolbars wrap onto a second row, side panels stack or
/// collapse, and a settings sidebar moves above its content. A 1280px window keeps the wide layouts; the 800px floor
/// takes the narrow ones.
pub const NARROW_BELOW: f32 = 960.0;

/// The window height under which a page shortens its hero band, so the first rows of its list show on a short screen.
pub const SHORT_BELOW: f32 = 700.0;

/// Whether a window `window_width` wide lays its pages out narrow.
pub fn is_narrow(window_width: f32) -> bool {
    content_width(window_width) < NARROW_BELOW
}

/// Whether a window `window_height` tall is short.
pub fn is_short(window_height: f32) -> bool {
    window_height < SHORT_BELOW
}

#[cfg(test)]
#[path = "../../../tests/app/components/page.rs"]
mod tests;
