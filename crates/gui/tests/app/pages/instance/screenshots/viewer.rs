//! Tests for `pages/instance/screenshots/viewer.rs`, kept out of the source file. They are compiled as the
//! `tests` child module of `viewer`.

use super::*;

#[test]
fn a_movement_lands_on_a_neighbour_or_an_end() {
    assert_eq!(moved(2, 5, Move::Previous), 1);
    assert_eq!(moved(2, 5, Move::Next), 3);
    assert_eq!(moved(3, 5, Move::First), 0);
    assert_eq!(moved(1, 5, Move::Last), 4);
}

#[test]
fn a_movement_past_an_end_is_not_a_move() {
    assert!(!can_move(0, 5, Move::Previous));
    assert!(can_move(0, 5, Move::Next));
    assert!(!can_move(4, 5, Move::Next));
    assert!(!can_move(0, 1, Move::Next), "a single capture has nowhere to go");
}

#[test]
fn the_full_size_window_is_the_capture_first_then_its_neighbours() {
    assert_eq!(needed(0, 3), [0, 1]);
    assert_eq!(needed(1, 3), [1, 0, 2]);
    assert_eq!(needed(2, 3), [2, 1]);
    assert!(needed(0, 0).is_empty());
}

#[test]
fn removing_a_capture_keeps_the_viewer_on_the_capture_it_shows() {
    assert_eq!(after_removal(Some(2), 0, 4), Some(1), "the shown capture shifts back one place");
    assert_eq!(after_removal(Some(1), 3, 4), Some(1), "a capture after the shown one leaves it alone");
    assert_eq!(after_removal(Some(1), 1, 3), Some(1), "the capture after the shown one takes its place");
    assert_eq!(after_removal(Some(2), 2, 2), Some(1), "the last capture gives way to the one before it");
    assert_eq!(after_removal(Some(0), 0, 0), None, "nothing is left to show");
    assert_eq!(after_removal(None, 0, 3), None, "a closed viewer stays closed");
}

#[test]
fn the_counter_counts_from_one() {
    assert_eq!(counter(2, 48), "3 / 48");
    assert_eq!(counter(0, 1), "1 / 1");
}
