//! Tests for `pages/instance/mods.rs`, kept out of the source file. They are
//! compiled as the `tests` child module of `mods`, so they can read the rows
//! and the visible indices directly.

use std::collections::HashSet;

use chrono::Local;

use super::*;

fn entry(name: &str, author: &str, category: ModCategory, enabled: bool) -> ModEntry {
    ModEntry {
        name: name.to_string(),
        author: author.to_string(),
        version: "1.0.0".to_string(),
        category,
        enabled,
    }
}

fn sample() -> State {
    let mut state = State {
        entries: vec![
            entry("Sodium", "JellySquid", ModCategory::Performance, true),
            entry("Iris Shaders", "coderbot", ModCategory::Graphics, true),
            entry("Lithium", "CaffeineMC", ModCategory::Performance, false),
        ],
        ..State::default()
    };
    state.refilter();
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
fn the_category_filter_narrows_the_list() {
    let mut state = sample();
    state.update(Message::Category(Some(ModCategory::Performance)));
    assert_eq!(visible_names(&state), ["Sodium", "Lithium"]);
    state.update(Message::Category(None));
    assert_eq!(visible_names(&state), ["Sodium", "Iris Shaders", "Lithium"]);
}

#[test]
fn text_and_category_combine() {
    let mut state = sample();
    state.update(Message::Category(Some(ModCategory::Performance)));
    state.update(Message::Query("li".to_string()));
    assert_eq!(visible_names(&state), ["Lithium"]);
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
fn categories_have_distinct_titles_and_tags() {
    let titles: HashSet<_> = ModCategory::ALL.iter().map(|category| category.title()).collect();
    let tags: HashSet<_> = ModCategory::ALL.iter().map(|category| category.label()).collect();
    assert_eq!(titles.len(), ModCategory::ALL.len());
    assert_eq!(tags.len(), ModCategory::ALL.len());
    assert_eq!(ModCategory::Hud.label(), "HUD");
}

#[test]
fn a_new_instance_starts_with_every_seeded_mod_visible() {
    let instance = Instance::seed_pack(Local::now()).remove(0);
    let state = State::for_instance(&instance);
    assert_eq!(state.visible.len(), 87);
}
