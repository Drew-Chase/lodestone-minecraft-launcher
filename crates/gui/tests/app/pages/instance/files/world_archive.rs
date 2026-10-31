//! Tests for `pages/instance/files/world_archive.rs`, kept out of the source file. They are compiled as the
//! `tests` child module of `world_archive`.

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use chrono::{Local, TimeZone};
use zip::write::SimpleFileOptions;

use super::*;

/// A fresh folder under the system temp directory, named for the test.
fn scratch(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("lodestone-archive-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).expect("scratch folder");
    path
}

/// Writes `contents` to `path`, creating the folders above it.
fn write(path: &Path, contents: &[u8]) {
    fs::create_dir_all(path.parent().expect("a file has a parent")).expect("parent folder");
    fs::write(path, contents).expect("file writes");
}

/// A save on disk: its `level.dat`, a region file, and the lock a running game holds on it.
fn save_folder(root: &Path, name: &str) -> PathBuf {
    let folder = root.join(name);
    write(&folder.join("level.dat"), b"level");
    write(&folder.join("region/r.0.0.mca"), b"region");
    write(&folder.join("session.lock"), b"lock");
    folder
}

/// A zip at `path` holding `entries`, each a name and its contents.
fn zip_of(path: &Path, entries: &[(&str, &[u8])]) {
    let mut writer = zip::ZipWriter::new(fs::File::create(path).expect("zip file creates"));
    for (name, contents) in entries {
        writer.start_file(*name, SimpleFileOptions::default()).expect("entry starts");
        writer.write_all(contents).expect("entry writes");
    }
    writer.finish().expect("zip finishes");
}

/// The names of the entries in the zip at `path`, sorted.
fn entry_names(path: &Path) -> Vec<String> {
    let archive = zip::ZipArchive::new(fs::File::open(path).expect("zip opens")).expect("zip reads");
    let mut names: Vec<String> = archive
        .file_names()
        .map(|name| name.expect("the entry name reads").into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn a_backup_zips_the_save_under_its_folder_name_without_the_session_lock() {
    let root = scratch("backup");
    let world = save_folder(&root, "Hub");
    let now = Local.with_ymd_and_hms(2026, 10, 30, 9, 5, 7).single().expect("a valid time");

    let archive = back_up(&world, &root.join("backups"), now).expect("the backup writes");
    assert_eq!(archive.file_name().and_then(|name| name.to_str()), Some("Hub-2026-10-30_09.05.07.zip"));
    assert_eq!(entry_names(&archive), ["Hub/level.dat", "Hub/region/r.0.0.mca"]);
    fs::remove_dir_all(&root).ok();
}

#[test]
fn a_zip_holding_the_save_at_its_root_imports_under_the_archive_name() {
    let root = scratch("import-root");
    let saves = root.join("saves");
    fs::create_dir_all(&saves).expect("saves folder");
    let archive = root.join("Hub.zip");
    zip_of(&archive, &[("level.dat", b"level"), ("region/r.0.0.mca", b"region")]);

    assert_eq!(import(&archive, &saves).expect("the import extracts"), "Hub");
    assert_eq!(fs::read(saves.join("Hub/level.dat")).expect("level.dat"), b"level");
    assert_eq!(fs::read(saves.join("Hub/region/r.0.0.mca")).expect("region"), b"region");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn a_top_folder_is_stripped_and_a_taken_name_gets_a_number() {
    let root = scratch("import-folder");
    let saves = root.join("saves");
    write(&saves.join("Hub/level.dat"), b"existing");
    let archive = root.join("Hub.zip");
    zip_of(&archive, &[("Hub/level.dat", b"level"), ("Hub/data/x.dat", b"x")]);

    assert_eq!(import(&archive, &saves).expect("the import extracts"), "Hub (2)");
    assert_eq!(fs::read(saves.join("Hub (2)/level.dat")).expect("level.dat"), b"level");
    assert!(saves.join("Hub (2)/data/x.dat").is_file(), "the folders inside the save come across");
    assert_eq!(fs::read(saves.join("Hub/level.dat")).expect("the save already there"), b"existing");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn an_archive_without_a_level_dat_is_refused_and_leaves_nothing_behind() {
    let root = scratch("import-none");
    let saves = root.join("saves");
    fs::create_dir_all(&saves).expect("saves folder");
    let archive = root.join("Pack.zip");
    zip_of(&archive, &[("mods/sodium.jar", b"jar")]);

    assert!(import(&archive, &saves).is_err());
    assert_eq!(fs::read_dir(&saves).expect("saves lists").count(), 0, "no folder is left behind");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn an_entry_that_would_leave_the_save_refuses_the_whole_archive() {
    let root = scratch("import-slip");
    let saves = root.join("saves");
    fs::create_dir_all(&saves).expect("saves folder");
    let archive = root.join("Evil.zip");
    zip_of(&archive, &[("level.dat", b"level"), ("../escape.txt", b"escape")]);

    let error = import(&archive, &saves).expect_err("the archive is refused");
    assert!(error.contains("escape.txt"), "the error names the entry: {error}");
    assert!(!root.join("escape.txt").exists(), "nothing lands outside the save");
    assert_eq!(fs::read_dir(&saves).expect("saves lists").count(), 0, "no folder is left behind");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn file_names_lose_the_characters_a_file_name_cannot_hold() {
    assert_eq!(sanitize(" My/World: \"1\" "), "My_World_ _1_");
}
