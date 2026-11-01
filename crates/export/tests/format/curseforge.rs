use std::fs::File;
use std::io::Read;
use std::sync::atomic::AtomicBool;

use super::*;
use crate::progress::Progress;
use crate::support::{curseforge, entry, metadata, modrinth, put, scratch};

fn read_manifest(target: &Path) -> serde_json::Value {
    let mut archive = zip::ZipArchive::new(File::open(target).expect("opens")).expect("reads");
    let mut text = String::new();
    archive
        .by_name(MANIFEST)
        .expect("the manifest is at the root")
        .read_to_string(&mut text)
        .expect("reads");
    serde_json::from_str(&text).expect("the manifest is JSON")
}

#[test]
fn the_manifest_lists_curseforge_mods_by_id_and_bundles_the_rest() {
    let root = scratch("curseforge-manifest");
    let entries = vec![
        entry("mods/local.jar", put(&root, "mods/local.jar", b"local"), true, Default::default()),
        entry(
            "mods/modrinth.jar",
            put(&root, "mods/modrinth.jar", b"mr"),
            true,
            modrinth("P1", "V1", "https://cdn.example/mr.jar"),
        ),
        entry(
            "mods/sodium.jar",
            put(&root, "mods/sodium.jar", b"sodium"),
            true,
            curseforge("238222", "4123456"),
        ),
        entry(
            "mods/broken.jar",
            put(&root, "mods/broken.jar", b"b"),
            true,
            curseforge("not-a-number", "4"),
        ),
    ];
    let cancel = AtomicBool::new(false);
    let mut report = |_: Progress| {};
    let mut control = Control::new(&cancel, entries.len(), 0, &mut report);
    let target = root.join("pack.zip");
    write(&target, &metadata(), &entries, &mut control).expect("writes");

    let manifest = read_manifest(&target);
    assert_eq!(manifest["manifestType"], "minecraftModpack");
    assert_eq!(manifest["manifestVersion"], 1);
    assert_eq!(manifest["overrides"], "overrides");
    assert_eq!(manifest["name"], "Enhanced Worlds");
    assert_eq!(manifest["minecraft"]["version"], "1.21.1");
    assert_eq!(manifest["minecraft"]["modLoaders"][0]["id"], "fabric-0.16.10");
    assert_eq!(manifest["minecraft"]["modLoaders"][0]["primary"], true);

    let files = manifest["files"].as_array().expect("files is a list");
    assert_eq!(files.len(), 1);
    assert_eq!(files[0]["projectID"], 238222);
    assert_eq!(files[0]["fileID"], 4123456);
    assert_eq!(files[0]["required"], true);

    let mut archive = zip::ZipArchive::new(File::open(&target).expect("opens")).expect("reads");
    assert!(archive.by_name("overrides/mods/local.jar").is_ok());
    assert!(archive.by_name("overrides/mods/modrinth.jar").is_ok());
    assert!(archive.by_name("overrides/mods/broken.jar").is_ok());
    assert!(archive.by_name("overrides/mods/sodium.jar").is_err());
}
