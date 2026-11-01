use std::fs::File;
use std::io::Read;
use std::sync::atomic::AtomicBool;

use super::*;
use crate::progress::Progress;
use crate::support::{entry, metadata, modrinth, put, scratch};

const SODIUM: &str = "https://cdn.modrinth.com/data/AANobbMI/versions/abc/sodium.jar";

fn read_index(target: &Path) -> serde_json::Value {
    let mut archive = zip::ZipArchive::new(File::open(target).expect("opens")).expect("reads");
    let mut text = String::new();
    archive
        .by_name(INDEX)
        .expect("the index is at the root")
        .read_to_string(&mut text)
        .expect("reads");
    serde_json::from_str(&text).expect("the index is JSON")
}

fn write_pack(target: &Path, entries: &[Entry]) {
    let cancel = AtomicBool::new(false);
    let mut report = |_: Progress| {};
    let total: u64 = entries.iter().map(|entry| entry.size).sum();
    let mut control = Control::new(&cancel, entries.len(), total, &mut report);
    write(target, &metadata(), entries, &mut control).expect("writes");
}

#[test]
fn the_index_places_linked_mods_and_bundles_the_rest() {
    let root = scratch("mrpack-index");
    let entries = vec![
        entry(
            "config/options.toml",
            put(&root, "config/options.toml", b"fov=70"),
            false,
            Default::default(),
        ),
        entry("mods/local.jar", put(&root, "mods/local.jar", b"local"), true, Default::default()),
        entry(
            "mods/sodium.jar",
            put(&root, "mods/sodium.jar", b"sodium"),
            true,
            modrinth("AANobbMI", "abc", SODIUM),
        ),
    ];
    let target = root.join("pack.mrpack");
    write_pack(&target, &entries);

    let index = read_index(&target);
    assert_eq!(index["formatVersion"], 1);
    assert_eq!(index["game"], "minecraft");
    assert_eq!(index["versionId"], "1.0.0");
    assert_eq!(index["name"], "Enhanced Worlds");
    assert_eq!(index["summary"], "A test pack");
    assert_eq!(index["dependencies"]["minecraft"], "1.21.1");
    assert_eq!(index["dependencies"]["fabric-loader"], "0.16.10");

    let files = index["files"].as_array().expect("files is a list");
    assert_eq!(files.len(), 1);
    assert_eq!(files[0]["path"], "mods/sodium.jar");
    assert_eq!(files[0]["downloads"][0], SODIUM);
    assert_eq!(files[0]["fileSize"], 6);
    assert_eq!(files[0]["hashes"]["sha1"].as_str().expect("sha1").len(), 40);
    assert_eq!(files[0]["hashes"]["sha512"].as_str().expect("sha512").len(), 128);

    let mut archive = zip::ZipArchive::new(File::open(&target).expect("opens")).expect("reads");
    assert!(archive.by_name("overrides/config/options.toml").is_ok());
    assert!(archive.by_name("overrides/mods/local.jar").is_ok());
    assert!(archive.by_name("overrides/mods/sodium.jar").is_err());
}

#[test]
fn a_vanilla_pack_names_only_the_game_version() {
    let root = scratch("mrpack-vanilla");
    let mut metadata = metadata();
    metadata.loader = "vanilla".to_string();
    metadata.loader_version = None;
    let entries: Vec<Entry> = Vec::new();
    let cancel = AtomicBool::new(false);
    let mut report = |_: Progress| {};
    let mut control = Control::new(&cancel, 0, 0, &mut report);
    let target = root.join("vanilla.mrpack");
    write(&target, &metadata, &entries, &mut control).expect("writes");
    let dependencies = read_index(&target)["dependencies"].clone();
    assert_eq!(dependencies, serde_json::json!({ "minecraft": "1.21.1" }));
}
