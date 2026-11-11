//! Tests for `pages/instance/worlds.rs`, kept out of the source file. They are
//! compiled as the `tests` child module of `worlds`, so they can read the
//! visible indices directly.

use chrono::{Local, TimeZone};

use super::*;
use crate::app::pages::instance::files::{Difficulty, GameMode, ServerFile};
use crate::app::shared_library::SharedEntry;

fn at(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> DateTime<Local> {
    Local
        .with_ymd_and_hms(year, month, day, hour, minute, 0)
        .single()
        .expect("a valid local time")
}

fn world(name: &str) -> WorldFile {
    WorldFile {
        path: PathBuf::from(format!("/saves/{name}")),
        name: name.to_string(),
        game_mode: Some(GameMode::Survival),
        difficulty: Some(Difficulty::Normal),
        seed: Some(1),
        version: Some("1.20.1".to_string()),
        size_bytes: 1,
        saved: Some(at(2026, 10, 29, 12, 0)),
        play_time: Some(std::time::Duration::from_secs(48 * 3600 + 12 * 60)),
        icon: None,
        backdrop: None,
        backed_up: false,
    }
}

fn server(name: &str, address: &str) -> ServerFile {
    ServerFile {
        name: name.to_owned(),
        address: address.to_owned(),
        icon: None,
    }
}

#[test]
fn the_search_keeps_the_servers_whose_name_or_address_contains_it() {
    let mut state = State::default();
    state.set_servers(vec![server("Ad Astra", "play.astra.net"), server("Creative", "192.168.1.42:25565")]);
    assert_eq!(state.visible_servers, [0, 1]);

    state.update(Message::Search("ASTRA".to_string()));
    assert_eq!(state.visible_servers, [0]);

    state.update(Message::Search("192.168".to_string()));
    assert_eq!(state.visible_servers, [1]);
}

#[test]
fn the_worlds_and_the_servers_share_one_search() {
    let mut state = State::default();
    state.set_worlds(vec![world("Aether Hub")]);
    state.set_servers(vec![server("Aether Server", "a.example")]);

    state.update(Message::Search("aether".to_string()));
    assert_eq!(state.visible, [0]);
    assert_eq!(state.visible_servers, [0]);

    state.update(Message::Search("pixel".to_string()));
    assert!(state.visible.is_empty());
    assert!(state.visible_servers.is_empty());
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
    state.set_worlds(vec![world("Aether Hub"), world("Aether Copy"), world("Pixel Town")]);
    assert_eq!(state.visible, [0, 1, 2]);

    state.update(Message::Search("AETHER".to_string()));
    assert_eq!(state.visible, [0, 1]);

    state.update(Message::Search("no world has this name".to_string()));
    assert!(state.visible.is_empty());
}

#[test]
fn search_matches_seeds_too() {
    let mut state = State::default();
    let mut seeded = world("Pixel Town");
    seeded.seed = Some(-4827);
    state.set_worlds(vec![world("Aether Hub"), seeded]);

    state.update(Message::Search("4827".to_string()));
    assert_eq!(state.visible, [1]);
}

#[test]
fn the_root_carries_the_intents_so_the_grid_is_left_alone() {
    let mut state = State::default();
    state.set_worlds(vec![world("Aether Hub")]);
    state.update(Message::Import);
    state.update(Message::NewWorld);
    state.update(Message::Menu {
        path: PathBuf::from("/saves/Aether Hub"),
        action: Some(Action::Play),
    });
    state.update(Message::Menu {
        path: PathBuf::from("/saves/Aether Hub"),
        action: Some(Action::Delete),
    });
    assert_eq!(state.visible, [0]);
}

#[test]
fn a_removed_world_leaves_the_list_and_the_search() {
    let mut state = State::default();
    state.set_worlds(vec![world("Aether Hub"), world("Pixel Town")]);
    state.update(Message::Search("pixel".to_string()));
    state.remove(Path::new("/saves/Pixel Town"));
    assert!(state.visible.is_empty());
    assert_eq!(state.worlds.len(), 1);

    state.update(Message::Search(String::new()));
    assert_eq!(state.visible, [0]);
}

#[test]
fn a_hover_is_set_and_cleared_by_its_own_card() {
    let mut state = State::default();
    let hub = PathBuf::from("/saves/Aether Hub");
    state.update(Message::Hover {
        path: hub.clone(),
        hovered: true,
    });
    assert_eq!(state.hovered.as_deref(), Some(hub.as_path()));

    // An exit for another card leaves this one hovered: enter and exit can arrive in any order.
    state.update(Message::Hover {
        path: PathBuf::from("/saves/Other"),
        hovered: false,
    });
    assert_eq!(state.hovered.as_deref(), Some(hub.as_path()));

    state.update(Message::Hover { path: hub, hovered: false });
    assert!(state.hovered.is_none());
}

#[test]
fn other_messages_keep_the_hover_while_the_pointer_stays_over_the_card() {
    let mut state = State::default();
    state.update(Message::Hover {
        path: PathBuf::from("/saves/Aether Hub"),
        hovered: true,
    });

    state.update(Message::Search(String::new()));
    assert!(state.hovered.is_some(), "the card is still under the pointer");
}

fn shared_world(name: &str) -> SharedEntry {
    SharedEntry {
        name: name.to_string(),
        path: PathBuf::from(format!("/shared/worlds/download/{name}")),
        is_dir: false,
        download: Some("download".to_string()),
    }
}

#[test]
fn the_shared_worlds_open_a_dialog_until_it_is_closed() {
    let mut state = State::default();
    assert!(state.dialog().is_none(), "nothing opens until the root has read the shared folder");

    state.update(Message::SharedLoaded {
        entries: vec![(shared_world("Aether Hub.zip"), false), (shared_world("Pixel Town.zip"), true)],
    });
    let dialog = state.dialog().expect("the dialog is open");
    assert_eq!(dialog.entries.len(), 2);
    assert!(dialog.entries[1].1, "a world the instance has is marked");

    state.update(Message::CloseShared);
    assert!(state.dialog().is_none());
}

#[test]
fn adding_or_deleting_a_shared_world_leaves_the_list_to_the_root() {
    let mut state = State::default();
    state.update(Message::SharedLoaded {
        entries: vec![(shared_world("Aether Hub.zip"), false)],
    });
    state.update(Message::AddShared { index: 0 });
    state.update(Message::DeleteShared { index: 0 });
    assert_eq!(state.dialog().map(|dialog| dialog.entries.len()), Some(1));
}
