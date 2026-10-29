//! Tests for `pages/instance/files/mod.rs`, kept out of the source file. They are
//! compiled as the `tests` child module of `files`, so they reach the folder
//! walk and the scanners directly.

use std::fs;
use std::path::{Path, PathBuf};

use super::*;

/// A fresh folder under the system temp directory, named for the test.
fn scratch(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("lodestone-files-{}-{name}", std::process::id()));
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
fn dir_size_adds_every_file_under_the_folder() {
    let root = scratch("size");
    write(&root.join("saves/World/level.dat"), &[0; 10]);
    write(&root.join("saves/World/region/r.0.0.mca"), &[0; 25]);
    assert_eq!(dir_size(&root), 35);
    assert_eq!(dir_size(&root.join("missing")), 0, "a missing folder is empty");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn count_mods_counts_enabled_and_disabled_jars() {
    let root = scratch("count");
    write(&root.join("mods/a.jar"), b"");
    write(&root.join("mods/b.jar.disabled"), b"");
    write(&root.join("mods/notes.txt"), b"");
    assert_eq!(count_mods(&root), 2);
    fs::remove_dir_all(&root).ok();
}

#[test]
fn scan_reads_every_part_of_the_folder() {
    let root = scratch("scan");
    write(&root.join("mods/a.jar"), b"not a zip");
    write(&root.join("saves/Hub/level.dat"), b"not gzip");
    write(&root.join("screenshots/2026-10-29_12.00.00.png"), &[0; 8]);
    write(&root.join("logs/latest.log"), b"[12:00:00] [main/INFO]: Ready\n");

    let files = scan(&root);
    assert_eq!(files.mods.len(), 1, "a jar that is not a zip is still listed");
    assert_eq!(files.worlds.len(), 1, "a save whose level.dat will not decode is still listed");
    assert_eq!(files.screenshots.len(), 1);
    assert_eq!(files.log.len(), 1);
    assert_eq!(files.folder_bytes, 9 + 8 + 8 + 30);
    fs::remove_dir_all(&root).ok();
}

#[test]
fn a_missing_instance_folder_scans_to_nothing() {
    let files = scan(Path::new("/definitely/not/an/instance/folder"));
    assert!(files.mods.is_empty());
    assert!(files.worlds.is_empty());
    assert!(files.screenshots.is_empty());
    assert!(files.log.is_empty());
    assert_eq!(files.folder_bytes, 0);
}

#[test]
fn decode_image_reads_pngs_and_rejects_everything_else() {
    let root = scratch("decode");
    let png = root.join("shot.png");
    image::RgbaImage::new(400, 200).save(&png).expect("a png is written");
    assert!(decode_image(&png, Some((80, 40))).is_some(), "a png decodes into the bounds");
    assert!(decode_image(&png, None).is_some(), "without bounds it decodes whole");

    let broken = root.join("broken.png");
    fs::write(&broken, b"not a png").expect("file writes");
    assert!(decode_image(&broken, None).is_none());
    assert!(decode_image(&root.join("missing.png"), None).is_none());
    fs::remove_dir_all(&root).ok();
}
