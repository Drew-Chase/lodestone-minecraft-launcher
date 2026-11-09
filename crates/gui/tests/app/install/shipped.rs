use std::path::PathBuf;

use lodestone_platforms::ModpackFile;

use super::*;

/// A scratch folder unique to this run and `name`, emptied first.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lodestone-shipped-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch folder");
    dir
}

/// Writes `bytes` to `path` under `dir`, making its folders.
fn put(dir: &Path, path: &str, bytes: &[u8]) {
    let full = dir.join(path);
    std::fs::create_dir_all(full.parent().expect("a parent folder")).expect("folders");
    std::fs::write(full, bytes).expect("file written");
}

/// A file a release ships, with the hash it gives.
fn shipped_file(path: &str, sha1: Option<&str>) -> ModpackFile {
    ModpackFile {
        path: path.to_string(),
        url: String::new(),
        sha1: sha1.map(str::to_string),
        size: None,
        project_id: None,
        file_id: None,
    }
}

/// The payload of a release that ships `files` and nothing else.
fn payload(files: Vec<ModpackFile>) -> Payload {
    Payload {
        files,
        unpacks: Vec::new(),
        sources: Vec::new(),
        packs: Vec::new(),
    }
}

/// The SHA-1 of the bytes `abc`, as lowercase hex.
const ABC: &str = "a9993e364706816aba3e25717850c26c9cd0d89d";

#[test]
fn user_folders_and_files_are_recognised() {
    assert!(user_folder("saves/World/level.dat"));
    assert!(user_folder("resourcepacks/Pack.zip"));
    assert!(!user_folder("config/saves.toml"));
    assert!(user_file("options.txt"));
    assert!(user_file(".lodestone/icon.png"));
    assert!(!user_file("config/options.txt"));
}

#[test]
fn only_text_without_a_nul_byte_is_text() {
    assert!(is_text(b"key=value\n"));
    assert!(is_text(&[]));
    assert!(!is_text(b"\x89PNG\r\n\x00\x00"));
}

#[test]
fn a_pack_jar_and_a_user_file_keep_no_copy() {
    assert!(!keeps_copy("mods/sodium.jar"));
    assert!(!keeps_copy("saves/World/level.dat"));
    assert!(!keeps_copy("options.txt"));
    assert!(keeps_copy("config/sodium.json"));
}

#[tokio::test]
async fn a_hash_the_release_gives_is_kept_and_a_missing_one_is_read_from_disk() {
    let dir = scratch("hashes");
    put(&dir, "config/a.toml", b"abc");
    put(&dir, "config/b.toml", b"abc");
    let release = payload(vec![shipped_file("config/a.toml", Some("ABCDEF")), shipped_file("config/b.toml", None)]);
    let files = record(&dir, "inst", Some(&release), vec!["config/b.toml".to_string()])
        .await
        .expect("the files are recorded");
    let hash = |path: &str| files.iter().find(|file| file.path == path).and_then(|file| file.sha1.clone());
    assert_eq!(hash("config/a.toml").as_deref(), Some("abcdef"), "the release's hash, in lowercase");
    assert_eq!(hash("config/b.toml").as_deref(), Some(ABC), "the hash read from disk");
    assert!(files.iter().all(|file| file.instance == "inst"));
}

#[tokio::test]
async fn a_file_listed_twice_is_recorded_once() {
    let dir = scratch("twice");
    put(&dir, "mods/m.jar", b"abc");
    let release = payload(vec![shipped_file("mods/m.jar", Some(ABC))]);
    let files = record(&dir, "inst", Some(&release), vec!["mods/m.jar".to_string()])
        .await
        .expect("the files are recorded");
    assert_eq!(files.len(), 1);
}

#[tokio::test]
async fn only_small_text_outside_the_mods_and_the_user_files_gets_a_copy() {
    let dir = scratch("copies");
    let base = scratch("copies-base");
    put(&dir, "config/a.toml", b"key = 1\n");
    put(&dir, "mods/m.jar", b"PK\x00\x00");
    put(&dir, "config/icon.bin", b"\x00\x01\x02");
    put(&dir, "saves/World/level.dat", b"x");
    put(&dir, "config/big.txt", &vec![b'a'; (COPY_LIMIT + 1) as usize]);
    let files: Vec<PackFile> = [
        "config/a.toml",
        "mods/m.jar",
        "config/icon.bin",
        "saves/World/level.dat",
        "config/big.txt",
    ]
    .iter()
    .map(|path| PackFile {
        instance: "inst".to_string(),
        path: path.to_string(),
        sha1: None,
    })
    .collect();
    write_base(&base, &dir, &files).await.expect("the copies are written");
    assert_eq!(std::fs::read(base.join("config/a.toml")).expect("the copy exists"), b"key = 1\n");
    assert!(!base.join("mods/m.jar").exists(), "a mod keeps no copy");
    assert!(!base.join("config/icon.bin").exists(), "a binary file keeps no copy");
    assert!(!base.join("saves/World/level.dat").exists(), "a user file keeps no copy");
    assert!(!base.join("config/big.txt").exists(), "a file over the limit keeps no copy");
}
