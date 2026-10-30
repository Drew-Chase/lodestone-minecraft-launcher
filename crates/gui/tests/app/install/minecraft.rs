use std::path::PathBuf;

use super::*;

/// A fresh folder for one test, named after the test so parallel runs do not share it.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lodestone-minecraft-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the scratch folder creates");
    dir
}

#[test]
fn multi_file_reports_map_onto_their_stage() {
    let report = MultiDownloadProgress {
        bytes_to_download: 100,
        bytes_downloaded: 25,
        bytes_per_second: 0,
        files_downloaded: 3,
        files_total: 12,
        file_names_downloaded: Vec::new(),
        file_names: Vec::new(),
    };

    let progress = multi_progress(Stage::Assets)(report);

    assert_eq!(progress.stage, Stage::Assets);
    assert_eq!(progress.fraction, Some(0.25));
    assert_eq!(progress.detail, "3/12 files");
}

#[test]
fn the_version_json_lands_under_versions() {
    let root = scratch("version-json");
    write_version_json(&root, "1.21.4", r#"{"id":"1.21.4"}"#).expect("the version JSON writes");

    let text = std::fs::read_to_string(root.join("versions/1.21.4/1.21.4.json")).expect("the file exists");
    assert_eq!(text, r#"{"id":"1.21.4"}"#);
    let _ = std::fs::remove_dir_all(&root);
}
