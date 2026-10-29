//! Tests for `pages/instance/demo.rs`, kept out of the source file.

use chrono::{Local, TimeZone};

use super::*;

fn now() -> DateTime<Local> {
    Local.with_ymd_and_hms(2026, 10, 29, 12, 0, 0).single().expect("a valid local time")
}

fn aether() -> Instance {
    Instance::seed_pack(now()).remove(0)
}

#[test]
fn reference_mods_match_the_mods_reference() {
    let mods = mods(&aether());
    assert_eq!(mods.len(), 87, "the header reports 87 mods");
    let names: Vec<_> = mods.iter().take(8).map(|entry| entry.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "Sodium",
            "Iris Shaders",
            "Lithium",
            "Fabric API",
            "Xaero's Minimap",
            "JEI",
            "Mod Menu",
            "Continuity"
        ]
    );
    assert!(!mods[6].enabled, "Mod Menu is switched off in the reference");
}

#[test]
fn every_seeded_mod_list_matches_its_mod_count() {
    for instance in Instance::seed_pack(now()) {
        assert_eq!(mods(&instance).len(), instance.mod_count as usize, "{}", instance.name);
    }
}

#[test]
fn reference_worlds_match_the_worlds_reference() {
    let worlds = worlds(&aether(), now());
    let names: Vec<_> = worlds.iter().map(|world| world.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "Aether Hub",
            "Creative Sandbox",
            "SkyBlock Classic",
            "Nether Outpost",
            "Pixel Town",
            "Desert Expedition"
        ]
    );
    assert_eq!(worlds.iter().filter(|world| world.pinned).count(), 2);
    assert!(worlds.iter().all(|world| !world.backup), "the reference shows no backups");
}

#[test]
fn reference_screenshots_are_nine_captures_of_2_1_mb() {
    let shots = screenshots(&aether());
    assert_eq!(shots.len(), 9);
    assert_eq!(shots[0].file_name, "2026-04-10_18.00.png");
    assert_eq!(shots[8].file_name, "2026-04-18_18.08.png");
    assert!(shots.iter().all(|shot| shot.size_label == "2.1 MB"));
}

#[test]
fn reference_log_has_nine_lines_and_one_warning() {
    let lines = log_lines(&aether());
    assert_eq!(lines.len(), 9);
    assert_eq!(lines.iter().filter(|line| line.level == LogLevel::Warn).count(), 1);
    assert_eq!(lines.last().map(|line| line.message.as_str()), Some("Awaiting player input"));
}

#[test]
fn reference_settings_match_the_settings_reference() {
    let settings = settings(&aether());
    assert_eq!(settings.memory_gb, 6);
    assert_eq!(settings.environment_variables, 3);
    assert_eq!(settings.loader_versions[0], "0.14.21");
    assert_eq!(MINECRAFT_VERSIONS[settings.minecraft_version], "1.20.1");
}

#[test]
fn generated_data_is_deterministic_for_each_instance() {
    let now = now();
    for instance in Instance::seed_pack(now).iter().skip(1).take(8) {
        assert_eq!(mods(instance), mods(instance));
        assert_eq!(worlds(instance, now), worlds(instance, now));
        assert_eq!(screenshots(instance), screenshots(instance));
        assert_eq!(log_lines(instance), log_lines(instance));
        assert_eq!(settings(instance), settings(instance));
    }
}

#[test]
fn vanilla_instances_have_no_mods_and_a_placeholder_loader_version() {
    let vanilla = Instance::seed_pack(now())
        .into_iter()
        .find(|instance| instance.loader == Loader::Vanilla)
        .expect("the seed pack includes a vanilla instance");
    assert!(mods(&vanilla).is_empty());
    assert_eq!(settings(&vanilla).loader_versions, ["—"]);
}

#[test]
fn a_seed_that_reuses_the_reference_name_keeps_its_own_data() {
    let renamed = Instance::seed_pack(now())
        .into_iter()
        .skip(1)
        .find(|instance| instance.name == "Aether Legacy")
        .expect("the seed pack repeats the reference name");
    assert_eq!(mods(&renamed).len(), renamed.mod_count as usize);
    assert!(worlds(&renamed, now()).iter().all(|world| world.name != "Aether Hub"));
}
