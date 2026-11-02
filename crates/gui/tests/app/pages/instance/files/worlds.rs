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
    /// Epoch milliseconds, written only when a test sets it.
    #[serde(skip_serializing_if = "Option::is_none")]
    last_played: Option<i64>,
    /// Ticks the world has existed, written only when a test sets it.
    #[serde(skip_serializing_if = "Option::is_none")]
    time: Option<i64>,
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
        last_played: None,
        time: None,
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

#[test]
fn last_played_and_time_give_the_save_and_play_times() {
    let root = scratch("played");
    let data = TestData {
        last_played: Some(1_782_000_000_000),
        time: Some((48 * 3600 + 12 * 60) * 20),
        ..hub()
    };
    write(&root.join("Hub/level.dat"), &level_dat(data));

    let world = &scan(&root)[0];
    assert_eq!(world.path, root.join("Hub"));
    let saved = DateTime::<Utc>::from_timestamp_millis(1_782_000_000_000)
        .expect("a valid time")
        .with_timezone(&Local);
    assert_eq!(world.saved, Some(saved), "LastPlayed is the save time");
    assert_eq!(
        world.play_time,
        Some(Duration::from_secs(48 * 3600 + 12 * 60)),
        "Time counts 20 ticks to the second"
    );
    fs::remove_dir_all(&root).ok();
}

#[test]
fn without_last_played_the_file_write_time_stands_in() {
    let root = scratch("written");
    write(&root.join("Hub/level.dat"), &level_dat(hub()));

    let world = &scan(&root)[0];
    assert!(world.play_time.is_none(), "no Time, no play time");
    assert!(
        world.saved.is_some_and(|saved| saved.timestamp() > 1_000_000_000),
        "the write time is recent"
    );
    fs::remove_dir_all(&root).ok();
}

#[test]
fn a_zero_last_played_is_not_a_save_time() {
    let root = scratch("zero");
    let data = TestData {
        last_played: Some(0),
        ..hub()
    };
    write(&root.join("Hub/level.dat"), &level_dat(data));

    let world = &scan(&root)[0];
    assert!(
        world.saved.is_some_and(|saved| saved.timestamp() > 1_000_000_000),
        "the write time stands in for 1970"
    );
    fs::remove_dir_all(&root).ok();
}

#[test]
fn a_backup_is_named_for_its_save_and_a_timestamp() {
    assert!(is_backup_of("Hub-2026-11-02_13.45.00.zip", "Hub"));
    assert!(!is_backup_of("Hub-2020.zip", "Hub"), "a name that is not the timestamp shape");
    assert!(
        !is_backup_of("HubTown-2026-11-02_13.45.00.zip", "Hub"),
        "another world with a longer name"
    );
    assert!(!is_backup_of("Hub-2026-11-02_13.45.00-extra.zip", "Hub"), "a stamp with more on it");
    assert!(!is_backup_of("Hub-2026-11-02_13.45.00.tar", "Hub"), "not a zip");
}

#[test]
fn a_save_is_backed_up_when_its_instance_holds_a_zip_of_it() {
    let root = scratch("backup");
    let instance = root.join("aether");
    write(&instance.join("saves/Hub/level.dat"), &level_dat(hub()));
    write(&instance.join("saves/Other/level.dat"), &level_dat(hub()));
    write(&instance.join("backups/Hub-2026-11-02_13.45.00.zip"), b"zip");

    assert!(read_world(&instance.join("saves/Hub")).is_some_and(|world| world.backed_up));
    assert!(read_world(&instance.join("saves/Other")).is_some_and(|world| !world.backed_up));
    fs::remove_dir_all(&root).ok();
}

#[test]
fn a_save_with_an_icon_has_the_icon_and_a_backdrop() {
    let root = scratch("icon");
    write(&root.join("Hub/level.dat"), &level_dat(hub()));
    let icon = image::RgbaImage::from_fn(64, 64, |x, _| {
        if x < 32 {
            image::Rgba([255, 0, 0, 255])
        } else {
            image::Rgba([0, 0, 255, 255])
        }
    });
    icon.save(root.join("Hub/icon.png")).expect("the icon writes");

    let world = read_world(&root.join("Hub")).expect("the save lists");
    assert!(world.icon.is_some(), "the icon is decoded");
    assert!(world.backdrop.is_some(), "the blurred backdrop is made beside it");
    fs::remove_dir_all(&root).ok();
}
