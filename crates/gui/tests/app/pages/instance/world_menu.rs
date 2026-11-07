//! Tests for `pages/instance/world_menu.rs`, kept out of the source file. They are compiled as the `tests`
//! child module of `world_menu`.

use super::*;

#[test]
fn copy_seed_is_listed_only_for_a_world_with_a_seed() {
    let with_seed = menu(true, false).actions;
    let without_seed = menu(false, false).actions;

    assert!(with_seed.contains(&Some(Action::CopySeed)));
    assert!(!without_seed.contains(&Some(Action::CopySeed)));
    assert_eq!(with_seed.len(), without_seed.len() + 1);
    assert_eq!(
        without_seed,
        [
            Some(Action::Play),
            None,
            Some(Action::Reveal),
            Some(Action::Pin),
            Some(Action::Datapacks),
            Some(Action::BackUp),
            None,
            Some(Action::Delete),
        ]
    );
}

#[test]
fn the_pin_row_offers_the_change_the_world_needs() {
    let unpinned = menu(true, false).actions;
    let pinned = menu(true, true).actions;

    assert!(unpinned.contains(&Some(Action::Pin)));
    assert!(!unpinned.contains(&Some(Action::Unpin)));
    assert!(pinned.contains(&Some(Action::Unpin)));
    assert!(!pinned.contains(&Some(Action::Pin)));
}
