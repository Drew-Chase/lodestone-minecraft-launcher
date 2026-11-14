//! The data folder's moves: the checks on a target, the moves of whole entries, the copy across drives and the rollback.

use super::*;

/// A scratch folder with a name unique to this test, emptied before use.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lodestone-data-folder-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("the scratch folder");
    dir
}

#[test]
fn a_target_must_be_missing_or_empty_and_not_nested() {
    let from = scratch("targets-from");
    assert_eq!(check_target(&from, &from.join("inside")), Err(Refusal::Nested));
    assert_eq!(check_target(&from, &from), Err(Refusal::Nested));
    assert_eq!(
        check_target(&from.join("inner"), &from),
        Err(Refusal::Nested),
        "a folder holding the data is nested too"
    );

    let busy = scratch("targets-busy");
    fs::write(busy.join("notes.txt"), b"hello").expect("a file");
    assert_eq!(check_target(&from, &busy), Err(Refusal::NotEmpty));

    let empty = scratch("targets-empty");
    assert_eq!(check_target(&from, &empty), Ok(()));

    let missing = scratch("targets-missing");
    fs::remove_dir_all(&missing).expect("the folder goes");
    assert_eq!(check_target(&from, &missing), Ok(()));
}

#[test]
fn every_entry_moves_and_the_old_folder_goes() {
    let from = scratch("move-from");
    fs::create_dir_all(from.join("instances/alpha/mods")).expect("an instance");
    fs::write(from.join("instances/alpha/mods/sodium.jar"), b"jar").expect("a mod");
    fs::create_dir_all(from.join("meta/java")).expect("a runtime folder");
    fs::write(from.join("skins.txt"), b"skin").expect("a top-level file");

    let to = scratch("move-to");
    fs::remove_dir_all(&to).expect("the folder goes");
    move_root(&from, &to).expect("the data moves");

    assert_eq!(fs::read(to.join("instances/alpha/mods/sodium.jar")).expect("the mod"), b"jar");
    assert!(to.join("meta/java").is_dir());
    assert_eq!(fs::read(to.join("skins.txt")).expect("the file"), b"skin");
    assert!(!from.exists(), "the old folder is removed once it is empty");
    let _ = fs::remove_dir_all(&to);
}

#[test]
fn a_refused_target_leaves_the_data_where_it_is() {
    let from = scratch("refused-from");
    fs::write(from.join("keep.txt"), b"data").expect("a file");
    let busy = scratch("refused-busy");
    fs::write(busy.join("other.txt"), b"other").expect("a file");

    assert!(move_root(&from, &busy).is_err());
    assert_eq!(fs::read(from.join("keep.txt")).expect("the data"), b"data");
    let _ = fs::remove_dir_all(&from);
}

#[test]
fn entries_put_back_return_to_the_old_folder() {
    let from = scratch("restore-from");
    let to = scratch("restore-to");
    fs::write(from.join("a.txt"), b"a").expect("a file");
    fs::write(from.join("b.txt"), b"b").expect("a file");
    move_entry(&from.join("a.txt"), &to.join("a.txt")).expect("a moves");

    let stuck = restore(&from, &to, &[OsString::from("a.txt")]);
    assert!(stuck.is_empty());
    assert_eq!(fs::read(from.join("a.txt")).expect("the entry is back"), b"a");
    assert!(!to.join("a.txt").exists());
    let _ = fs::remove_dir_all(&from);
    let _ = fs::remove_dir_all(&to);
}

#[cfg(unix)]
#[test]
fn a_copy_keeps_links_as_links() {
    let source = scratch("copy-source");
    fs::create_dir_all(source.join("bin")).expect("a runtime folder");
    fs::write(source.join("lib.so"), b"lib").expect("a library");
    std::os::unix::fs::symlink("../lib.so", source.join("bin/link")).expect("a link");

    let target = scratch("copy-target");
    fs::remove_dir_all(&target).expect("the folder goes");
    copy_tree(&source, &target).expect("the copy");

    assert_eq!(fs::read_link(target.join("bin/link")).expect("the link"), PathBuf::from("../lib.so"));
    assert_eq!(fs::read(target.join("lib.so")).expect("the library"), b"lib");
    let _ = fs::remove_dir_all(&source);
    let _ = fs::remove_dir_all(&target);
}

#[test]
fn instances_may_not_sit_inside_the_data_folder_unless_they_take_the_default_place() {
    let root = PathBuf::from("/work/data");
    let from = PathBuf::from("/games/instances");
    assert_eq!(check_instances_target(&from, &root, &root.join("custom")), Err(Refusal::InsideData));
    assert_eq!(
        check_instances_target(&from, &root, &root.join("instances")),
        Ok(()),
        "the default folder inside the data folder is fine"
    );
    assert_eq!(check_instances_target(&from, &root, &PathBuf::from("/games/other")), Ok(()));
}

#[test]
fn instances_may_not_hold_the_data_folder() {
    let root = PathBuf::from("/work/data");
    let from = root.join("instances");
    assert_eq!(check_instances_target(&from, &root, &PathBuf::from("/work")), Err(Refusal::Nested));
}

#[test]
fn the_data_folder_may_not_nest_with_the_instances() {
    let from = PathBuf::from("/work/data");
    let instances = PathBuf::from("/games/instances");
    assert_eq!(
        check_shared_target(&from, &instances, &PathBuf::from("/games")),
        Err(Refusal::Nested),
        "a folder holding the instances"
    );
    assert_eq!(
        check_shared_target(&from, &instances, &instances.join("inner")),
        Err(Refusal::Nested),
        "a folder inside the instances"
    );
    assert_eq!(check_shared_target(&from, &instances, &PathBuf::from("/mnt/data")), Ok(()));
}

#[test]
fn a_folder_choice_that_cannot_be_saved_moves_the_files_back() {
    let from = scratch("choice-from");
    fs::write(from.join("keep.txt"), b"keep").expect("a file");
    let to = scratch("choice-to");
    fs::remove_dir_all(&to).expect("the folder goes");

    let result = move_then_save(&from, &to, Some(to.clone()), |_| Err("the preferences are read only".to_string()));
    assert!(result.is_err());
    assert_eq!(fs::read(from.join("keep.txt")).expect("the data is back"), b"keep");
    assert!(!to.join("keep.txt").exists());
    let _ = fs::remove_dir_all(&from);
    let _ = fs::remove_dir_all(&to);
}

#[test]
fn a_folder_choice_that_is_saved_leaves_the_files_in_the_new_folder() {
    let from = scratch("saved-from");
    fs::write(from.join("keep.txt"), b"keep").expect("a file");
    let to = scratch("saved-to");
    fs::remove_dir_all(&to).expect("the folder goes");

    let mut saved_as = None;
    let result = move_then_save(&from, &to, None, |folder| {
        saved_as = Some(folder);
        Ok(())
    });
    assert!(result.is_ok());
    assert_eq!(saved_as, Some(None), "the default folder is what the preferences save");
    assert_eq!(fs::read(to.join("keep.txt")).expect("the file moved"), b"keep");
    assert!(!from.exists());
    let _ = fs::remove_dir_all(&to);
}
