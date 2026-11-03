use std::sync::atomic::AtomicBool;

use lodestone_export::format::{Metadata, Package};
use lodestone_export::link::{Entry, Links};
use lodestone_export::progress::{Control, Progress};

use super::*;
use crate::app::install::modpack::Origin;
use crate::app::pages::library::instance::Loader;

/// A scratch folder unique to this test run.
fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lodestone-import-{tag}-{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).expect("scratch folder created");
    dir
}

/// Writes a zip at `path` holding each `(name, body)` entry.
fn zip_at(path: &Path, entries: &[(&str, &str)]) {
    let file = std::fs::File::create(path).expect("zip created");
    let mut zip = zip::ZipWriter::new(file);
    for (name, body) in entries {
        zip.start_file(*name, zip::write::SimpleFileOptions::default()).expect("entry started");
        std::io::Write::write_all(&mut zip, body.as_bytes()).expect("entry written");
    }
    zip.finish().expect("zip finished");
}

/// A Modrinth index that names a pack and nothing else, with its overrides folder.
const MRPACK_INDEX: &str =
    r#"{"name": "Fabulously Optimized", "dependencies": {"minecraft": "1.20.1", "fabric-loader": "0.16.7"}, "files": [], "overrides": "overrides"}"#;

/// A CurseForge manifest that names a pack and nothing else, with its overrides folder.
const CURSEFORGE_MANIFEST: &str = r#"{"name": "Better MC", "minecraft": {"version": "1.20.1", "modLoaders": [{"id": "fabric-0.16.7", "primary": true}]}, "files": [], "overrides": "overrides"}"#;

/// Writes a Lodestone package at `path` that bundles one file, `config/a.toml`, and links nothing.
fn package_at(path: &Path) {
    let source = path.with_file_name("a.toml");
    std::fs::write(&source, "x=1").expect("source written");
    let entries = vec![Entry {
        path: "config/a.toml".to_string(),
        source,
        size: 3,
        digests: None,
        links: Links::default(),
    }];
    let metadata = Metadata {
        name: "Enhanced Worlds".to_string(),
        version: "1.0.0".to_string(),
        description: String::new(),
        minecraft: "1.21.1".to_string(),
        loader: "fabric".to_string(),
        loader_version: Some("0.16.10".to_string()),
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
    std::fs::write(path, bytes).expect("package saved");
}

#[tokio::test]
async fn inspect_reads_the_name_and_format_of_each_modpack_file() {
    let dir = scratch("inspect-modpacks");
    let modrinth = dir.join("fabulous.mrpack");
    zip_at(&modrinth, &[("modrinth.index.json", MRPACK_INDEX)]);
    let curseforge = dir.join("better.zip");
    zip_at(&curseforge, &[("manifest.json", CURSEFORGE_MANIFEST)]);
    let renamed = dir.join("better.mcpack");
    zip_at(&renamed, &[("manifest.json", CURSEFORGE_MANIFEST)]);

    let picked = inspect(modrinth.clone(), false).await.expect("an mrpack is accepted");
    assert_eq!(picked.name, "Fabulously Optimized");
    assert_eq!(picked.format, ImportFormat::Mrpack);
    assert_eq!(picked.bytes, std::fs::metadata(&modrinth).expect("file exists").len());
    assert_eq!(
        inspect(curseforge, false).await.expect("a zip pack is accepted").format,
        ImportFormat::CurseForge
    );
    assert_eq!(inspect(renamed, false).await.expect("a .mcpack zip is accepted").name, "Better MC");
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn inspect_refuses_a_zip_that_is_no_modpack_and_a_file_for_the_wrong_tab() {
    let dir = scratch("inspect-refused");
    let plain = dir.join("notes.zip");
    zip_at(&plain, &[("readme.txt", "not a pack")]);
    let backup = dir.join("world.lodestone");
    package_at(&backup);
    let text = dir.join("notes.txt");
    std::fs::write(&text, "x").expect("text written");

    assert_eq!(inspect(plain, false).await, Err(lang::t("import.error.not_a_modpack").to_string()));
    assert_eq!(inspect(backup.clone(), false).await, Err(lang::t("import.error.use_backup").to_string()));
    assert_eq!(inspect(text, false).await, Err(lang::t("import.error.file_type").to_string()));
    let zip = dir.join("pack.zip");
    zip_at(&zip, &[("manifest.json", CURSEFORGE_MANIFEST)]);
    assert_eq!(inspect(zip, true).await, Err(lang::t("import.error.backup_type").to_string()));
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn inspect_reads_the_name_of_a_lodestone_package() {
    let dir = scratch("inspect-backup");
    let backup = dir.join("worlds.lodestone");
    package_at(&backup);
    let picked = inspect(backup.clone(), true).await.expect("a package is accepted");
    assert_eq!(picked.name, "Enhanced Worlds");
    assert_eq!(picked.format, ImportFormat::Lodestone);
    assert_eq!(picked.path, backup);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_source_over_four_gibibytes_is_refused_and_one_at_the_limit_is_taken() {
    assert_eq!(check_size(MAX_BYTES), Ok(MAX_BYTES));
    assert_eq!(check_size(MAX_BYTES + 1), Err(lang::t("import.error.too_large").to_string()));
}

#[tokio::test]
async fn a_modpack_file_queues_an_install_that_lays_its_overrides_and_records_the_import() {
    let dir = scratch("from-file");
    let pack = dir.join("better.zip");
    zip_at(&pack, &[("manifest.json", CURSEFORGE_MANIFEST), ("overrides/config/a.toml", "x=1")]);
    let paths = LauncherPaths::new(dir.join("launcher"));

    let prepared = from_file(pack.clone(), paths, Vec::new()).await.expect("the pack imports");
    assert_eq!(prepared.request.name, "Better MC");
    assert_eq!(prepared.request.loader, Loader::Fabric);
    assert_eq!(prepared.record.name, "Better MC");
    assert_eq!(prepared.record.format, ImportFormat::CurseForge);
    assert_eq!(prepared.record.origin, ImportOrigin::File);
    assert_eq!(prepared.record.source, path_string(&pack));
    assert_eq!(prepared.record.bytes, std::fs::metadata(&pack).expect("file exists").len());
    let payload = prepared.request.modpack.expect("the install carries the pack");
    assert!(
        payload
            .unpacks
            .iter()
            .any(|unpack| matches!(&unpack.origin, Origin::Local(path) if *path == pack) && unpack.folder == "overrides"),
        "the pack's overrides are laid over the instance"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn a_backup_queues_a_new_instance_under_a_fresh_id_and_lays_its_bundled_files() {
    let dir = scratch("from-backup");
    let backup = dir.join("worlds.lodestone");
    package_at(&backup);
    let paths = LauncherPaths::new(dir.join("launcher"));

    let prepared = from_backup(backup.clone(), paths, Vec::new()).await.expect("the package restores");
    assert_ne!(prepared.request.id, "pack-id", "a restore never takes the package's id");
    assert_eq!(prepared.request.name, "Enhanced Worlds");
    assert_eq!(prepared.request.loader, Loader::Fabric);
    assert_eq!(prepared.record.format, ImportFormat::Lodestone);
    assert_eq!(prepared.record.origin, ImportOrigin::File);
    let payload = prepared.request.modpack.expect("the install carries the package");
    assert!(
        payload
            .unpacks
            .iter()
            .any(|unpack| matches!(&unpack.origin, Origin::Package(path) if *path == backup)),
        "the package's overrides are laid over the instance"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn a_text_that_is_no_curseforge_page_is_refused_before_anything_is_fetched() {
    let paths = LauncherPaths::new(scratch("from-url").join("launcher"));
    let refused = from_url("https://www.example.com/minecraft/modpacks/better-mc".to_string(), paths, Vec::new()).await;
    assert_eq!(refused.err(), Some(lang::t("import.url.invalid").to_string()));
}
