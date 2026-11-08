use std::path::PathBuf;

use super::*;

/// A fresh folder for one test, named after the test so parallel runs do not share it.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lodestone-download-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the scratch folder creates");
    dir
}

fn item(path: PathBuf, size: Option<u64>) -> Item {
    Item {
        url: Some("https://files.example/object".to_string()),
        path,
        sha1: None,
        size,
    }
}

#[tokio::test]
async fn a_file_counts_as_present_only_at_its_expected_size() {
    let root = scratch("present");
    std::fs::write(root.join("exact.jar"), b"12345").expect("the file writes");
    std::fs::write(root.join("short.jar"), b"123").expect("the file writes");
    std::fs::write(root.join("unsized.jar"), b"anything").expect("the file writes");
    std::fs::write(root.join("empty.jar"), b"").expect("the file writes");

    let items = vec![
        item(root.join("exact.jar"), Some(5)),
        item(root.join("short.jar"), Some(5)),
        item(root.join("absent.jar"), Some(5)),
        item(root.join("unsized.jar"), None),
        item(root.join("empty.jar"), None),
    ];
    let split = missing(items).await.expect("the check runs");

    assert_eq!(split.present, 2, "the exact file and the non-empty unsized file are on disk");
    let fetch: Vec<PathBuf> = split.missing.iter().map(|item| item.path.clone()).collect();
    assert_eq!(fetch, vec![root.join("short.jar"), root.join("absent.jar"), root.join("empty.jar")]);
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn an_item_with_no_url_cannot_be_fetched() {
    let items = vec![Item {
        url: None,
        path: PathBuf::from("/nowhere/installer-written.jar"),
        sha1: None,
        size: None,
    }];
    let (reports, _rx) = mpsc::channel::<PartReport>(1);
    let client = reqwest::Client::new();
    let error = fetch(&client, items, Part::Libraries, 0, 1, &reports)
        .await
        .expect_err("nothing to download from");
    assert!(error.contains("installer-written.jar"), "the error names the file: {error}");
}

#[tokio::test]
async fn with_verification_off_nothing_is_looked_at_and_nothing_is_fetched() {
    let root = scratch("unverified");
    std::fs::write(root.join("short.jar"), b"1").expect("the file writes");
    let items = vec![item(root.join("short.jar"), Some(5)), item(root.join("absent.jar"), Some(5))];

    let split = missing_when(items, false).await.expect("the check runs");

    assert_eq!(split.present, 2, "every file is taken as present");
    assert!(split.missing.is_empty(), "a file that is absent is not fetched either");
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn a_file_of_the_right_size_with_the_wrong_hash_is_fetched_again() {
    let dir = scratch("hash");
    std::fs::write(dir.join("a.bin"), b"hello world").expect("the file writes");
    // The SHA-1 of "hello world" is what the file must hash to. A file of the same size with other bytes is not present.
    let right = Item {
        url: Some("https://files.example/object".to_string()),
        path: dir.join("a.bin"),
        sha1: Some("2aae6c35c94fcfb415dbe95f408b9ce91ee846ed".to_string()),
        size: Some(11),
    };
    let wrong = Item {
        sha1: Some("0000000000000000000000000000000000000000".to_string()),
        ..right.clone()
    };
    assert_eq!(missing(vec![right]).await.expect("the check runs").present, 1);
    let split = missing(vec![wrong]).await.expect("the check runs");
    assert_eq!((split.present, split.missing.len()), (0, 1));
}
