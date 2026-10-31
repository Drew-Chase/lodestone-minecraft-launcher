use super::*;
use crate::app::pages::library::instance::fixture;

#[test]
fn whole_hours_is_a_bare_count() {
    assert_eq!(whole_hours(24 * 3600 + 12 * 60), "24");
    assert_eq!(whole_hours(59 * 60), "0");
}

/// The compact line adds its own hour unit to a bare count, so the strings built here must not print it twice.
#[test]
fn playtime_prints_the_hour_unit_once() {
    let mut instance = fixture("aether", "Aether Legacy", Loader::Fabric);
    instance.playtime_seconds = 24 * 3600 + 12 * 60;
    instance.mod_count = 87;
    assert!(
        !playtime_hours(instance.playtime_seconds).contains("hh"),
        "{}",
        playtime_hours(instance.playtime_seconds)
    );
    assert!(!compact_stats(&instance).contains("hh"), "{}", compact_stats(&instance));
}
