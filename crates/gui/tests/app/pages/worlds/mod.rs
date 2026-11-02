//! Tests for `pages/worlds/mod.rs`, kept out of the source file. They are compiled as the `tests` child module of
//! `worlds`, so they can read the page's state directly.

use std::time::Duration;

use chrono::{Local, TimeZone};

use super::*;
use crate::app::pages::instance::files::GameMode;
use crate::app::world_pins::WorldPins;
use filters::{WorldFilter, WorldSort};

fn at(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> DateTime<Local> {
    Local
        .with_ymd_and_hms(year, month, day, hour, minute, 0)
        .single()
        .expect("a valid local time")
}

/// A save named `name` in the instance `instance`, saved on October `day`.
fn entry(instance: &str, name: &str, mode: Option<GameMode>, seed: Option<i64>, size: u64, played_secs: u64, day: u32) -> WorldEntry {
    WorldEntry {
        instance_id: instance.to_owned(),
        instance_name: format!("{instance} instance"),
        mc_version: "1.20.1".to_owned(),
        world: WorldFile {
            path: PathBuf::from(format!("/instances/{instance}/saves/{name}")),
            name: name.to_owned(),
            game_mode: mode,
            difficulty: None,
            seed,
            version: None,
            size_bytes: size,
            saved: Some(at(2026, 10, day, 12, 0)),
            play_time: Some(Duration::from_secs(played_secs)),
            icon: None,
            backdrop: None,
            backed_up: false,
        },
    }
}

/// A page whose scan has landed with `entries`.
fn loaded(entries: Vec<WorldEntry>) -> State {
    let mut state = State::default();
    let generation = state.begin_scan(&[]);
    state.update(Message::Scanned { generation, entries });
    state
}

/// The names of the visible worlds, in order.
fn visible_names(state: &State) -> Vec<&str> {
    state.visible.iter().map(|&index| state.entries[index].world.name.as_str()).collect()
}

fn sample() -> Vec<WorldEntry> {
    vec![
        entry("aether", "Hub", Some(GameMode::Survival), Some(4827), 300, 3600, 20),
        entry("aether", "Spawn", Some(GameMode::Hardcore), None, 100, 60, 28),
        entry("skyfall", "Sky", Some(GameMode::Creative), Some(-99), 900, 7200, 25),
        entry("skyfall", "Mines", Some(GameMode::Survival), Some(1), 500, 36000, 10),
    ]
}

#[test]
fn the_list_shows_the_most_recently_saved_first() {
    let state = loaded(sample());

    assert_eq!(visible_names(&state), ["Spawn", "Sky", "Hub", "Mines"]);
}

#[test]
fn the_chips_count_their_worlds_and_narrow_the_list() {
    let mut state = loaded(sample());
    state.set_pins(WorldPins::from_rows(vec![("skyfall".to_owned(), "Sky".to_owned())]));

    assert_eq!(state.counts, [4, 1, 2, 1, 1], "all, pinned, survival, creative, hardcore");

    state.update(Message::Filter(WorldFilter::Survival));
    assert_eq!(visible_names(&state), ["Hub", "Mines"]);

    state.update(Message::Filter(WorldFilter::Pinned));
    assert_eq!(visible_names(&state), ["Sky"]);

    state.update(Message::Filter(WorldFilter::Hardcore));
    assert_eq!(visible_names(&state), ["Spawn"]);
}

#[test]
fn search_matches_a_name_a_seed_or_an_instance_name() {
    let mut state = loaded(sample());

    state.update(Message::Search("spawn".to_owned()));
    assert_eq!(visible_names(&state), ["Spawn"]);

    state.update(Message::Search("4827".to_owned()));
    assert_eq!(visible_names(&state), ["Hub"], "a seed matches its world");

    state.update(Message::Search("skyfall instance".to_owned()));
    assert_eq!(visible_names(&state), ["Sky", "Mines"], "an instance name matches its worlds");

    state.update(Message::Search(String::new()));
    assert_eq!(state.visible.len(), 4);
}

#[test]
fn the_sorts_order_by_name_size_and_play_time() {
    let mut state = loaded(sample());

    state.update(Message::Sort(WorldSort::Name));
    assert_eq!(visible_names(&state), ["Hub", "Mines", "Sky", "Spawn"]);

    state.update(Message::Sort(WorldSort::Size));
    assert_eq!(visible_names(&state), ["Sky", "Mines", "Hub", "Spawn"]);

    state.update(Message::Sort(WorldSort::PlayTime));
    assert_eq!(visible_names(&state), ["Mines", "Sky", "Hub", "Spawn"]);
}

#[test]
fn a_scan_a_newer_scan_overtook_is_dropped() {
    let mut state = State::default();
    let first = state.begin_scan(&[]);
    let second = state.begin_scan(&[]);

    state.update(Message::Scanned {
        generation: first,
        entries: sample(),
    });
    assert!(!state.loaded, "the first scan answers a page that has moved on");
    assert!(state.entries.is_empty());

    state.update(Message::Scanned {
        generation: second,
        entries: sample(),
    });
    assert!(state.loaded);
    assert_eq!(state.visible.len(), 4);
}

#[test]
fn removing_a_world_leaves_its_namesake_in_another_instance() {
    let mut state = loaded(vec![
        entry("aether", "Hub", None, None, 1, 0, 1),
        entry("skyfall", "Hub", None, None, 1, 0, 2),
    ]);

    state.remove("aether", Path::new("/instances/aether/saves/Hub"));

    assert_eq!(state.entries.len(), 1);
    assert_eq!(state.entries[0].instance_id, "skyfall");
}

#[test]
fn a_world_is_found_by_its_instance_and_its_save() {
    let state = loaded(sample());

    assert!(state.world("skyfall", Path::new("/instances/skyfall/saves/Sky")).is_some());
    assert!(state.world("aether", Path::new("/instances/skyfall/saves/Sky")).is_none());
}

#[test]
fn a_menu_opens_closes_and_closes_on_a_pick() {
    let mut state = loaded(sample());

    state.update(Message::ToggleMenu(Menu::New));
    assert_eq!(state.menu, Some(Menu::New));
    state.update(Message::ToggleMenu(Menu::New));
    assert_eq!(state.menu, None, "the trigger closes its own menu");

    state.update(Message::ToggleMenu(Menu::Sort));
    state.update(Message::DismissMenu);
    assert_eq!(state.menu, None);

    state.update(Message::ToggleMenu(Menu::Import));
    state.update(Message::Pick {
        menu: Menu::Import,
        instance: "aether".to_owned(),
    });
    assert_eq!(state.menu, None);
}

#[test]
fn the_summary_counts_worlds_and_instances() {
    let state = loaded(sample());

    assert!(state.summary().starts_with("4 worlds across 2 instances"), "got: {}", state.summary());
}
