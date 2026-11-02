//! Tests for `pages/worlds/scan.rs`, kept out of the source file. They are compiled as the `tests` child module of
//! `scan`.

use std::fs;
use std::path::Path;

use super::*;

/// A fresh folder under the system temp directory, named for the test.
fn scratch(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("lodestone-worlds-page-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).expect("scratch folder");
    path
}

/// A save named `name` in the instance folder `instance`. Its `level.dat` does not decode, so the save still lists,
/// under its folder name.
fn save(instance: &Path, name: &str) {
    let folder = instance.join("saves").join(name);
    fs::create_dir_all(&folder).expect("save folder");
    fs::write(folder.join("level.dat"), b"not gzip").expect("level.dat writes");
}

/// A backup zip named `name` in the `backups/` of the instance folder `instance`.
fn backup(instance: &Path, name: &str) {
    let folder = instance.join("backups");
    fs::create_dir_all(&folder).expect("backups folder");
    fs::write(folder.join(name), b"zip").expect("backup writes");
}

#[test]
fn every_source_lists_its_own_saves_under_its_instance() {
    let root = scratch("sources");
    let aether = root.join("aether");
    let skyfall = root.join("skyfall");
    save(&aether, "Hub");
    save(&aether, "Spawn");
    save(&skyfall, "Sky");
    let sources = vec![
        Source {
            id: "aether-id".into(),
            name: "Aether Legacy".into(),
            mc_version: "1.20.1".into(),
            folder: aether,
        },
        Source {
            id: "skyfall-id".into(),
            name: "Skyfall".into(),
            mc_version: "1.20.4".into(),
            folder: skyfall,
        },
        Source {
            id: "missing-id".into(),
            name: "Missing".into(),
            mc_version: "1.21".into(),
            folder: root.join("missing"),
        },
    ];

    let mut listed: Vec<(String, String)> = scan_all(&sources)
        .into_iter()
        .map(|entry| (entry.instance_id, entry.world.name))
        .collect();
    listed.sort();

    assert_eq!(
        listed,
        [
            ("aether-id".to_owned(), "Hub".to_owned()),
            ("aether-id".to_owned(), "Spawn".to_owned()),
            ("skyfall-id".to_owned(), "Sky".to_owned()),
        ]
    );
    fs::remove_dir_all(&root).ok();
}

#[test]
fn every_source_lists_its_backups_newest_first_under_its_instance() {
    let root = scratch("backups");
    let aether = root.join("aether");
    let skyfall = root.join("skyfall");
    backup(&aether, "Hub-2026-10-20_12.00.00.zip");
    backup(&skyfall, "Sky-2026-10-25_12.00.00.zip");
    backup(&aether, "Spawn-2026-10-28_12.00.00.zip");
    let sources = vec![
        Source {
            id: "aether-id".into(),
            name: "Aether Legacy".into(),
            mc_version: "1.20.1".into(),
            folder: aether,
        },
        Source {
            id: "skyfall-id".into(),
            name: "Skyfall".into(),
            mc_version: "1.20.4".into(),
            folder: skyfall,
        },
    ];

    let listed: Vec<(String, String)> = scan_backups(&sources)
        .into_iter()
        .map(|entry| (entry.instance_id, entry.backup.folder))
        .collect();

    assert_eq!(
        listed,
        [
            ("aether-id".to_owned(), "Spawn".to_owned()),
            ("skyfall-id".to_owned(), "Sky".to_owned()),
            ("aether-id".to_owned(), "Hub".to_owned()),
        ],
        "newest first, whichever instance holds the backup"
    );
    fs::remove_dir_all(&root).ok();
}

#[test]
fn the_subtitle_falls_back_to_the_instance_version_without_a_saved_one() {
    let root = scratch("subtitle");
    let aether = root.join("aether");
    save(&aether, "Hub");
    let sources = vec![Source {
        id: "aether-id".into(),
        name: "Aether Legacy".into(),
        mc_version: "1.20.1".into(),
        folder: aether,
    }];

    let entries = scan_all(&sources);

    assert_eq!(entries[0].subtitle(), "Aether Legacy · 1.20.1");
    fs::remove_dir_all(&root).ok();
}
