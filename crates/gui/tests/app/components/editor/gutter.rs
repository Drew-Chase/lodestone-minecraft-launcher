use super::*;

#[test]
fn the_gutter_is_as_wide_as_the_last_lines_number_and_never_under_two_digits() {
    assert_eq!(width_for(1, 8.0), 2.0 * 8.0 + INSET * 2.0);
    assert_eq!(width_for(99, 8.0), 2.0 * 8.0 + INSET * 2.0);
    assert_eq!(width_for(100, 8.0), 3.0 * 8.0 + INSET * 2.0);
    assert_eq!(width_for(12345, 8.0), 5.0 * 8.0 + INSET * 2.0);
    assert_eq!(width_for(0, 8.0), width_for(1, 8.0), "an empty text still has a line");
}

#[test]
fn only_the_numbers_of_the_lines_on_screen_are_drawn() {
    // 20px lines under 12px of room, in a gutter 100px tall.
    assert_eq!(visible(0.0, 12.0, 20.0, 100.0, 1000), 0..5);
    assert_eq!(
        visible(12.0, 12.0, 20.0, 100.0, 1000),
        0..5,
        "scrolled past the room above the first line"
    );
    assert_eq!(visible(212.0, 12.0, 20.0, 100.0, 1000), 10..15);
    assert_eq!(
        visible(222.0, 12.0, 20.0, 100.0, 1000),
        10..16,
        "a line cut by either edge still shows its number"
    );
}

#[test]
fn a_short_text_draws_no_numbers_past_its_last_line() {
    assert_eq!(visible(0.0, 12.0, 20.0, 400.0, 3), 0..3);
    assert_eq!(visible(900.0, 12.0, 20.0, 400.0, 3), 3..3, "scrolled past the end, nothing is drawn");
}
