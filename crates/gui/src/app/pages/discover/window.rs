//! The results' virtual window. Only the rows on screen, and a couple either side, are built, and spacers stand in for the rest,
//! so the page keeps the height of the whole list. The arithmetic is pure and tested. The page-facing helpers read the page's
//! own layout, so the rows that are built and the rows whose pictures are asked for are always the same rows.

use super::card::{COMPACT_ROW_HEIGHT, GRID_CARD_HEIGHT, TABLE_ROW_HEIGHT};
use super::results::{CARD_MIN_WIDTH, COMPACT_TWO_COLUMNS, GAP as RESULTS_GAP, TABLE_HEADER_HEIGHT};
use super::{Discover, DiscoverView, FILTERS_PANEL_HEIGHT, GAP as PAGE_GAP, SIDE, filters_top, hero, sidebar, toolbar};
use crate::app::components::page;
use crate::app::components::scrollbar;
use crate::app::components::sidebar as rail;

/// How many rows above the screen are built, so a row that scrolls into view is already there.
const OVERSCAN_ABOVE: usize = 1;
/// How many rows below the screen are built.
const OVERSCAN_BELOW: usize = 2;

/// The rows a layout builds: `first..end` of `total` rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rows {
    pub first: usize,
    pub end: usize,
    pub total: usize,
}

/// How a layout arranges its results: the items in a row, the height of a row, and the gap between rows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shape {
    pub columns: usize,
    pub row_height: f32,
    pub gap: f32,
}

impl Shape {
    /// The distance from the top of one row to the top of the next.
    pub fn pitch(self) -> f32 {
        self.row_height + self.gap
    }
}

/// The shape of `view`'s results in a column `width` wide.
pub fn shape(view: DiscoverView, width: f32) -> Shape {
    match view {
        DiscoverView::Grid => Shape {
            columns: grid_columns(width),
            row_height: GRID_CARD_HEIGHT,
            gap: RESULTS_GAP,
        },
        DiscoverView::Compact => Shape {
            columns: compact_columns(width),
            row_height: COMPACT_ROW_HEIGHT,
            gap: RESULTS_GAP,
        },
        DiscoverView::Table => Shape {
            columns: 1,
            row_height: TABLE_ROW_HEIGHT,
            gap: 0.0,
        },
    }
}

/// The grid's columns in a `width`-wide column: as many cards as fit at their minimum width, four at most.
pub fn grid_columns(width: f32) -> usize {
    let available = width.max(CARD_MIN_WIDTH);
    ((available + RESULTS_GAP) / (CARD_MIN_WIDTH + RESULTS_GAP)).floor().clamp(1.0, 4.0) as usize
}

/// The compact rows' columns in a `width`-wide column: two side by side once it is wide enough, one otherwise.
pub fn compact_columns(width: f32) -> usize {
    if width.max(320.0) >= COMPACT_TWO_COLUMNS { 2 } else { 1 }
}

/// The rows to build: those on screen, and the overscan either side. `scroll` is how far the content has scrolled, `top` is
/// where the first row starts in the content, `viewport` is how tall the window is, and `pitch` is the distance between rows.
pub fn rows(scroll: f32, top: f32, viewport: f32, pitch: f32, total: usize) -> Rows {
    if total == 0 {
        return Rows { first: 0, end: 0, total };
    }
    let offset = (scroll - top).max(0.0);
    let visible_first = (offset / pitch).floor() as usize;
    let visible_end = ((offset + viewport) / pitch).ceil() as usize;
    let first = visible_first.saturating_sub(OVERSCAN_ABOVE).min(total - 1);
    let end = (visible_end + OVERSCAN_BELOW).clamp(first + 1, total);
    Rows { first, end, total }
}

/// The spacer above the built rows: the height of the rows before `first`, less the gap the line leaves beneath it, so the
/// first built row starts where it would in the whole list. `None` when no row lies above.
pub fn top_spacer(rows: Rows, pitch: f32, gap: f32) -> Option<f32> {
    (rows.first > 0).then_some(rows.first as f32 * pitch - gap)
}

/// The spacer below the built rows, sized the same way for the rows after `end`. `None` when no row lies below.
pub fn bottom_spacer(rows: Rows, pitch: f32, gap: f32) -> Option<f32> {
    (rows.end < rows.total).then_some((rows.total - rows.end) as f32 * pitch - gap)
}

/// Whether the filter panel is open beside the results: the narrow window's panel, while the sidebar has filters to show.
fn panel_open(page: &Discover, narrow: bool) -> bool {
    narrow && page.filters_open && sidebar::shown(page)
}

/// Where the results start in the scrolling content, in a window `window_width` wide: below the hero, the filter row and the
/// toolbar, and below the filter panel while it is open. The table's header sits above its rows, so it counts too.
pub fn results_top(page: &Discover, window_width: f32) -> f32 {
    let narrow = page::is_narrow(window_width);
    let mut top = filters_top(hero::shown(page)) + toolbar::HEIGHT + PAGE_GAP;
    if panel_open(page, narrow) {
        top += FILTERS_PANEL_HEIGHT + PAGE_GAP;
    }
    if matches!(page.view, DiscoverView::Table) {
        top += TABLE_HEADER_HEIGHT;
    }
    top
}

/// The width of the results in a window `window_width` wide, as the page lays them out: the window less the rail, the filter
/// column when it shows (the margin beside it when it does not), and the page's right margin.
pub fn results_width(page: &Discover, window_width: f32) -> f32 {
    let narrow = page::is_narrow(window_width);
    let filters = if !narrow && sidebar::shown(page) {
        sidebar::WIDTH + scrollbar::TRACK_WIDTH
    } else {
        SIDE
    };
    (window_width - rail::WIDTH - filters - SIDE).max(0.0)
}

#[cfg(test)]
#[path = "../../../../tests/app/pages/discover/window.rs"]
mod tests;
