//! Tests for `pages/worlds/list.rs`, kept out of the source file. They are compiled as the `tests` child module of
//! `list`, so they can read the window the list builds.

use super::*;

#[test]
fn the_grid_takes_as_many_cards_as_the_width_allows() {
    assert_eq!(grid_columns(1155.0), 3, "the reference's content width");
    assert_eq!(grid_columns(700.0), 2);
    assert_eq!(grid_columns(300.0), 1, "never fewer than one");
    assert_eq!(grid_columns(3000.0), MAX_COLUMNS, "never more than the cap");
}

#[test]
fn only_the_rows_around_the_scroll_are_built() {
    let mut state = State {
        visible: (0..100).collect(),
        ..State::default()
    };
    let viewport = Size::new(1200.0, 800.0);
    let pitch = CARD_HEIGHT + GAP;

    let top = window(state.scroll_y, state.visible.len(), viewport, 3, pitch);
    assert_eq!((top.first, top.end, top.total), (0, 7, 34));

    state.scroll_y = 2470.0;
    let scrolled = window(state.scroll_y, state.visible.len(), viewport, 3, pitch);
    assert_eq!(
        (scrolled.first, scrolled.end),
        (8, 15),
        "one row above the scroll, the viewport and two more below"
    );
}
