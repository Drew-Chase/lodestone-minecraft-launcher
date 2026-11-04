use super::*;
use std::fs;

/// The SHA-256 of the bytes `abc`.
const ABC_SHA256: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

/// A fresh folder under the system temp directory, named for the test.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lodestone-updater-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("creating the scratch folder");
    dir
}

#[test]
fn a_file_is_checked_against_its_hash() {
    let dir = scratch("verify");
    let path = dir.join("file.bin");
    fs::write(&path, b"abc").expect("writing the file");
    assert!(verify_file(&path, ABC_SHA256).expect("hashing"));
    assert!(verify_file(&path, &ABC_SHA256.to_uppercase()).expect("hashing"));
    assert!(!verify_file(&path, &"0".repeat(64)).expect("hashing"));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_release_without_an_installer_is_refused_before_any_download() {
    let client = crate::client("lodestone-updater-tests").expect("a client");
    let release = Release {
        tag: "v0.2.0".to_string(),
        html_url: String::new(),
        body: String::new(),
        assets: Vec::new(),
    };
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().expect("a runtime");
    let result = runtime.block_on(fetch_installer(&client, &release, &scratch("refuse")));
    assert!(matches!(result, Err(Error::MissingAsset(_))));
}
