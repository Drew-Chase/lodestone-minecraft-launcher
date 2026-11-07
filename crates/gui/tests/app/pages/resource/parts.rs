//! Tests for `pages/resource/parts.rs`, kept out of the source file. They are compiled as the `tests` child module of
//! `parts`, so they can reach its private helpers.

use super::*;
use crate::app::pages::discover::InstanceChoice;
use crate::app::pages::library::instance::Loader;

/// An instance choice that takes mods, or one that does not when `loader` is vanilla.
fn choice(loader: Loader) -> InstanceChoice {
    InstanceChoice {
        id: "a".to_string(),
        name: "Instance a".to_string(),
        loader,
        mc_version: "1.21.1".to_string(),
    }
}

/// The status of a mod the Mods tab's target instance has at `added_version`, while `adding` is set.
fn status(target: Option<InstanceChoice>, added_version: Option<&str>, adding: bool) -> ProjectStatus {
    ProjectStatus {
        in_library: false,
        downloading: false,
        install: None,
        target,
        instances: Vec::new(),
        target_index: None,
        target_world: None,
        worlds: Vec::new(),
        world_index: None,
        added: added_version.is_some(),
        added_version: added_version.map(str::to_string),
        adding,
    }
}

#[test]
fn a_release_is_installed_switchable_addable_or_blocked_by_what_the_target_has() {
    let fabric = Some(choice(Loader::Fabric));
    assert_eq!(mod_state(&status(fabric.clone(), Some("v1"), false), Some("v1")), ModState::Installed);
    assert_eq!(mod_state(&status(fabric.clone(), Some("v1"), false), Some("v2")), ModState::Switch);
    assert_eq!(mod_state(&status(fabric.clone(), None, false), Some("v2")), ModState::Add);
    assert_eq!(mod_state(&status(fabric.clone(), Some("v1"), true), Some("v2")), ModState::Adding);
    assert_eq!(mod_state(&status(fabric.clone(), Some("v1"), false), None), ModState::Added);
    assert_eq!(
        mod_state(&status(Some(choice(Loader::Vanilla)), None, false), Some("v1")),
        ModState::Unavailable
    );
    assert_eq!(mod_state(&status(None, None, false), Some("v1")), ModState::Unavailable);
}
