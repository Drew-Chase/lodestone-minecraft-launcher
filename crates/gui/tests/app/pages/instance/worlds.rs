//! Tests for `pages/instance/worlds.rs`, kept out of the source file. They are
//! compiled as the `tests` child module of `worlds`, so they can read the
//! visible indices directly.

use chrono::{Local, TimeZone};

use super::*;

fn at(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> DateTime<Local> {
    Local
        .with_ymd_and_hms(year, month, day, hour, minute, 0)
        .single()
        .expect("a valid local time")
}

fn world(name: &str) -> WorldFile {
    WorldFile {
        name: name.to_string(),
        game_mode: Some(GameMode::Survival),
        difficulty: Some(Difficulty::Normal),
        seed: Some(1),
        version: Some("1.20.1".to_string()),
        size_bytes: 1,
        saved: Some(at(2026, 10, 29, 12, 0)),
        icon: None,
    }
}

#[test]
fn the_saved_caption_is_relative_to_now() {
    let now = at(2026, 10, 29, 12, 0);
    assert_eq!(format_saved(at(2026, 10, 29, 2, 14), now), "Saved 02:14 today");
    assert_eq!(format_saved(at(2026, 10, 28, 20, 0), now), "Saved Yesterday");
    assert_eq!(format_saved(at(2026, 10, 26, 12, 0), now), "Saved 3 days ago");
    assert_eq!(format_saved(at(2026, 10, 20, 12, 0), now), "Saved Last week");
    assert_eq!(format_saved(at(2026, 10, 14, 12, 0), now), "Saved 2 weeks ago");
    assert_eq!(format_saved(at(2026, 4, 8, 20, 0), now), "Saved Apr 8, 2026");
}

#[test]
fn search_matches_names_ignoring_case() {
    let mut state = State::default();
    state.set_worlds(
        vec![world("Aether Hub"), world("Aether Copy"), world("Pixel Town")],
        at(2026, 10, 29, 12, 0),
    );
    assert_eq!(state.visible, [0, 1, 2]);

    state.update(Message::Search("AETHER".to_string()));
    assert_eq!(state.visible, [0, 1]);

    state.update(Message::Search("no world has this name".to_string()));
    assert!(state.visible.is_empty());
}

#[test]
fn display_only_actions_leave_the_grid_alone() {
    let mut state = State::default();
    state.set_worlds(vec![world("Aether Hub")], at(2026, 10, 29, 12, 0));
    state.update(Message::Import);
    state.update(Message::NewWorld);
    state.update(Message::Play);
    state.update(Message::Reveal);
    state.update(Message::More);
    assert_eq!(state.visible, [0]);
}
