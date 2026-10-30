use super::*;

/// A folder under the system temp directory, unique to `name` and this process.
fn scratch(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("lodestone-{name}-{}", std::process::id()))
}

#[test]
fn removing_a_folder_removes_everything_in_it() {
    let folder = scratch("remove-tree");
    std::fs::create_dir_all(folder.join("mods")).expect("folder creates");
    std::fs::write(folder.join("mods").join("a.jar"), b"x").expect("file writes");

    assert_eq!(remove_folder(&folder), Ok(()));
    assert!(!folder.exists());
}

#[test]
fn a_folder_that_is_already_gone_counts_as_removed() {
    let folder = scratch("already-gone");
    let _ = std::fs::remove_dir_all(&folder);

    assert_eq!(remove_folder(&folder), Ok(()));
}
