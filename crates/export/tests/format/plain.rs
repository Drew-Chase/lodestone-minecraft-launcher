use std::fs::File;
use std::io::Read;
use std::sync::atomic::AtomicBool;

use super::*;
use crate::format::Package;
use crate::progress::Progress;
use crate::support::{entry, metadata, modrinth, put, scratch};

#[test]
fn the_zip_keeps_every_file_at_its_own_path_with_a_manifest_at_the_root() {
    let root = scratch("plain-zip");
    let metadata = metadata();
    let entries = vec![
        entry(
            "config/options.toml",
            put(&root, "config/options.toml", b"fov=70"),
            false,
            Default::default(),
        ),
        entry(
            "mods/sodium.jar",
            put(&root, "mods/sodium.jar", b"sodium"),
            true,
            modrinth("P", "V", "https://cdn.example/s.jar"),
        ),
    ];
    let package = Package {
        id: "abc123".to_string(),
        exported_at: 1_700_000_000,
        metadata: &metadata,
        entries: &entries,
        bundle_linked: false,
    };
    let cancel = AtomicBool::new(false);
    let mut report = |_: Progress| {};
    let mut control = Control::new(&cancel, entries.len(), 11, &mut report);
    let target = root.join("instance.zip");
    write(&target, &package, &mut control).expect("writes");

    let mut archive = zip::ZipArchive::new(File::open(&target).expect("opens")).expect("reads");
    let mut text = String::new();
    archive
        .by_name(MANIFEST)
        .expect("the manifest is at the root")
        .read_to_string(&mut text)
        .expect("reads");
    let manifest: serde_json::Value = serde_json::from_str(&text).expect("JSON");
    assert_eq!(manifest["id"], "abc123");
    assert_eq!(manifest["exportedAt"], 1_700_000_000);
    assert!(manifest.get("overrides").is_none(), "a plain zip has no overrides list");
    let files = manifest["files"].as_array().expect("files is a list");
    assert_eq!(files.len(), 1, "only the mod has digests");
    assert_eq!(files[0]["bundled"], true);
    assert_eq!(files[0]["links"]["modrinth"]["project"], "P");

    let mut bytes = String::new();
    archive
        .by_name("config/options.toml")
        .expect("the config is at its own path")
        .read_to_string(&mut bytes)
        .expect("reads");
    assert_eq!(bytes, "fov=70");
    assert!(archive.by_name("mods/sodium.jar").is_ok());
}
