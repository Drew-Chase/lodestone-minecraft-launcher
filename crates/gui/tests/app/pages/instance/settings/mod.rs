//! Tests for `pages/instance/settings/mod.rs`, kept out of the source file. They are compiled as the `tests` child module
//! of `settings`, so they can read the open section and the resolved values directly.

use super::*;
use crate::app::launch_settings::DEFAULT_MEMORY_MB;
use crate::app::pages::library::instance::{Loader, fixture};

fn aether() -> Instance {
    fixture("aether", "Aether Legacy", Loader::Fabric)
}

/// A page state that has been synced with `instance` and the built-in defaults.
fn synced(instance: &Instance) -> State {
    let mut state = State::default();
    state.sync(instance, &LaunchDefaults::default());
    state
}

#[test]
fn sections_follow_the_sub_nav_order() {
    let labels: Vec<_> = Section::ALL.iter().map(|section| section.label()).collect();
    assert_eq!(
        labels,
        [
            "General",
            "Java & Memory",
            "Window",
            "Launch Args",
            "Game Version",
            "Paths & Files",
            "Advanced",
            "Danger Zone"
        ]
    );
}

#[test]
fn every_section_has_its_own_anchor() {
    let anchors: std::collections::HashSet<_> = Section::ALL.iter().map(|section| section.anchor_id()).collect();
    assert_eq!(anchors.len(), Section::ALL.len());
}

#[test]
fn the_rows_show_the_global_defaults_until_the_instance_overrides_them() {
    let instance = aether();
    let mut state = synced(&instance);
    assert_eq!(state.resolved.as_ref().map(|values| values.memory_mb), Some(DEFAULT_MEMORY_MB));

    let defaults = LaunchDefaults {
        memory_mb: 6144,
        ..LaunchDefaults::default()
    };
    state.sync(&instance, &defaults);
    assert_eq!(state.resolved.as_ref().map(|values| values.memory_mb), Some(6144));
}

#[test]
fn an_override_shows_its_own_value_beside_the_defaults() {
    let mut instance = aether();
    instance.settings.override_with(LaunchChange::Memory(8192));
    let state = synced(&instance);
    assert_eq!(state.resolved.as_ref().map(|values| values.memory_mb), Some(8192));
    assert_eq!(state.overrides.as_ref().map(InstanceSettings::inherited_count), Some(12));
}

#[test]
fn pressing_a_sidebar_entry_selects_its_section() {
    let instance = aether();
    let mut state = synced(&instance);
    let _ = state.update(Message::Section(Section::DangerZone), &instance);
    assert_eq!(state.section, Section::DangerZone);
}

#[test]
fn measured_offsets_move_the_sidebar_to_the_section_at_the_top() {
    let instance = aether();
    let mut state = synced(&instance);
    let offsets = vec![(Section::General, 0.0), (Section::Window, 300.0), (Section::Advanced, 900.0)];
    let _ = state.update(Message::Measured(offsets), &instance);
    let _ = state.update(
        Message::Scrolled {
            offset: 310.0,
            at_end: false,
        },
        &instance,
    );
    let _ = state.update(Message::Measured(vec![(Section::General, 0.0), (Section::Window, 300.0)]), &instance);
    assert_eq!(state.section, Section::Window);
}

#[test]
fn a_typed_resolution_keeps_the_text_until_both_sides_are_valid() {
    let instance = aether();
    let mut state = synced(&instance);
    let _ = state.update(Message::Resolution(Axis::Width, "10".to_string()), &instance);
    assert_eq!(state.draft.width.as_deref(), Some("10"), "a width below 320 is kept as typed");
    let _ = state.update(Message::Resolution(Axis::Width, "2x".to_string()), &instance);
    assert_eq!(state.draft.width.as_deref(), Some("2"), "letters are dropped");
}

#[test]
fn a_valid_pair_clears_the_drafts() {
    let instance = aether();
    let mut state = synced(&instance);
    let _ = state.update(Message::Resolution(Axis::Width, "2560".to_string()), &instance);
    assert_eq!(state.draft.width, None, "the pair is valid with the saved height");
}

#[test]
fn the_environment_editor_opens_and_closes() {
    let instance = aether();
    let mut state = synced(&instance);
    let _ = state.update(Message::OpenEnvironment, &instance);
    assert!(state.environment_open);
    let _ = state.update(Message::CloseEnvironment, &instance);
    assert!(!state.environment_open);
}

#[test]
fn an_instance_picture_is_read_into_the_editor_when_it_has_one() {
    let mut instance = aether();
    instance.icon = InstanceIcon::Custom(std::path::PathBuf::from("/instances/aether/.lodestone/icon-1.png"));
    let mut state = synced(&instance);
    let _ = state.update(Message::OpenIconEditor, &instance);
    assert!(state.icon.is_some());
}

#[test]
fn the_scene_of_a_banner_is_found_from_its_biome_and_seed() {
    for index in 0..SCENE_COUNT {
        let (biome, seed) = banner::scene(index);
        assert_eq!(scene_index(biome, seed), index);
    }
}
