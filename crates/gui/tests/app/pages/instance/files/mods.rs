//! Tests for `pages/instance/files/mods.rs`, kept out of the source file. They
//! are compiled as the `tests` child module of `mods`.

use std::fs;
use std::path::{Path, PathBuf};

use super::*;

/// A fresh folder under the system temp directory, named for the test.
fn scratch(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("lodestone-mods-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).expect("scratch folder");
    path
}

/// Writes `contents` to `path`, creating the folders above it.
fn write(path: &Path, contents: &[u8]) {
    fs::create_dir_all(path.parent().expect("a file has a parent")).expect("parent folder");
    fs::write(path, contents).expect("file writes");
}

#[test]
fn only_jars_are_listed_sorted_by_name() {
    let root = scratch("list");
    write(&root.join("b-mod.jar"), b"");
    write(&root.join("A-mod.jar.disabled"), b"");
    write(&root.join("c.JAR"), b"");
    write(&root.join("notes.txt"), b"");
    fs::create_dir_all(root.join("folder.jar")).expect("a folder named like a jar");

    let listed: Vec<(String, bool)> = scan(&root).into_iter().map(|jar| (jar.name, jar.enabled)).collect();
    assert_eq!(
        listed,
        [("A-mod".to_string(), false), ("b-mod".to_string(), true), ("c".to_string(), true),],
        "the disabled suffix turns a mod off, and folders and other files are skipped"
    );
    fs::remove_dir_all(&root).ok();
}

#[test]
fn a_jar_without_metadata_keeps_its_file_name() {
    let root = scratch("bare");
    write(&root.join("sodium-0.5.8.jar"), b"not a zip");
    let jar = scan(&root).remove(0);
    assert_eq!(jar.file_name, "sodium-0.5.8.jar");
    assert_eq!(jar.name, "sodium-0.5.8");
    assert!(jar.author.is_none() && jar.version.is_none());
    fs::remove_dir_all(&root).ok();
}

#[test]
fn count_matches_the_jars_scan_lists() {
    let root = scratch("count");
    write(&root.join("a.jar"), b"");
    write(&root.join("b.jar.disabled"), b"");
    write(&root.join("c.txt"), b"");
    assert_eq!(count(&root), 2);
    assert_eq!(scan(&root).len(), 2);
    fs::remove_dir_all(&root).ok();
}

#[test]
fn a_missing_mods_folder_has_no_mods() {
    let root = scratch("missing");
    assert!(scan(&root.join("missing")).is_empty());
    assert_eq!(count(&root.join("missing")), 0);
    fs::remove_dir_all(&root).ok();
}

#[test]
fn the_disabled_suffix_is_matched_without_regard_to_case_and_splits_no_character() {
    assert_eq!(strip_suffix_ignore_case("Sodium.JAR.Disabled", ".jar.disabled"), Some("Sodium"));
    assert_eq!(strip_suffix_ignore_case("naïve.jar", ".jar"), Some("naïve"));
    assert_eq!(strip_suffix_ignore_case("€.jar", ".jar.disabled"), None);
    assert_eq!(strip_suffix_ignore_case("x", ".jar"), None);
}
