use std::path::PathBuf;

use super::*;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lodestone-fs-tree-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("the scratch folder");
    dir
}

/// A folder `config` holding a file and a nested folder with a file.
fn sample(root: &Path) -> PathBuf {
    let source = root.join("config");
    fs::create_dir_all(source.join("nested")).expect("the folders");
    fs::write(source.join("a.toml"), "a = 1").expect("a file");
    fs::write(source.join("nested").join("b.json"), "{}").expect("a nested file");
    source
}

#[test]
fn copy_tree_copies_a_folder_with_everything_in_it() {
    let root = scratch("copy");
    let source = sample(&root);
    let target = root.join("copy");
    copy_tree(&source, &target).expect("the copy");
    assert_eq!(fs::read_to_string(target.join("a.toml")).expect("a"), "a = 1");
    assert_eq!(fs::read_to_string(target.join("nested").join("b.json")).expect("b"), "{}");
    assert!(source.join("a.toml").is_file(), "the source is left as it was");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn copy_tree_refuses_a_target_that_exists() {
    let root = scratch("exists");
    let source = sample(&root);
    let target = root.join("taken");
    fs::create_dir(&target).expect("the taken folder");
    assert!(copy_tree(&source, &target).is_err());
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn move_tree_moves_a_folder_and_leaves_nothing_behind() {
    let root = scratch("move");
    let source = sample(&root);
    let target = root.join("moved");
    move_tree(&source, &target).expect("the move");
    assert!(!source.exists());
    assert_eq!(fs::read_to_string(target.join("nested").join("b.json")).expect("b"), "{}");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn remove_tree_removes_a_file_and_a_folder() {
    let root = scratch("remove");
    let source = sample(&root);
    remove_tree(&source.join("a.toml")).expect("the file");
    assert!(!source.join("a.toml").exists());
    remove_tree(&source).expect("the folder");
    assert!(!source.exists());
    let _ = fs::remove_dir_all(&root);
}

#[cfg(unix)]
#[test]
fn a_link_is_copied_and_removed_as_a_link() {
    let root = scratch("link");
    let source = sample(&root);
    let link = root.join("link");
    std::os::unix::fs::symlink(&source, &link).expect("the link");

    let copy = root.join("link-copy");
    copy_tree(&link, &copy).expect("the copy");
    assert!(fs::symlink_metadata(&copy).expect("the copy's kind").file_type().is_symlink());

    remove_tree(&copy).expect("the link goes");
    remove_tree(&link).expect("the link goes");
    assert!(source.join("a.toml").is_file(), "what the link pointed at is untouched");
    let _ = fs::remove_dir_all(&root);
}
