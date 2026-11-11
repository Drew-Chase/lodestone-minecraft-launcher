use super::*;
use crate::app::pages::instance::files::ModFile;

#[test]
fn the_window_covers_the_viewport_with_overscan_on_both_sides() {
    // 600px shows nine rows of 73px, and eight more are built past the bottom edge. Nothing lies above the top.
    assert_eq!(
        window(0.0, 600.0, 1_000),
        Window {
            first: 0,
            end: 17,
            total: 1_000
        }
    );
}

#[test]
fn the_window_moves_with_the_scroll() {
    // 6,100px down, at 73px a row, is row 83, so rows 75 to 99 are built.
    assert_eq!(
        window(6_100.0, 600.0, 1_000),
        Window {
            first: 75,
            end: 100,
            total: 1_000
        }
    );
}

#[test]
fn a_short_list_builds_every_row() {
    assert_eq!(window(0.0, 600.0, 3), Window { first: 0, end: 3, total: 3 });
}

#[test]
fn a_scroll_past_a_shrunken_list_builds_its_last_rows() {
    // A filter that shrinks the list leaves the scroll where it was, so the last rows are the ones built.
    assert_eq!(window(50_000.0, 600.0, 5), Window { first: 4, end: 5, total: 5 });
}

#[test]
fn an_empty_list_builds_no_rows() {
    assert_eq!(window(0.0, 600.0, 0), Window { first: 0, end: 0, total: 0 });
}

#[test]
fn a_mod_without_metadata_is_described_by_its_file_name() {
    let row = Row {
        file: ModFile {
            file_name: "mystery.jar".to_string(),
            enabled: true,
            name: "mystery".to_string(),
            author: None,
            version: None,
            id: None,
            dependencies: Vec::new(),
            icon: None,
        },
        source: None,
        update: None,
        library: false,
    };
    assert_eq!(subtitle(&row), "mystery.jar");
}

#[test]
fn a_mod_names_its_author_and_version_when_it_declares_them() {
    let row = Row {
        file: ModFile {
            file_name: "sodium.jar".to_string(),
            enabled: true,
            name: "Sodium".to_string(),
            author: Some("jellysquid3".to_string()),
            version: Some("0.5.8".to_string()),
            id: None,
            dependencies: Vec::new(),
            icon: None,
        },
        source: None,
        update: None,
        library: false,
    };
    let line = subtitle(&row);
    assert!(line.contains("jellysquid3") && line.contains("0.5.8"), "got {line}");
}
