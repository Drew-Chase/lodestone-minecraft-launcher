use lodestone_export::format::lodestone::{FORMAT, FileRecord, LoaderRecord, Manifest};
use lodestone_export::link::{CurseForgeLink, Links, ModrinthLink};

use super::*;

fn manifest(files: Vec<FileRecord>) -> Manifest {
    Manifest {
        format: FORMAT,
        id: "pack-id".to_string(),
        name: "Enhanced Worlds".to_string(),
        version: "1.0.0".to_string(),
        description: String::new(),
        exported_at: 0,
        minecraft: "1.21.1".to_string(),
        loader: LoaderRecord {
            id: "fabric".to_string(),
            version: Some("0.16.10".to_string()),
        },
        files,
        overrides: Vec::new(),
        locked: false,
    }
}

fn record(path: &str, bundled: bool, links: Links) -> FileRecord {
    FileRecord {
        path: path.to_string(),
        sha1: "a".repeat(40),
        sha512: "b".repeat(128),
        size: 9,
        bundled,
        links,
    }
}

fn modrinth(download: &str) -> Links {
    Links {
        modrinth: Some(ModrinthLink {
            project: "AANobbMI".to_string(),
            version: "V1".to_string(),
            page: "https://modrinth.com/project/AANobbMI".to_string(),
            download_url: download.to_string(),
        }),
        curseforge: None,
    }
}

fn curseforge() -> Links {
    Links {
        modrinth: None,
        curseforge: Some(CurseForgeLink {
            project: "238222".to_string(),
            file: "4123456".to_string(),
        }),
    }
}

#[test]
fn a_modrinth_file_downloads_from_its_url_and_a_curseforge_file_comes_by_its_ids() {
    let files = vec![
        record("mods/sodium.jar", false, modrinth("https://cdn.example/sodium.jar")),
        record("mods/curse.jar", false, curseforge()),
        record("config/options.toml", true, Links::default()),
    ];
    let payload = payload(&manifest(files), None);
    assert_eq!(payload.files.len(), 2);
    assert_eq!(payload.files[0].url, "https://cdn.example/sodium.jar");
    assert_eq!(payload.files[0].project_id, None);
    assert_eq!(payload.files[1].url, "", "CurseForge resolves its download when the install starts");
    assert_eq!(payload.files[1].project_id.as_deref(), Some("238222"));
    assert_eq!(payload.files[1].file_id.as_deref(), Some("4123456"));
}

#[test]
fn each_linked_mod_is_recorded_with_the_platform_it_came_from() {
    let files = vec![
        record("mods/sodium.jar", false, modrinth("https://cdn.example/sodium.jar")),
        record("mods/curse.jar", false, curseforge()),
    ];
    let payload = payload(&manifest(files), None);
    assert_eq!(payload.sources.len(), 2);
    assert_eq!(payload.sources[0].platform, "modrinth");
    assert_eq!(payload.sources[0].file_name, "sodium.jar");
    assert_eq!(payload.sources[1].platform, "curseforge");
    assert_eq!(payload.sources[1].version_id, "4123456");
}

#[test]
fn each_linked_pack_in_a_resource_or_shader_folder_is_recorded_as_content() {
    let files = vec![
        record("resourcepacks/Fresh.zip", false, modrinth("https://cdn.example/fresh.zip")),
        record("shaderpacks/Shade.zip", false, curseforge()),
        record("resourcepacks/nested/deep.zip", false, modrinth("https://cdn.example/deep.zip")),
        record("config/options.toml", true, Links::default()),
    ];
    let payload = payload(&manifest(files), None);
    assert_eq!(payload.packs.len(), 2, "a pack nested below its folder is not one of the folder's packs");
    assert_eq!(payload.packs[0].kind, "resourcepacks");
    assert_eq!(payload.packs[0].platform, "modrinth");
    assert_eq!(payload.packs[0].project_id, "AANobbMI");
    assert_eq!(payload.packs[0].version_id, "V1");
    assert_eq!(payload.packs[0].file_name, "Fresh.zip");
    assert_eq!(payload.packs[1].kind, "shaderpacks");
    assert_eq!(payload.packs[1].platform, "curseforge");
    assert_eq!(payload.packs[1].version_id, "4123456");
    assert!(payload.sources.is_empty(), "a pack is recorded as content, not as a mod");
}

#[test]
fn a_bundled_file_is_left_to_the_extraction() {
    let files = vec![
        record("config/options.toml", true, Links::default()),
        record("mods/local.jar", true, Links::default()),
    ];
    let payload = payload(&manifest(files), None);
    assert!(payload.files.is_empty());
    assert!(payload.sources.is_empty());
}

#[test]
fn a_package_on_disk_is_unpacked_over_the_instance_and_one_without_a_path_is_not() {
    let package = Path::new("pack.lodestone");
    let files = vec![record("config/options.toml", true, Links::default())];
    let with = payload(&manifest(files.clone()), Some(package));
    assert_eq!(with.unpacks.len(), 1);
    assert!(matches!(&with.unpacks[0].origin, Origin::Package(path) if path == package));
    let without = payload(&manifest(files), None);
    assert!(without.unpacks.is_empty());
}

#[test]
fn the_instance_takes_the_given_id_and_the_package_versions() {
    let root = std::env::temp_dir().join(format!("lodestone-package-request-{}", std::process::id()));
    let paths = LauncherPaths::new(&root);
    let manifest = manifest(Vec::new());
    let install = request(&paths, &manifest, payload(&manifest, None), manifest.id.clone(), &[]);
    assert_eq!(install.id, "pack-id");
    assert_eq!(install.mc_version, "1.21.1");
    assert_eq!(install.loader, Loader::Fabric);
    assert_eq!(install.loader_version.as_deref(), Some("0.16.10"));
    assert!(!install.directory_name.is_empty());
}

#[test]
fn a_locked_package_installs_a_locked_instance_and_an_older_one_an_unlocked_instance() {
    let root = std::env::temp_dir().join(format!("lodestone-package-locked-{}", std::process::id()));
    let paths = LauncherPaths::new(&root);
    let mut locked = manifest(Vec::new());
    locked.locked = true;
    assert!(request(&paths, &locked, payload(&locked, None), locked.id.clone(), &[]).locked);
    let unlocked = manifest(Vec::new());
    assert!(!request(&paths, &unlocked, payload(&unlocked, None), unlocked.id.clone(), &[]).locked);
}
