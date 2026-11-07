use lodestone_platforms::Modpack;

use super::*;

fn pack(name: &str, loader: Option<&'static str>) -> Modpack {
    Modpack {
        name: name.to_string(),
        version: None,
        minecraft: "1.20.1".to_string(),
        loader,
        loader_version: None,
        files: Vec::new(),
        overrides: None,
        overrides_skip: Vec::new(),
        archives: Vec::new(),
    }
}

fn paths() -> LauncherPaths {
    let root = std::env::temp_dir().join(format!("lodestone-pack-request-{}", std::process::id()));
    LauncherPaths::new(&root)
}

#[tokio::test]
async fn a_pack_read_from_a_file_records_no_page() {
    let install = request(&paths(), &[], pack("Better MC", Some("fabric")), None, None, None, None)
        .await
        .expect("a pack with no files needs no network");
    assert_eq!(install.name, "Better MC");
    assert_eq!(install.loader, Loader::Fabric);
    assert_eq!(install.source_url, None);
    assert_eq!(install.source_platform, None);
    assert_eq!(install.source_project_id, None);
    assert!(matches!(install.icon, IconSource::Composed { .. }));
}

#[tokio::test]
async fn a_pack_taken_from_a_page_records_the_page_and_its_project() {
    let page = Page {
        source: Source::CurseForge,
        project_id: "123".to_string(),
        url: Some("https://www.curseforge.com/minecraft/modpacks/better-mc".to_string()),
        icon_url: None,
    };
    let install = request(&paths(), &[], pack("Better MC", None), None, Some(Source::CurseForge), Some(page), None)
        .await
        .expect("a pack with no files needs no network");
    assert_eq!(install.source_platform.as_deref(), Some("curseforge"));
    assert_eq!(install.source_project_id.as_deref(), Some("123"));
    assert_eq!(
        install.source_url.as_deref(),
        Some("https://www.curseforge.com/minecraft/modpacks/better-mc")
    );
    assert_eq!(install.loader, Loader::Vanilla);
}

#[tokio::test]
async fn the_folder_avoids_the_names_already_taken() {
    let base = naming::clean_directory_name("Better MC");
    let install = request(&paths(), std::slice::from_ref(&base), pack("Better MC", None), None, None, None, None)
        .await
        .expect("a pack with no files needs no network");
    assert_ne!(install.directory_name, base);
}

/// A scratch folder unique to this test run, holding the archive a test writes.
fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lodestone-pack-bundled-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch folder created");
    dir
}

/// Writes an archive at `path` holding each `(name, text)` entry, in order.
fn write_archive(path: &Path, entries: &[(&str, &str)]) {
    let file = std::fs::File::create(path).expect("archive created");
    let mut archive = zip::ZipWriter::new(file);
    for (name, text) in entries {
        archive
            .start_file(*name, zip::write::SimpleFileOptions::default())
            .expect("entry started");
        std::io::Write::write_all(&mut archive, text.as_bytes()).expect("entry written");
    }
    archive.finish().expect("archive finished");
}

#[test]
fn a_modpacks_overrides_name_each_pack_they_bundle_with_the_hash_of_its_bytes() {
    let dir = scratch("overrides");
    let archive = dir.join("pack.zip");
    write_archive(
        &archive,
        &[
            ("overrides/resourcepacks/Fresh.zip", "abc"),
            ("overrides/config/options.txt", "abc"),
            ("overrides/resourcepacks/nested/deep.zip", "abc"),
            ("overrides/shaderpacks/Shade.zip", "abc"),
            ("elsewhere/resourcepacks/Outside.zip", "abc"),
        ],
    );
    let abc = "a9993e364706816aba3e25717850c26c9cd0d89d";
    assert_eq!(
        read_bundled_packs(&archive, "overrides", false),
        vec![
            BundledPack {
                path: "resourcepacks/Fresh.zip".to_string(),
                sha1: abc.to_string(),
                fingerprint: None,
            },
            BundledPack {
                path: "shaderpacks/Shade.zip".to_string(),
                sha1: abc.to_string(),
                fingerprint: None,
            },
        ]
    );
    std::fs::remove_dir_all(&dir).expect("scratch folder removed");
}

#[test]
fn a_bundled_pack_carries_the_curseforge_fingerprint_of_its_bytes_when_one_is_asked_for() {
    let dir = scratch("fingerprints");
    let archive = dir.join("pack.zip");
    // The fingerprint leaves out the whitespace, so the first pack fingerprints as "abcd" does.
    write_archive(
        &archive,
        &[
            ("overrides/resourcepacks/Spaced.zip", "a b\tc\r\nd"),
            ("overrides/shaderpacks/Plain.zip", "abc"),
        ],
    );
    let fingerprints: Vec<Option<u32>> = read_bundled_packs(&archive, "overrides", true)
        .iter()
        .map(|pack| pack.fingerprint)
        .collect();
    assert_eq!(fingerprints, vec![Some(3_376_380_438), Some(1_621_425_345)]);
    std::fs::remove_dir_all(&dir).expect("scratch folder removed");
}

#[test]
fn an_archive_that_cannot_be_opened_bundles_no_packs() {
    assert!(read_bundled_packs(Path::new("/nonexistent/lodestone/pack.zip"), "overrides", true).is_empty());
}

#[tokio::test]
async fn a_pack_with_no_archive_or_no_overrides_folder_has_no_bundled_packs() {
    let dir = scratch("nothing");
    let archive = dir.join("pack.zip");
    write_archive(&archive, &[("overrides/resourcepacks/Fresh.zip", "abc")]);
    assert!(
        bundled_packs(None, Some("overrides".to_string()), true).await.is_empty(),
        "no archive is read"
    );
    assert!(
        bundled_packs(Some(archive), None, true).await.is_empty(),
        "no overrides folder names the packs"
    );
    std::fs::remove_dir_all(&dir).expect("scratch folder removed");
}
