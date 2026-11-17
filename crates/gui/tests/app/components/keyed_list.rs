use super::*;

/// Rematches `old` (the state stored under `old_keys`) to `new_keys`, with `"new"` for a key that was not there.
fn moved(old_keys: &[Key], old: &[&'static str], new_keys: &[Key]) -> Vec<&'static str> {
    rematch(old_keys, old.to_vec(), new_keys, |_| "new")
}

#[test]
fn a_window_that_slides_down_keeps_each_rows_state_with_its_row() {
    // The list shows rows 1 to 3 and scrolls to rows 2 to 4. The count is unchanged, which is the case iced's own keyed
    // column matches by position.
    assert_eq!(moved(&[1, 2, 3], &["one", "two", "three"], &[2, 3, 4]), ["two", "three", "new"]);
}

#[test]
fn a_window_that_slides_up_keeps_each_rows_state_with_its_row() {
    assert_eq!(moved(&[2, 3, 4], &["two", "three", "four"], &[1, 2, 3]), ["new", "two", "three"]);
}

#[test]
fn rows_that_reorder_take_their_state_with_them() {
    assert_eq!(moved(&[1, 2, 3], &["one", "two", "three"], &[3, 1, 2]), ["three", "one", "two"]);
}

#[test]
fn a_row_that_leaves_and_returns_starts_fresh() {
    let gone = moved(&[1, 2], &["one", "two"], &[2]);
    assert_eq!(gone, ["two"]);
    assert_eq!(moved(&[2], &gone, &[1, 2]), ["new", "two"]);
}

#[test]
fn a_list_that_grows_or_shrinks_keeps_the_rows_it_still_has() {
    assert_eq!(moved(&[1, 2], &["one", "two"], &[1, 5, 2, 6]), ["one", "new", "two", "new"]);
    assert_eq!(moved(&[1, 5, 2, 6], &["one", "five", "two", "six"], &[5, 6]), ["five", "six"]);
}

#[test]
fn a_fresh_row_is_made_for_its_own_place_in_the_list() {
    let made = rematch(&[1], vec![usize::MAX], &[7, 1, 8], |index| index);
    assert_eq!(made, [0, usize::MAX, 2]);
}

#[test]
fn a_key_used_twice_gives_the_second_row_fresh_state() {
    assert_eq!(moved(&[1, 2], &["one", "two"], &[1, 1]), ["one", "new"]);
}

#[test]
fn a_windowed_list_builds_the_rows_on_screen_and_the_overscan() {
    // 400px of 40px rows shows ten. Scrolled to row 20, with 2 rows of overscan, rows 18 to 31 are built.
    assert_eq!(visible_rows(800.0, 400.0, 100, 40.0, 2), 18..32);
    assert_eq!(visible_rows(0.0, 400.0, 100, 40.0, 2), 0..12, "nothing is built above the first row");
    assert_eq!(visible_rows(800.0, 400.0, 25, 40.0, 2), 18..25, "or past the last");
}

#[test]
fn a_scroll_past_a_list_that_shrank_builds_its_last_rows() {
    assert_eq!(visible_rows(4000.0, 400.0, 5, 40.0, 2), 4..5);
    assert_eq!(visible_rows(4000.0, 400.0, 0, 40.0, 2), 0..0);
}

#[test]
fn one_name_always_has_one_key() {
    assert_eq!(key_of("config/sodium.json"), key_of("config/sodium.json"));
    assert_ne!(key_of("config/sodium.json"), key_of("config/lithium.json"));
}
