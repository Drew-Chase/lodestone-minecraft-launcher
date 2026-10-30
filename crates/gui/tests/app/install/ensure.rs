use std::path::{Path, PathBuf};
use std::time::Duration;

use super::*;

/// A fresh folder for one test, named after the test so parallel runs do not share it.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lodestone-ensure-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the scratch folder creates");
    dir
}

#[test]
fn asset_objects_sit_under_their_two_character_prefix() {
    let paths = LauncherPaths::new("/data/lodestone");
    let text = r#"{"objects":{
        "minecraft/sounds/a.ogg":{"hash":"abcdef0123456789abcdef0123456789abcdef01","size":42},
        "broken":{"hash":"a","size":1}}}"#;
    let items = object_items(&paths, "17", text).expect("the index parses");

    assert_eq!(items.len(), 1, "a hash too short to have a prefix is skipped");
    let item = &items[0];
    assert_eq!(
        item.url.as_deref(),
        Some("https://resources.download.minecraft.net/ab/abcdef0123456789abcdef0123456789abcdef01")
    );
    assert_eq!(
        item.path,
        Path::new("/data/lodestone/meta/assets/objects/ab/abcdef0123456789abcdef0123456789abcdef01")
    );
    assert_eq!(item.size, Some(42));
    assert_eq!(item.sha1.as_deref(), Some("abcdef0123456789abcdef0123456789abcdef01"));
}

#[test]
fn a_written_file_appears_only_once_it_is_whole() {
    let root = scratch("write");
    let path = root.join("nested/17.json");
    write_file(&path, br#"{"objects":{}}"#).expect("the file writes");

    assert_eq!(std::fs::read_to_string(&path).expect("the file exists"), r#"{"objects":{}}"#);
    assert!(!root.join("nested/17.json.part").exists(), "the partial file is renamed away");
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn the_same_key_waits_for_its_first_holder() {
    let first = lock("test:serial-lock".to_string()).await;
    let waiting = tokio::time::timeout(Duration::from_millis(50), lock("test:serial-lock".to_string())).await;
    assert!(waiting.is_err(), "a second holder waits while the first holds the key");
    drop(first);
    let _second = lock("test:serial-lock".to_string()).await;
}

#[tokio::test]
async fn a_launch_waits_for_another_extraction_of_the_same_folder() {
    let dir = scratch("natives-wait");
    let holder = lock(natives_key(&dir)).await;
    let waiting = tokio::time::timeout(Duration::from_millis(50), natives(dir.clone(), Vec::new())).await;
    assert!(waiting.is_err(), "a second extraction waits while the first holds the folder");
    assert!(!dir.join(COMPLETE).exists(), "nothing is written while the folder is held");

    drop(holder);
    natives(dir.clone(), Vec::new())
        .await
        .expect("the folder extracts once the holder lets go");
    assert!(dir.join(COMPLETE).is_file(), "the marker is written when extraction finishes");
    let _ = std::fs::remove_dir_all(&dir);
}
