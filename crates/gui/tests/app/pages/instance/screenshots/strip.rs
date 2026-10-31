//! Tests for `pages/instance/screenshots/strip.rs`, kept out of the source file. They are compiled as the
//! `tests` child module of `strip`.

use super::*;

#[test]
fn the_row_shows_an_odd_number_of_thumbnails_from_three_to_eleven() {
    assert_eq!(visible_thumbnails(0.0), 3, "even the narrowest row shows three");
    assert_eq!(visible_thumbnails(62.0 * 3.0 - GAP), 3);
    assert_eq!(visible_thumbnails(62.0 * 4.0 - GAP), 3, "an even count rounds down to the odd one below");
    assert_eq!(visible_thumbnails(62.0 * 5.0 - GAP), 5);
    assert_eq!(visible_thumbnails(62.0 * 11.0 - GAP), 11);
    assert_eq!(visible_thumbnails(62.0 * 30.0), 11, "no more than eleven thumbnails");
}

#[test]
fn the_row_slides_to_keep_the_shown_capture_in_the_middle() {
    assert_eq!(window_start(0, 20, 11), 0, "the first captures show from the start");
    assert_eq!(window_start(4, 20, 11), 0, "the middle capture still fits the first page");
    assert_eq!(window_start(10, 20, 11), 5, "the shown capture sits in the middle");
    assert_eq!(window_start(19, 20, 11), 9, "the last capture holds the row to the end");
    assert_eq!(window_start(2, 5, 11), 0, "fewer captures than thumbnails show from the start");
}
