//! Tests for `pages/instance/settings/mod.rs`, kept out of the source file. They
//! are compiled as the `tests` child module of `settings`, so they can read the
//! open section and the values directly.

use chrono::Local;

use super::*;

fn aether() -> Instance {
    Instance::seed_pack(Local::now()).remove(0)
}

#[test]
fn memory_readings_clamp_to_the_slider_range() {
    assert_eq!(clamp_memory(1.2), 2);
    assert_eq!(clamp_memory(5.4), 5);
    assert_eq!(clamp_memory(5.6), 6);
    assert_eq!(clamp_memory(40.0), 16);
}

#[test]
fn resolution_sides_must_be_whole_numbers_in_range() {
    assert_eq!(parse_dimension("1920"), Some(1920));
    assert_eq!(parse_dimension("100"), None, "below the 320px floor");
    assert_eq!(parse_dimension("99999"), None, "above the 7680px ceiling");
    assert_eq!(parse_dimension(""), None);
}

#[test]
fn resolution_input_keeps_digits_and_caps_the_length() {
    assert_eq!(sanitize_dimension("19a20"), "1920");
    assert_eq!(sanitize_dimension("123456789"), "12345");
}

#[test]
fn sections_follow_the_sub_nav_order() {
    let labels: Vec<_> = Section::ALL.iter().map(|section| section.label()).collect();
    assert_eq!(
        labels,
        [
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
fn switches_and_choices_change_only_their_own_field() {
    let mut state = State::for_instance(&aether());
    let before = state.settings.clone();

    state.update(Message::Switch(Switch::Snapshots, true));
    assert!(state.settings.snapshots);

    state.update(Message::Choose(Choice::GarbageCollector, 1));
    assert_eq!(state.settings.garbage_collector, 1);

    assert_eq!(state.settings.java_runtime, before.java_runtime);
    assert_eq!(state.settings.memory_gb, before.memory_gb);
}

#[test]
fn the_memory_slider_stores_a_clamped_whole_gigabyte_value() {
    let mut state = State::default();
    state.update(Message::Memory(12.4));
    assert_eq!(state.settings.memory_gb, 12);
    state.update(Message::Memory(0.0));
    assert_eq!(state.settings.memory_gb, 2);
}

#[test]
fn resolution_messages_store_sanitised_digits() {
    let mut state = State::default();
    state.update(Message::Dimension(Axis::Width, "2560x".to_string()));
    assert_eq!(state.settings.width, "2560");
}

#[test]
fn opening_a_section_changes_only_the_section() {
    let mut state = State::default();
    state.update(Message::Section(Section::DangerZone));
    assert_eq!(state.section, Section::DangerZone);
    assert_eq!(state.settings, Settings::default());
}
