//! Tests for `pages/instance/mods.rs`, kept out of the source file. They are
//! compiled as the `tests` child module of `mods`, so they can read the rows
//! and the visible indices directly.

use super::*;

fn entry(name: &str, author: Option<&str>, enabled: bool) -> ModFile {
    ModFile {
        file_name: format!("{}.jar", name.to_lowercase().replace(' ', "-")),
        enabled,
        name: name.to_string(),
        author: author.map(str::to_string),
        version: Some("1.0.0".to_string()),
    }
}

fn sample() -> State {
    let mut state = State::default();
    state.set_mods(vec![
        entry("Sodium", Some("JellySquid"), true),
        entry("Iris Shaders", Some("coderbot"), true),
        entry("Lithium", Some("CaffeineMC"), false),
    ]);
    state
}

fn visible_names(state: &State) -> Vec<&str> {
    state.visible.iter().map(|&index| state.entries[index].name.as_str()).collect()
}

#[test]
fn an_empty_filter_shows_every_mod() {
    assert_eq!(visible_names(&sample()), ["Sodium", "Iris Shaders", "Lithium"]);
}

#[test]
fn text_matches_names_and_authors_ignoring_case() {
    let mut state = sample();
    state.update(Message::Query("SODIUM".to_string()));
    assert_eq!(visible_names(&state), ["Sodium"]);
    state.update(Message::Query("caffeine".to_string()));
    assert_eq!(visible_names(&state), ["Lithium"], "authors are searched too");
}

#[test]
fn toggling_changes_only_that_mod() {
    let mut state = sample();
    state.update(Message::Toggle { index: 2, enabled: true });
    assert!(state.entries[2].enabled);
    assert!(state.entries[0].enabled, "untouched mods keep their state");
    state.update(Message::Toggle { index: 0, enabled: false });
    assert!(!state.entries[0].enabled);
}

#[test]
fn toggling_past_the_end_is_ignored() {
    let mut state = sample();
    state.update(Message::Toggle { index: 99, enabled: false });
    assert_eq!(state.entries.len(), 3);
}

#[test]
fn display_only_actions_leave_the_list_alone() {
    let mut state = sample();
    state.update(Message::Add);
    state.update(Message::More);
    assert_eq!(visible_names(&state), ["Sodium", "Iris Shaders", "Lithium"]);
}

#[test]
fn a_mod_without_metadata_is_described_by_its_file_name() {
    let bare = ModFile {
        file_name: "bare-1.0.jar".to_string(),
        enabled: true,
        name: "bare-1.0".to_string(),
        author: None,
        version: None,
    };
    assert_eq!(subtitle(&bare), "bare-1.0.jar");
    assert_eq!(subtitle(&entry("Sodium", Some("JellySquid"), true)), "by JellySquid · v1.0.0");
}

#[test]
fn an_empty_folder_lists_nothing() {
    let state = State::default();
    assert!(visible_names(&state).is_empty());
}
