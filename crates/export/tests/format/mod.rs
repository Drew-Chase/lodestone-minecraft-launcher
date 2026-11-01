use super::*;

#[test]
fn jars_and_zips_are_stored_and_everything_else_is_deflated() {
    let source = Path::new("missing");
    let names = ["mods/sodium.jar", "resourcepacks/pack.zip", "config/options.toml"];
    let mut zip = ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for name in names {
        zip.start_file(name, file_options(name, source)).expect("starts");
        zip.write_all(b"data").expect("writes");
    }
    let bytes = zip.finish().expect("finishes").into_inner();
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).expect("reads");
    assert_eq!(archive.by_name("mods/sodium.jar").expect("jar").compression(), CompressionMethod::Stored);
    assert_eq!(
        archive.by_name("resourcepacks/pack.zip").expect("zip").compression(),
        CompressionMethod::Stored
    );
    assert_eq!(
        archive.by_name("config/options.toml").expect("config").compression(),
        CompressionMethod::Deflated
    );
}

#[test]
fn a_write_archive_puts_the_manifest_at_the_root_and_each_file_under_the_prefix() {
    use crate::format::write_archive;
    use crate::progress::{Control, Progress};
    use crate::support::{entry, put, scratch};
    use std::io::Read;
    use std::sync::atomic::AtomicBool;

    let root = scratch("write-archive");
    let config = put(&root, "config/options.toml", b"fov=70");
    let entries = [entry("config/options.toml", config, false, Default::default())];
    let target = root.join("out.zip");
    let cancel = AtomicBool::new(false);
    let mut report = |_: Progress| {};
    let mut control = Control::new(&cancel, 1, 6, &mut report);
    let files: Vec<&Entry> = entries.iter().collect();
    write_archive(&target, ("manifest.json", b"{}"), &files, "overrides/", &mut control).expect("writes");

    let mut archive = zip::ZipArchive::new(std::fs::File::open(&target).expect("opens")).expect("reads");
    let mut manifest = String::new();
    archive
        .by_name("manifest.json")
        .expect("manifest")
        .read_to_string(&mut manifest)
        .expect("reads");
    assert_eq!(manifest, "{}");
    let mut bytes = String::new();
    archive
        .by_name("overrides/config/options.toml")
        .expect("bundled")
        .read_to_string(&mut bytes)
        .expect("reads");
    assert_eq!(bytes, "fov=70");
}

#[test]
fn a_format_bundles_a_linked_file_only_when_it_cannot_link_it() {
    use std::path::PathBuf;

    use crate::hash::Digests;
    use crate::link::{CurseForgeLink, Links};
    use crate::support::modrinth;

    let digests = Digests {
        sha1: "1".repeat(40),
        sha512: "2".repeat(128),
    };
    let modrinth_file = Entry {
        path: "mods/sodium.jar".to_string(),
        source: PathBuf::from("/a"),
        size: 9,
        digests: Some(digests.clone()),
        links: modrinth("P", "V", "https://cdn.example/s.jar"),
    };
    let curseforge_file = Entry {
        path: "mods/curse.jar".to_string(),
        source: PathBuf::from("/b"),
        size: 9,
        digests: Some(digests),
        links: Links {
            modrinth: None,
            curseforge: Some(CurseForgeLink {
                project: "1".to_string(),
                file: "2".to_string(),
            }),
        },
    };
    let plain = Entry {
        path: "config/options.toml".to_string(),
        source: PathBuf::from("/c"),
        size: 3,
        digests: None,
        links: Links::default(),
    };
    assert!(!is_bundled(Format::Mrpack, &modrinth_file, false));
    assert!(
        is_bundled(Format::Mrpack, &curseforge_file, false),
        "the index cannot name a CurseForge file"
    );
    assert!(!is_bundled(Format::CurseForge, &curseforge_file, false));
    assert!(
        is_bundled(Format::CurseForge, &modrinth_file, false),
        "a CurseForge pack cannot name a Modrinth file"
    );
    assert!(is_bundled(Format::Zip, &modrinth_file, false), "a zip holds every file");
    assert!(!is_bundled(Format::Lodestone, &modrinth_file, false));
    assert!(
        is_bundled(Format::Lodestone, &modrinth_file, true),
        "an offline portable bundles every file"
    );
    assert!(is_bundled(Format::Portable, &plain, false));
}
