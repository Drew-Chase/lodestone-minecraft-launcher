use super::*;
use std::path::PathBuf;

/// A scratch folder for one test, emptied first.
fn scratch(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("lodestone-packs-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("scratch folder");
    path
}

#[test]
fn zips_and_folders_are_packs_and_other_files_are_not() {
    let dir = scratch("kinds");
    std::fs::write(dir.join("Fresh.zip"), b"abc").unwrap();
    std::fs::write(dir.join("Loose.txt"), b"x").unwrap();
    std::fs::create_dir_all(dir.join("Folder Pack")).unwrap();
    let packs = scan(&dir);
    let names: Vec<&str> = packs.iter().map(|pack| pack.name.as_str()).collect();
    assert_eq!(names, ["Folder Pack", "Fresh"]);
    assert!(packs[0].is_dir && packs[0].size.is_none());
    assert_eq!(packs[1].size, Some(3));
    assert_eq!(packs[1].file_name, "Fresh.zip");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_zip_extension_in_any_case_is_still_a_pack() {
    let dir = scratch("case");
    std::fs::write(dir.join("Upper.ZIP"), b"x").unwrap();
    let names: Vec<String> = scan(&dir).into_iter().map(|pack| pack.name).collect();
    assert_eq!(names, ["Upper"]);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_shared_zip_and_an_installed_one_are_named_alike() {
    assert_eq!(name_of("Fresh Animations.zip"), "Fresh Animations");
    assert_eq!(name_of("Upper.ZIP"), "Upper");
    assert_eq!(name_of("Folder Pack"), "Folder Pack", "a folder keeps its name");
    assert_eq!(name_of("archive.tar.zip"), "archive.tar");
}

#[test]
fn a_folder_that_is_not_there_holds_no_packs() {
    assert!(scan(&scratch("missing").join("absent")).is_empty());
}
