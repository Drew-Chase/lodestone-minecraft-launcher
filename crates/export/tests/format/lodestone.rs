use std::sync::atomic::AtomicBool;

use super::*;
use crate::format::{Metadata, Package};
use crate::progress::Progress;
use crate::support::{curseforge, entry, metadata, modrinth, put, scratch};

const SODIUM: &str = "https://cdn.modrinth.com/data/AANobbMI/versions/abc/sodium.jar";

/// A package of a linked mod, an unlinked mod and a config file, written at `bundle_linked`.
fn package_bytes(root: &Path, bundle_linked: bool) -> Vec<u8> {
    package_bytes_of(root, bundle_linked, &metadata())
}

/// The package of [`package_bytes`], for the instance `metadata` describes.
fn package_bytes_of(root: &Path, bundle_linked: bool, metadata: &Metadata) -> Vec<u8> {
    let entries = vec![
        entry(
            "config/options.toml",
            put(root, "config/options.toml", b"fov=70"),
            false,
            Default::default(),
        ),
        entry("mods/local.jar", put(root, "mods/local.jar", b"local"), true, Default::default()),
        entry(
            "mods/sodium.jar",
            put(root, "mods/sodium.jar", b"sodium"),
            true,
            modrinth("AANobbMI", "abc", SODIUM),
        ),
        entry(
            "mods/curse.jar",
            put(root, "mods/curse.jar", b"curse"),
            true,
            curseforge("238222", "4123456"),
        ),
    ];
    let package = Package {
        id: "pack-id".to_string(),
        exported_at: 1_700_000_000,
        metadata,
        entries: &entries,
        bundle_linked,
    };
    let cancel = AtomicBool::new(false);
    let mut report = |_: Progress| {};
    let total: u64 = entries.iter().map(|entry| entry.size).sum();
    let mut control = Control::new(&cancel, entries.len(), total, &mut report);
    write_to(Vec::new(), &package, &mut control).expect("writes")
}

#[test]
fn the_package_is_xz_and_its_first_entry_is_the_manifest() {
    let root = scratch("lodestone-xz");
    let bytes = package_bytes(&root, false);
    assert_eq!(&bytes[..6], &[0xFD, b'7', b'z', b'X', b'Z', 0x00], "the xz magic");
    let manifest = read_manifest(&bytes[..]).expect("reads");
    assert_eq!(manifest.format, FORMAT);
    assert_eq!(manifest.id, "pack-id");
    assert_eq!(manifest.name, "Enhanced Worlds");
    assert_eq!(manifest.minecraft, "1.21.1");
    assert_eq!(manifest.loader.id, "fabric");
    assert_eq!(manifest.loader.version.as_deref(), Some("0.16.10"));
    assert_eq!(manifest.exported_at, 1_700_000_000);
    assert_eq!(manifest.files.len(), 3, "every mod and resource file is recorded");
}

#[test]
fn linked_files_are_not_bundled_unless_asked() {
    let root = scratch("lodestone-links");
    let manifest = read_manifest(&package_bytes(&root, false)[..]).expect("reads");
    let mut overrides = manifest.overrides.clone();
    overrides.sort();
    assert_eq!(overrides, ["config/options.toml", "mods/local.jar"]);
    let sodium = manifest.files.iter().find(|file| file.path == "mods/sodium.jar").expect("recorded");
    assert!(!sodium.bundled);
    assert_eq!(sodium.links.modrinth.as_ref().expect("linked").download_url, SODIUM);
    let curse = manifest.files.iter().find(|file| file.path == "mods/curse.jar").expect("recorded");
    assert_eq!(curse.links.curseforge.as_ref().expect("linked").file, "4123456");
}

#[test]
fn an_offline_package_bundles_every_file() {
    let root = scratch("lodestone-offline");
    let manifest = read_manifest(&package_bytes(&root, true)[..]).expect("reads");
    assert_eq!(manifest.overrides.len(), 4);
    assert!(manifest.files.iter().all(|file| file.bundled));
}

#[test]
fn a_locked_instance_is_locked_in_its_package() {
    let root = scratch("lodestone-locked");
    let mut locked = metadata();
    locked.locked = true;
    let manifest = read_manifest(&package_bytes_of(&root, false, &locked)[..]).expect("reads");
    assert!(manifest.locked);
}

#[test]
fn an_unlocked_instance_is_unlocked_in_its_package() {
    let root = scratch("lodestone-unlocked");
    let manifest = read_manifest(&package_bytes(&root, false)[..]).expect("reads");
    assert!(!manifest.locked);
}

#[test]
fn a_package_from_before_the_lock_restores_unlocked() {
    let older = r#"{"format":1,"id":"older","name":"Older","version":"1.0.0","exportedAt":1,"minecraft":"1.21.1","loader":{"id":"fabric"}}"#;
    let manifest: Manifest = serde_json::from_str(older).expect("an older manifest parses");
    assert!(!manifest.locked);
}

#[test]
fn extracting_the_overrides_restores_the_bundled_bytes_only() {
    let root = scratch("lodestone-extract");
    let bytes = package_bytes(&root, false);
    let destination = root.join("instance");
    extract_overrides(&bytes[..], &destination).expect("extracts");
    assert_eq!(std::fs::read(destination.join("config/options.toml")).expect("extracted"), b"fov=70");
    assert_eq!(std::fs::read(destination.join("mods/local.jar")).expect("extracted"), b"local");
    assert!(!destination.join("mods/sodium.jar").exists(), "a linked mod is fetched, not bundled");
    assert!(!destination.join(MANIFEST).exists(), "the manifest is not an override");
}

#[test]
fn a_cancelled_write_stops_before_the_next_file() {
    let root = scratch("lodestone-cancel");
    let metadata = metadata();
    let entries = vec![entry(
        "config/options.toml",
        put(&root, "config/options.toml", b"x"),
        false,
        Default::default(),
    )];
    let package = Package {
        id: "id".to_string(),
        exported_at: 0,
        metadata: &metadata,
        entries: &entries,
        bundle_linked: false,
    };
    let cancel = AtomicBool::new(true);
    let mut report = |_: Progress| {};
    let mut control = Control::new(&cancel, 1, 1, &mut report);
    assert!(matches!(write_to(Vec::new(), &package, &mut control), Err(crate::Error::Cancelled)));
}
