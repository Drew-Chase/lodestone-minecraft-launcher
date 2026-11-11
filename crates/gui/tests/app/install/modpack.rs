use super::*;
use lodestone_export::format::{Metadata, Package};
use lodestone_export::link::{Entry, Links};
use lodestone_export::progress::{Control, Progress};
use lodestone_platforms::{Modpack, ModpackArchive};
use sha1::{Digest, Sha1};
use std::io::Read;
use std::sync::atomic::AtomicBool;

/// The SHA-1 of the file at `path`, as lowercase hex.
fn sha1_of(path: &Path) -> Result<String, String> {
    let mut file = std::fs::File::open(path).map_err(|error| error.to_string())?;
    let mut hasher = Sha1::new();
    let mut buffer = [0u8; 8192];
    loop {
        let read = file.read(&mut buffer).map_err(|error| error.to_string())?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

/// A scratch folder unique to this test run.
fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lodestone-modpack-{tag}-{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).expect("scratch folder created");
    dir
}

/// Writes a zip at `path` holding each `(name, body)` entry.
fn archive(path: &Path, entries: &[(&str, &str)]) {
    let file = std::fs::File::create(path).expect("archive created");
    let mut zip = zip::ZipWriter::new(file);
    for (name, body) in entries {
        zip.start_file(*name, zip::write::SimpleFileOptions::default()).expect("entry started");
        zip.write_all(body.as_bytes()).expect("entry written");
    }
    zip.finish().expect("archive finished");
}

/// A modpack with no files, for the payload tests to add to.
fn modpack() -> Modpack {
    Modpack {
        name: "Pack".to_string(),
        version: None,
        minecraft: "1.20.1".to_string(),
        loader: Some("fabric"),
        loader_version: Some("0.16.7".to_string()),
        files: Vec::new(),
        overrides: None,
        overrides_skip: Vec::new(),
        archives: Vec::new(),
    }
}

#[test]
fn overrides_are_laid_into_the_instance_and_nothing_else_is_extracted() {
    let dir = scratch("overrides");
    let zip_path = dir.join("pack.zip");
    let game = dir.join("game");
    archive(
        &zip_path,
        &[
            ("overrides/config/a.toml", "x=1"),
            ("other/readme.txt", "not part of the overrides"),
            ("overrides/../escape.txt", "must not leave the instance"),
        ],
    );
    extract(&zip_path, "overrides", "", &[], &game).expect("overrides extract");
    assert_eq!(std::fs::read_to_string(game.join("config/a.toml")).expect("override written"), "x=1");
    assert!(!game.join("other").exists());
    assert!(!dir.join("escape.txt").exists(), "a path that climbs out is skipped");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn extract_reports_the_paths_it_wrote_relative_to_the_instance() {
    let dir = scratch("written");
    let zip_path = dir.join("pack.zip");
    let game = dir.join("game");
    archive(
        &zip_path,
        &[
            ("overrides/config/a.toml", "x=1"),
            ("overrides/mods/m.jar", "jar"),
            ("other/readme.txt", "not written"),
        ],
    );
    let at_root = extract(&zip_path, "overrides", "", &[], &game).expect("overrides extract");
    assert_eq!(at_root, vec!["config/a.toml".to_string(), "mods/m.jar".to_string()]);
    let in_folder = extract(&zip_path, "overrides", "bundle", &[], &game).expect("overrides extract into a folder");
    assert_eq!(in_folder, vec!["bundle/config/a.toml".to_string(), "bundle/mods/m.jar".to_string()]);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_lodestone_package_lays_its_bundled_files_over_the_instance() {
    let dir = scratch("package");
    let source = dir.join("source/config/a.toml");
    std::fs::create_dir_all(source.parent().expect("a file has a parent")).expect("source folder made");
    std::fs::write(&source, "x=1").expect("source written");
    let entries = vec![Entry {
        path: "config/a.toml".to_string(),
        source,
        size: 3,
        digests: None,
        links: Links::default(),
    }];
    let metadata = Metadata {
        name: "Pack".to_string(),
        version: "1.0.0".to_string(),
        description: String::new(),
        minecraft: "1.20.1".to_string(),
        loader: "fabric".to_string(),
        loader_version: None,
        locked: false,
    };
    let package = Package {
        id: "pack-id".to_string(),
        exported_at: 0,
        metadata: &metadata,
        entries: &entries,
        bundle_linked: true,
    };
    let cancel = AtomicBool::new(false);
    let mut report = |_: Progress| {};
    let mut control = Control::new(&cancel, 1, 3, &mut report);
    let bytes = lodestone::write_to(Vec::new(), &package, &mut control).expect("package written");
    let archive = dir.join("pack.lodestone");
    std::fs::write(&archive, bytes).expect("package saved");
    let game = dir.join("game");
    let unpack = Unpack {
        origin: Origin::Package(archive.clone()),
        folder: String::new(),
        target: String::new(),
        skip: Vec::new(),
    };
    unpack_all(&[(archive, unpack)], &game, &AtomicBool::new(false)).expect("package unpacked");
    assert_eq!(std::fs::read_to_string(game.join("config/a.toml")).expect("override written"), "x=1");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn the_root_of_an_archive_unpacks_whole_except_the_entries_skipped() {
    let dir = scratch("root");
    let zip_path = dir.join("pack.zip");
    let game = dir.join("game");
    archive(&zip_path, &[("bin/version.json", "{}"), ("mods/a.jar", "jar"), ("config/a.cfg", "x=1")]);
    extract(&zip_path, "", "", &["bin/version.json".to_string()], &game).expect("archive extracts");
    assert_eq!(std::fs::read_to_string(game.join("mods/a.jar")).expect("mod written"), "jar");
    assert_eq!(std::fs::read_to_string(game.join("config/a.cfg")).expect("config written"), "x=1");
    assert!(!game.join("bin/version.json").exists(), "a skipped entry is not unpacked");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn an_archive_folder_lands_in_its_target_folder() {
    let dir = scratch("target");
    let zip_path = dir.join("resources.zip");
    let game = dir.join("game");
    archive(&zip_path, &[("mods/extra.jar", "jar"), ("readme.txt", "not in mods")]);
    extract(&zip_path, "mods", "mods", &[], &game).expect("folder extracts");
    assert_eq!(std::fs::read_to_string(game.join("mods/extra.jar")).expect("mod written"), "jar");
    assert!(!game.join("readme.txt").exists());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_pack_lays_its_overrides_after_its_archives() {
    let mut pack = modpack();
    pack.overrides = Some("overrides".to_string());
    pack.overrides_skip = vec!["bin/version.json".to_string()];
    pack.archives = vec![ModpackArchive {
        name: "Configs.zip".to_string(),
        url: "https://cdn.test/Configs.zip".to_string(),
        sha1: Some("abc".to_string()),
        size: Some(10),
        target: String::new(),
        prefix: String::new(),
    }];
    let payload = Payload::from_modpack(&pack, Some(PathBuf::from("pack.zip")));
    assert_eq!(payload.unpacks.len(), 2);
    assert!(matches!(payload.unpacks[0].origin, Origin::Remote { .. }));
    assert_eq!(payload.unpacks[1].folder, "overrides");
    assert!(matches!(&payload.unpacks[1].origin, Origin::Local(path) if path == Path::new("pack.zip")));
    assert_eq!(payload.unpacks[1].skip, ["bin/version.json"]);
}

#[test]
fn a_pack_without_overrides_or_archives_unpacks_nothing() {
    let payload = Payload::from_modpack(&modpack(), Some(PathBuf::from("pack.zip")));
    assert!(payload.unpacks.is_empty());
}

#[test]
fn the_sha1_of_a_file_matches_its_digest() {
    let dir = scratch("sha1");
    let path = dir.join("abc.txt");
    std::fs::write(&path, b"abc").expect("file written");
    assert_eq!(sha1_of(&path).expect("hashed"), "a9993e364706816aba3e25717850c26c9cd0d89d");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_launcher_folder_is_copied_into_the_instance_without_the_names_it_skips() {
    let dir = scratch("launcher-folder");
    let source = dir.join("source");
    std::fs::create_dir_all(source.join("mods")).expect("mods folder made");
    std::fs::create_dir_all(source.join("logs")).expect("logs folder made");
    std::fs::write(source.join("mods").join("sodium.jar"), "jar").expect("mod written");
    std::fs::write(source.join("logs").join("latest.log"), "log").expect("log written");
    std::fs::write(source.join("minecraftinstance.json"), "{}").expect("metadata written");
    let game = dir.join("instance");
    let unpack = Unpack {
        origin: Origin::Folder {
            root: source.clone(),
            skip: vec!["minecraftinstance.json".to_string()],
        },
        folder: String::new(),
        target: String::new(),
        skip: Vec::new(),
    };
    unpack_all(&[(source.clone(), unpack)], &game, &AtomicBool::new(false)).expect("the folder is copied");
    assert_eq!(std::fs::read_to_string(game.join("mods").join("sodium.jar")).expect("mod copied"), "jar");
    assert!(!game.join("logs").exists(), "logs are never copied");
    assert!(!game.join("minecraftinstance.json").exists(), "the launcher's own file is left out");
    std::fs::remove_dir_all(&dir).ok();
}
