use super::*;

#[test]
fn content_width_takes_the_sidebar_and_both_margins_off_a_narrow_window() {
    assert_eq!(content_width(1280.0), 1280.0 - 68.0 - 56.0);
}

#[test]
fn content_width_stops_growing_at_the_cap() {
    // The cap is reached at a window of 1920 plus the sidebar.
    assert_eq!(content_width(1920.0 + 68.0), MAX_CONTENT_WIDTH - 56.0);
    assert_eq!(content_width(2560.0), MAX_CONTENT_WIDTH - 56.0);
    assert_eq!(content_width(3840.0), MAX_CONTENT_WIDTH - 56.0);
}
