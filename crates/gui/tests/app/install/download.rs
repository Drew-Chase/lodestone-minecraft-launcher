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
