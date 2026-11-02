//! Tests for `nav.rs`, kept out of the source file. They compile as its `tests` child module.

use super::*;

/// Three sections at tops 0, 400 and 900, as a page with two tall sections above the last one lays them out.
fn offsets() -> Vec<(Section, f32)> {
    vec![(Section::General, 0.0), (Section::Window, 400.0), (Section::DangerZone, 900.0)]
}

#[test]
fn the_section_at_the_top_is_active() {
    assert_eq!(active(&offsets(), 0.0, false), Some(Section::General));
    assert_eq!(active(&offsets(), 380.0, false), Some(Section::Window), "its top is within the spy line");
    assert_eq!(active(&offsets(), 500.0, false), Some(Section::Window));
}

#[test]
fn a_section_becomes_active_only_once_its_top_is_near_the_top_edge() {
    assert_eq!(
        active(&offsets(), 350.0, false),
        Some(Section::General),
        "the Window top is still 50px below the line"
    );
    assert_eq!(
        active(&offsets(), 360.0, false),
        Some(Section::Window),
        "the Window top has reached the spy line"
    );
}

#[test]
fn the_bottom_of_the_page_activates_the_last_section() {
    assert_eq!(active(&offsets(), 1200.0, true), Some(Section::DangerZone));
}

#[test]
fn content_shorter_than_the_page_never_counts_as_at_the_end() {
    let short = [(Section::General, 0.0), (Section::Window, 60.0)];
    assert_eq!(active(&short, 0.0, false), Some(Section::General));
    assert_eq!(active(&[], 0.0, false), None);
}
