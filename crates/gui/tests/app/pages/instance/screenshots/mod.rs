//! Tests for `pages/instance/screenshots/mod.rs`, kept out of the source file. They are compiled as the
//! `tests` child module of `screenshots`, so they can read the state directly.

use std::path::{Path, PathBuf};

use super::*;

fn shot(name: &str) -> ScreenshotFile {
    ScreenshotFile {
        file_name: name.to_string(),
        size_label: "2.1 MB".to_string(),
        path: PathBuf::from(format!("/shots/{name}")),
        modified: std::time::SystemTime::UNIX_EPOCH,
        bytes: 2_100_000,
    }
}

fn state(names: &[&str]) -> State {
    let mut state = State::default();
    state.set_shots(names.iter().map(|name| shot(name)).collect());
    state
}

fn pixel() -> Handle {
    Handle::from_rgba(1, 1, vec![0; 4])
}

#[test]
fn showing_a_capture_opens_the_viewer_on_it_and_closing_shuts_it() {
    let mut state = state(&["a.png", "b.png", "c.png"]);
    assert_eq!(state.viewing(), None);

    state.show(1);
    assert_eq!(state.viewing(), Some(1));
    state.show(9);
    assert_eq!(state.viewing(), Some(1), "an index past the last capture is ignored");

    state.close();
    assert_eq!(state.viewing(), None);
}

#[test]
fn moving_the_viewer_stays_within_the_captures() {
    let mut state = state(&["a.png", "b.png", "c.png"]);
    state.show(0);

    state.move_viewer(Move::Previous);
    assert_eq!(state.viewing(), Some(0), "the first capture has no previous one");
    state.move_viewer(Move::Last);
    assert_eq!(state.viewing(), Some(2));
    state.move_viewer(Move::Next);
    assert_eq!(state.viewing(), Some(2), "the last capture has no next one");
    state.move_viewer(Move::First);
    assert_eq!(state.viewing(), Some(0));
}

#[test]
fn the_current_capture_is_the_one_the_viewer_shows() {
    let mut state = state(&["a.png", "b.png"]);
    assert_eq!(state.current_path(), None);

    state.show(1);
    assert_eq!(state.current_path(), Some(PathBuf::from("/shots/b.png")));
}

#[test]
fn full_size_decodes_cover_the_viewers_capture_and_its_neighbours_once() {
    let mut state = state(&["a.png", "b.png", "c.png", "d.png"]);
    state.show(1);

    let mut started = state.begin_decodes();
    started.sort();
    assert_eq!(
        started,
        [
            PathBuf::from("/shots/a.png"),
            PathBuf::from("/shots/b.png"),
            PathBuf::from("/shots/c.png")
        ]
    );
    assert!(state.begin_decodes().is_empty(), "a decode in flight is not started again");
}

#[test]
fn a_full_size_image_is_kept_only_while_its_capture_is_near_the_viewer() {
    let mut state = state(&["a.png", "b.png", "c.png", "d.png"]);
    state.show(0);
    state.begin_decodes();

    state.store_full(PathBuf::from("/shots/a.png"), Some(pixel()));
    state.store_full(PathBuf::from("/shots/b.png"), None);
    assert!(state.full.contains_key(Path::new("/shots/a.png")));
    assert!(
        !state.full.contains_key(Path::new("/shots/b.png")),
        "a decode that failed keeps the thumbnail"
    );

    state.show(3);
    state.begin_decodes();
    state.store_full(PathBuf::from("/shots/d.png"), Some(pixel()));
    assert!(
        !state.full.contains_key(Path::new("/shots/a.png")),
        "an image the viewer has moved past is released"
    );
    assert!(state.full.contains_key(Path::new("/shots/d.png")));
}

#[test]
fn a_decode_for_a_capture_the_viewer_no_longer_needs_is_dropped() {
    let mut state = state(&["a.png", "b.png", "c.png", "d.png"]);
    state.show(0);
    state.begin_decodes();
    state.show(3);

    state.store_full(PathBuf::from("/shots/a.png"), Some(pixel()));
    assert!(!state.full.contains_key(Path::new("/shots/a.png")));
}

#[test]
fn removing_a_capture_moves_the_viewer_to_the_one_that_takes_its_place() {
    let mut state = state(&["a.png", "b.png", "c.png"]);
    state.show(1);

    state.remove(Path::new("/shots/b.png"));
    assert_eq!(state.shots().len(), 2);
    assert_eq!(state.viewing(), Some(1));
    assert_eq!(state.current_path(), Some(PathBuf::from("/shots/c.png")));
}

#[test]
fn removing_the_last_capture_steps_back_and_removing_the_final_one_closes_the_viewer() {
    let mut state = state(&["a.png", "b.png"]);
    state.show(1);

    state.remove(Path::new("/shots/b.png"));
    assert_eq!(state.viewing(), Some(0));
    state.remove(Path::new("/shots/a.png"));
    assert_eq!(state.viewing(), None);
    assert!(state.shots().is_empty());
}

#[test]
fn removing_an_unknown_capture_changes_nothing() {
    let mut state = state(&["a.png"]);
    state.show(0);

    state.remove(Path::new("/shots/zzz.png"));
    assert_eq!(state.shots().len(), 1);
    assert_eq!(state.viewing(), Some(0));
}

#[test]
fn a_refresh_keeps_the_thumbnails_of_captures_that_did_not_change() {
    let mut state = state(&["a.png", "b.png"]);
    for _ in 0..2 {
        let path = state.next_pending().expect("a capture waits for its thumbnail");
        state.store(path, Some(pixel()));
    }

    let mut rewritten = shot("b.png");
    rewritten.bytes = 4_000_000;
    state.set_shots(vec![shot("a.png"), rewritten]);
    assert!(
        state.thumbnail(Path::new("/shots/a.png")).is_some(),
        "an unchanged capture keeps its thumbnail"
    );
    assert!(state.thumbnail(Path::new("/shots/b.png")).is_none(), "a rewritten capture decodes again");
    assert_eq!(state.next_pending(), Some(PathBuf::from("/shots/b.png")));
}

#[test]
fn only_one_thumbnail_decodes_at_a_time() {
    let mut state = state(&["a.png", "b.png"]);
    assert_eq!(state.next_pending(), Some(PathBuf::from("/shots/a.png")));
    assert_eq!(state.next_pending(), None, "the second waits for the first");

    state.store(PathBuf::from("/shots/a.png"), Some(pixel()));
    assert_eq!(state.next_pending(), Some(PathBuf::from("/shots/b.png")));
}

#[test]
fn a_refresh_keeps_the_viewer_on_its_capture() {
    let mut state = state(&["a.png", "b.png", "c.png"]);
    state.show(2);

    state.set_shots(vec![shot("new.png"), shot("a.png"), shot("b.png"), shot("c.png")]);
    assert_eq!(state.current_path(), Some(PathBuf::from("/shots/c.png")));
    assert_eq!(state.viewing(), Some(3));
}

#[test]
fn a_refresh_moves_the_viewer_on_when_its_capture_is_gone() {
    let mut state = state(&["a.png", "b.png", "c.png"]);
    state.show(1);

    state.set_shots(vec![shot("a.png"), shot("c.png")]);
    assert_eq!(
        state.current_path(),
        Some(PathBuf::from("/shots/c.png")),
        "the capture that took its place is shown"
    );

    state.set_shots(vec![shot("a.png")]);
    assert_eq!(state.viewing(), Some(0), "the last capture gone steps the viewer back");

    state.set_shots(Vec::new());
    assert_eq!(state.viewing(), None, "with no captures left the viewer closes");
}
