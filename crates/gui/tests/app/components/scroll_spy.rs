//! Tests for `scroll_spy.rs`, kept out of the source file. They compile as its `tests` child module.

use super::*;

/// Three sections at tops 0, 400 and 900, as a page with two tall sections above the last one lays them out.
fn offsets() -> Vec<(&'static str, f32)> {
    vec![("general", 0.0), ("network", 400.0), ("privacy", 900.0)]
}

#[test]
fn the_section_at_the_top_is_active() {
    assert_eq!(active(&offsets(), 0.0, false), Some("general"));
    assert_eq!(active(&offsets(), 380.0, false), Some("network"), "its top is within the spy line");
    assert_eq!(active(&offsets(), 500.0, false), Some("network"));
}

#[test]
fn a_section_becomes_active_only_once_its_top_is_near_the_top_edge() {
    assert_eq!(
        active(&offsets(), 350.0, false),
        Some("general"),
        "the network top is still 50px below the line"
    );
    assert_eq!(
        active(&offsets(), 360.0, false),
        Some("network"),
        "the network top has reached the spy line"
    );
}

#[test]
fn the_bottom_of_the_page_activates_the_last_section() {
    assert_eq!(active(&offsets(), 1200.0, true), Some("privacy"));
}

#[test]
fn content_shorter_than_the_page_never_counts_as_at_the_end() {
    let short = [("general", 0.0), ("network", 60.0)];
    assert_eq!(active(&short, 0.0, false), Some("general"));
    assert_eq!(active::<&str>(&[], 0.0, false), None);
}
