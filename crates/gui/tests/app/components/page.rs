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

#[test]
fn the_800_by_600_floor_lays_out_narrow() {
    // 800 less the 68px rail and the 56px of margins leaves 676px of content.
    assert_eq!(content_width(800.0), 676.0);
    assert!(is_narrow(800.0));
}

#[test]
fn narrow_switches_at_the_content_width_threshold() {
    // The page content is 959px at a 1083px window, just under the threshold, and 960px at 1084px.
    assert!(is_narrow(1083.0));
    assert!(!is_narrow(1084.0));
    assert!(!is_narrow(1280.0));
}

#[test]
fn short_is_a_window_under_the_height_threshold() {
    assert!(is_short(600.0));
    assert!(is_short(699.0));
    assert!(!is_short(700.0));
    assert!(!is_short(720.0));
}
