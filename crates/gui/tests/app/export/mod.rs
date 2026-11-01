use std::fs;
use std::sync::atomic::AtomicBool;

use lodestone_export::format::{Metadata, curseforge, mrpack};
use lodestone_export::hash::digest_file;
use lodestone_export::link::{CurseForgeLink, Entry, Links, ModrinthLink};
use lodestone_export::progress::{Control, Progress};
use lodestone_platforms::{CurseForge, Modrinth, Platform};

use super::*;

const SODIUM: &str = "https://cdn.modrinth.com/data/AANobbMI/versions/abc/sodium.jar";

/// A scratch folder with a mod and a config file in it, and the entries that describe them.
fn fixture(name: &str, links: Links) -> (PathBuf, Vec<Entry>) {
    let root = std::env::temp_dir().join(format!("lodestone-gui-export-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("mods")).expect("the folder can be made");
    fs::create_dir_all(root.join("config")).expect("the folder can be made");
    let jar = root.join("mods/sodium.jar");
    let config = root.join("config/options.toml");
    fs::write(&jar, b"sodium").expect("written");
    fs::write(&config, b"fov=70").expect("written");
    let entries = vec![
        Entry {
            path: "config/options.toml".to_string(),
            size: 6,
            digests: None,
            links: Links::default(),
            source: config,
        },
        Entry {
            path: "mods/sodium.jar".to_string(),
            size: 6,
            digests: Some(digest_file(&jar).expect("hashes")),
            links,
            source: jar,
        },
    ];
    (root, entries)
}

fn metadata() -> Metadata {
    Metadata {
        name: "Enhanced Worlds".to_string(),
        version: "1.0.0".to_string(),
        description: "A test pack".to_string(),
        minecraft: "1.21.1".to_string(),
        loader: "fabric".to_string(),
        loader_version: Some("0.16.10".to_string()),
    }
}

#[test]
fn the_formats_are_listed_in_the_picker_order() {
    assert_eq!(
        FORMATS,
        [Format::Mrpack, Format::CurseForge, Format::Zip, Format::Lodestone, Format::Portable]
    );
}

#[test]
fn each_format_has_its_own_label_and_extension() {
    assert_eq!(format_label(Format::Mrpack), "Modrinth pack (.mrpack)");
    assert_eq!(extension(Format::Mrpack), "mrpack");
    assert_eq!(extension(Format::CurseForge), "zip");
    assert_eq!(extension(Format::Zip), "zip");
    assert_eq!(extension(Format::Lodestone), "lodestone");
}

#[test]
fn a_modrinth_pack_written_by_the_export_reads_back_through_the_platform_parser() {
    let links = Links {
        modrinth: Some(ModrinthLink {
            project: "AANobbMI".to_string(),
            version: "abc".to_string(),
            page: "https://modrinth.com/project/AANobbMI".to_string(),
            download_url: SODIUM.to_string(),
        }),
        curseforge: None,
    };
    let (root, entries) = fixture("mrpack", links);
    let target = root.join("pack.mrpack");
    let cancel = AtomicBool::new(false);
    let mut report = |_: Progress| {};
    let mut control = Control::new(&cancel, 1, 6, &mut report);
    mrpack::write(&target, &metadata(), &entries, &mut control).expect("writes");

    let modpack = Platform::<Modrinth>::new().parse_modpack(&target).expect("the platform reads the pack");
    assert_eq!(modpack.name, "Enhanced Worlds");
    assert_eq!(modpack.version.as_deref(), Some("1.0.0"));
    assert_eq!(modpack.minecraft, "1.21.1");
    assert_eq!(modpack.loader, Some("fabric"));
    assert_eq!(modpack.loader_version.as_deref(), Some("0.16.10"));
    assert_eq!(modpack.files.len(), 1, "the linked mod is listed; the config is in the overrides");
    assert_eq!(modpack.files[0].path, "mods/sodium.jar");
    assert_eq!(modpack.files[0].url, SODIUM);
    assert!(modpack.files[0].sha1.is_some());
    assert_eq!(modpack.overrides.as_deref(), Some("overrides"));
}

#[test]
fn a_curseforge_pack_written_by_the_export_reads_back_through_the_platform_parser() {
    let links = Links {
        modrinth: None,
        curseforge: Some(CurseForgeLink {
            project: "238222".to_string(),
            file: "4123456".to_string(),
        }),
    };
    let (root, entries) = fixture("curseforge", links);
    let target = root.join("pack.zip");
    let cancel = AtomicBool::new(false);
    let mut report = |_: Progress| {};
    let mut control = Control::new(&cancel, 1, 6, &mut report);
    curseforge::write(&target, &metadata(), &entries, &mut control).expect("writes");

    let modpack = Platform::<CurseForge>::new().parse_modpack(&target).expect("the platform reads the pack");
    assert_eq!(modpack.minecraft, "1.21.1");
    assert_eq!(modpack.loader, Some("fabric"));
    assert_eq!(modpack.loader_version.as_deref(), Some("0.16.10"));
    assert_eq!(modpack.files.len(), 1);
    assert_eq!(modpack.files[0].project_id.as_deref(), Some("238222"));
    assert_eq!(modpack.files[0].file_id.as_deref(), Some("4123456"));
}
