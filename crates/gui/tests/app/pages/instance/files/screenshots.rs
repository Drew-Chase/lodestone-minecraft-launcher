//! Tests for `pages/instance/files/screenshots.rs`, kept out of the source file.
//! They are compiled as the `tests` child module of `screenshots`.

use std::fs;
use std::path::{Path, PathBuf};

use super::*;

/// A fresh folder under the system temp directory, named for the test.
fn scratch(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("lodestone-shots-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).expect("scratch folder");
    path
}

/// Writes `contents` to `path`, creating the folders above it.
fn write(path: &Path, contents: &[u8]) {
    fs::create_dir_all(path.parent().expect("a file has a parent")).expect("parent folder");
    fs::write(path, contents).expect("file writes");
}

#[test]
fn only_pngs_are_listed() {
    let root = scratch("png-only");
    write(&root.join("a.png"), b"");
    write(&root.join("B.PNG"), b"");
    write(&root.join("c.jpg"), b"");
    write(&root.join("d.txt"), b"");

    let mut names: Vec<String> = scan(&root).into_iter().map(|shot| shot.file_name).collect();
    names.sort();
    assert_eq!(names, ["B.PNG", "a.png"]);
    fs::remove_dir_all(&root).ok();
}

#[test]
fn sizes_read_as_the_launcher_formats_them() {
    let root = scratch("size");
    write(&root.join("shot.png"), &vec![0; 2_202_009]);
    let shot = scan(&root).remove(0);
    assert_eq!(shot.size_label, "2.1 MB");
    assert_eq!(shot.path, root.join("shot.png"));
    fs::remove_dir_all(&root).ok();
}

#[test]
fn a_missing_folder_has_no_captures() {
    assert!(scan(Path::new("/definitely/not/a/screenshots/folder")).is_empty());
}
