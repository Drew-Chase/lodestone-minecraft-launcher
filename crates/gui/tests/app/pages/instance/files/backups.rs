//! Tests for `pages/instance/files/backups.rs`, kept out of the source file. They are compiled as the `tests` child
//! module of `backups`.

use std::fs;
use std::path::{Path, PathBuf};

use chrono::{Local, TimeZone};

use super::*;

/// A fresh folder under the system temp directory, named for the test.
fn scratch(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("lodestone-backups-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).expect("scratch folder");
    path
}

/// Writes `bytes` to `path`, making its folders first.
fn write(path: &Path, bytes: &[u8]) {
    fs::create_dir_all(path.parent().expect("a parent folder")).expect("parent folder");
    fs::write(path, bytes).expect("file writes");
}

fn at(year: i32, month: u32, day: u32, hour: u32, minute: u32, second: u32) -> DateTime<Local> {
    Local
        .with_ymd_and_hms(year, month, day, hour, minute, second)
        .single()
        .expect("a valid local time")
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
fn a_name_gives_its_save_folder_and_the_time_it_was_made() {
    let (folder, taken) = parse("My-World-2026-11-02_13.45.00.zip").expect("a backup name");
    assert_eq!(folder, "My-World", "the folder keeps its own dashes");
    assert_eq!(taken, at(2026, 11, 2, 13, 45, 0));
    assert!(parse("Hub-2026-13-02_13.45.00.zip").is_none(), "a month that does not exist");
    assert!(parse("2026-11-02_13.45.00.zip").is_none(), "no save folder");
}

#[test]
fn a_folder_lists_its_backups_newest_first_and_skips_the_rest() {
    let root = scratch("scan");
    let backups = root.join("backups");
    write(&backups.join("Hub-2026-11-01_09.00.00.zip"), b"older");
    write(&backups.join("Hub-2026-11-02_13.45.00.zip"), b"newer!");
    write(&backups.join("Hub.zip"), b"not a backup");
    fs::create_dir_all(backups.join("Spawn-2026-11-03_08.00.00.zip")).expect("a folder named like a backup");

    let listed = scan(&backups);
    let sizes: Vec<(&str, u64)> = listed.iter().map(|backup| (backup.folder.as_str(), backup.size_bytes)).collect();
    assert_eq!(sizes, [("Hub", 6), ("Hub", 5)]);
    assert_eq!(listed[0].taken, at(2026, 11, 2, 13, 45, 0));
    assert!(listed[0].path.ends_with("Hub-2026-11-02_13.45.00.zip"));
    fs::remove_dir_all(&root).ok();
}

#[test]
fn a_missing_backups_folder_lists_nothing() {
    let root = scratch("missing");
    assert!(scan(&root.join("backups")).is_empty());
    fs::remove_dir_all(&root).ok();
}

#[test]
fn a_folder_name_is_one_ordinary_path_component() {
    assert!(is_folder_name("Hub"));
    assert!(is_folder_name("My World (2)"));
    assert!(!is_folder_name(""));
    assert!(!is_folder_name("."));
    assert!(!is_folder_name(".."));
    assert!(!is_folder_name("a/b"));
    assert!(parse("..-2026-11-02_13.45.00.zip").is_none(), "a parent folder is no save");
}
