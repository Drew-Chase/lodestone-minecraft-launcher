//! Scratch folders and files for the tests that read and write disk.

use std::fs;
use std::path::{Path, PathBuf};

/// A fresh, empty folder under the system temp directory, named for the test that asked for it. Each test uses its own name.
pub fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lodestone-snapshots-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("the scratch folder can be made");
    dir
}

/// Writes `contents` to `relative` under `root`, making the folders it needs, and returns the file's path.
pub fn put(root: &Path, relative: &str, contents: &[u8]) -> PathBuf {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("a file has a parent")).expect("the folders can be made");
    fs::write(&path, contents).expect("the file can be written");
    path
}
