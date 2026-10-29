//! Tests for `pages/instance/files/worlds.rs`, kept out of the source file. They
//! are compiled as the `tests` child module of `worlds`.

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use flate2::Compression;
use flate2::write::GzEncoder;
use serde::Serialize;

use super::*;

/// A fresh folder under the system temp directory, named for the test.
fn scratch(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("lodestone-worlds-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).expect("scratch folder");
    path
}

/// Writes `contents` to `path`, creating the folders above it.
fn write(path: &Path, contents: &[u8]) {
    fs::create_dir_all(path.parent().expect("a file has a parent")).expect("parent folder");
    fs::write(path, contents).expect("file writes");
}

/// The `level.dat` tree, laid out the way the game writes it.
#[derive(Serialize)]
struct TestLevel {
    #[serde(rename = "Data")]
    data: TestData,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct TestData {
    level_name: String,
    game_type: i32,
    /// Written in lowercase, as the game does.
    #[serde(rename = "hardcore")]
    hardcore: i8,
    difficulty: i8,
    version: TestVersion,
    world_gen_settings: TestSeed,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct TestVersion {
    name: String,
}

#[derive(Serialize)]
struct TestSeed {
    seed: i64,
}

/// A hardcore world on difficulty 3 with a negative seed.
fn hub() -> TestData {
    TestData {
        level_name: "Aether Hub".to_string(),
        game_type: 0,
        hardcore: 1,
        difficulty: 3,
        version: TestVersion { name: "1.20.1".to_string() },
        world_gen_settings: TestSeed { seed: -4_827_193_047_281 },
    }
}

/// `level.dat` bytes: NBT, gzip-compressed as the game stores them.
fn level_dat(data: TestData) -> Vec<u8> {
    let nbt = fastnbt::to_bytes(&TestLevel { data }).expect("nbt encodes");
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&nbt).expect("gzip writes");
    encoder.finish().expect("gzip finishes")
}

#[test]
fn a_folder_without_a_level_dat_is_not_a_save() {
    let root = scratch("marker");
    fs::create_dir_all(root.join("Empty")).expect("a folder with nothing in it");
    write(&root.join("Hub/level.dat"), &level_dat(hub()));

    let names: Vec<String> = scan(&root).into_iter().map(|world| world.name).collect();
    assert_eq!(names, ["Aether Hub"]);
    fs::remove_dir_all(&root).ok();
}

#[test]
fn level_dat_supplies_the_name_mode_difficulty_seed_and_version() {
    let root = scratch("level");
    write(&root.join("Hub/level.dat"), &level_dat(hub()));

    let worlds = scan(&root);
    assert_eq!(worlds.len(), 1);
    let world = &worlds[0];
    assert_eq!(world.name, "Aether Hub");
    assert_eq!(world.game_mode, Some(GameMode::Hardcore));
    assert_eq!(world.difficulty, Some(Difficulty::Hard));
    assert_eq!(world.seed, Some(-4_827_193_047_281));
    assert_eq!(world.version.as_deref(), Some("1.20.1"));
    assert!(world.saved.is_some(), "the level.dat's write time is the save time");
    assert!(world.size_bytes > 0, "the folder's size is read");
    assert!(world.icon.is_none(), "a save without icon.png has no icon");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn game_modes_and_difficulties_map_from_the_nbt_codes() {
    let root = scratch("codes");
    let cases = [
        ("Creative", 1, 0, 0, Some(GameMode::Creative), Some(Difficulty::Peaceful)),
        ("Adventure", 2, 0, 1, Some(GameMode::Adventure), Some(Difficulty::Easy)),
        ("Spectator", 3, 0, 2, None, Some(Difficulty::Normal)),
    ];
    for (name, game_type, hardcore, difficulty, _, _) in cases {
        let data = TestData {
            level_name: name.to_string(),
            game_type,
            hardcore,
            difficulty,
            ..hub()
        };
        write(&root.join(name).join("level.dat"), &level_dat(data));
    }

    for (name, _, _, _, mode, difficulty) in cases {
        let world = scan(&root).into_iter().find(|world| world.name == name).expect("the save is listed");
        assert_eq!(world.game_mode, mode, "{name}");
        assert_eq!(world.difficulty, difficulty, "{name}");
    }
    fs::remove_dir_all(&root).ok();
}

#[test]
fn an_unreadable_level_dat_still_lists_the_save_under_its_folder_name() {
    let root = scratch("broken");
    write(&root.join("Broken/level.dat"), b"not gzip");

    let worlds = scan(&root);
    assert_eq!(worlds.len(), 1);
    assert_eq!(worlds[0].name, "Broken");
    assert!(worlds[0].game_mode.is_none() && worlds[0].seed.is_none() && worlds[0].version.is_none());
    fs::remove_dir_all(&root).ok();
}

#[test]
fn a_blank_level_name_falls_back_to_the_folder_name() {
    let root = scratch("blank");
    let data = TestData {
        level_name: "  ".to_string(),
        ..hub()
    };
    write(&root.join("Blank/level.dat"), &level_dat(data));

    assert_eq!(scan(&root)[0].name, "Blank");
    fs::remove_dir_all(&root).ok();
}
