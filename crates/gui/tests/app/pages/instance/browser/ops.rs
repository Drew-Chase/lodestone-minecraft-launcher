use std::cell::RefCell;

use super::*;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lodestone-browser-ops-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("the scratch folder");
    dir
}

/// A trash that removes what it is given, and remembers it.
fn trash_into(log: &RefCell<Vec<PathBuf>>) -> impl Fn(&Path) -> Result<(), String> + '_ {
    |path| {
        log.borrow_mut().push(path.to_path_buf());
        lodestone_fs::remove_tree(path).map_err(|error| error.to_string())
    }
}

fn no_trash(_: &Path) -> Result<(), String> {
    Err("nothing is replaced in this test".to_string())
}

fn job(sources: &[PathBuf], into: &Path, mode: Mode) -> Transfer {
    Transfer {
        sources: sources.to_vec(),
        into: into.to_path_buf(),
        mode,
    }
}

#[test]
fn a_path_inside_the_instance_is_joined_and_one_that_leaves_it_is_refused() {
    let root = Path::new("/instances/aether");
    assert_eq!(inside(root, ""), Some(root.to_path_buf()));
    assert_eq!(inside(root, "config/sodium.json"), Some(root.join("config").join("sodium.json")));
    assert_eq!(inside(root, "../other"), None);
    assert_eq!(inside(root, "config/../../other"), None);
    assert_eq!(inside(root, "/etc/passwd"), None);
}

#[test]
fn names_a_system_cannot_hold_are_refused() {
    assert_eq!(valid_name("sodium-options.json"), Ok(()));
    assert_eq!(valid_name(".gitignore"), Ok(()));
    assert_eq!(valid_name(""), Err(NameError::Empty));
    assert_eq!(valid_name("a/b"), Err(NameError::Character('/')));
    assert_eq!(valid_name("what?.txt"), Err(NameError::Character('?')));
    assert_eq!(valid_name("tab\there"), Err(NameError::Character('\t')));
    assert_eq!(valid_name("trailing."), Err(NameError::Ending));
    assert_eq!(valid_name("trailing "), Err(NameError::Ending));
    assert_eq!(valid_name("."), Err(NameError::Reserved));
    assert_eq!(valid_name(".."), Err(NameError::Reserved));
    assert_eq!(valid_name("con"), Err(NameError::Reserved));
    assert_eq!(valid_name("LPT1.txt"), Err(NameError::Reserved));
    assert_eq!(valid_name("console.log"), Ok(()), "only the whole stem is reserved");
}

#[test]
fn a_free_name_counts_up_before_the_extension() {
    let root = scratch("free");
    assert_eq!(free_name(&root, "New file.txt"), "New file.txt");
    fs::write(root.join("New file.txt"), "").expect("a file");
    assert_eq!(free_name(&root, "New file.txt"), "New file 2.txt");
    fs::write(root.join("New file 2.txt"), "").expect("a file");
    assert_eq!(free_name(&root, "New file.txt"), "New file 3.txt");
    fs::create_dir(root.join("New folder")).expect("a folder");
    assert_eq!(free_name(&root, "New folder"), "New folder 2");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_copy_beside_its_original_is_named_as_a_copy() {
    let root = scratch("copy-name");
    assert_eq!(copy_name(&root, "notes.txt", "copy"), "notes (copy).txt");
    fs::write(root.join("notes (copy).txt"), "").expect("a file");
    assert_eq!(copy_name(&root, "notes.txt", "copy"), "notes (copy 2).txt");
    assert_eq!(
        copy_name(&root, ".gitignore", "copy"),
        ".gitignore (copy)",
        "a leading dot is not an extension"
    );
    assert_eq!(copy_name(&root, "saves", "copy"), "saves (copy)");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_copy_into_another_folder_keeps_the_name_and_the_source() {
    let root = scratch("copy");
    fs::create_dir(root.join("config")).expect("a folder");
    fs::write(root.join("a.toml"), "a = 1").expect("a file");
    let report = transfer(
        &job(&[root.join("a.toml")], &root.join("config"), Mode::Copy),
        Existing::Report,
        "copy",
        no_trash,
    );
    assert_eq!(report.written, [root.join("config").join("a.toml")]);
    assert!(report.conflicts.is_empty() && report.errors.is_empty());
    assert!(root.join("a.toml").is_file());
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_copy_into_its_own_folder_makes_a_copy_beside_it() {
    let root = scratch("copy-beside");
    fs::write(root.join("a.toml"), "a = 1").expect("a file");
    let report = transfer(&job(&[root.join("a.toml")], &root, Mode::Copy), Existing::Report, "copy", no_trash);
    assert_eq!(report.written, [root.join("a (copy).toml")]);
    assert_eq!(fs::read_to_string(root.join("a (copy).toml")).expect("the copy"), "a = 1");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_move_takes_a_folder_with_it_and_a_move_into_its_own_folder_does_nothing() {
    let root = scratch("move");
    fs::create_dir_all(root.join("kubejs").join("data")).expect("the folders");
    fs::write(root.join("kubejs").join("data").join("x.json"), "{}").expect("a file");
    fs::create_dir(root.join("backup")).expect("a folder");

    let still = transfer(&job(&[root.join("kubejs")], &root, Mode::Move), Existing::Report, "copy", no_trash);
    assert_eq!(still, Report::default());
    assert!(root.join("kubejs").is_dir());

    let report = transfer(
        &job(&[root.join("kubejs")], &root.join("backup"), Mode::Move),
        Existing::Report,
        "copy",
        no_trash,
    );
    assert_eq!(report.written, [root.join("backup").join("kubejs")]);
    assert!(!root.join("kubejs").exists());
    assert!(root.join("backup").join("kubejs").join("data").join("x.json").is_file());
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_folder_cannot_go_into_itself_or_a_folder_inside_it() {
    let root = scratch("itself");
    fs::create_dir_all(root.join("config").join("inner")).expect("the folders");
    for into in [root.join("config"), root.join("config").join("inner")] {
        for mode in [Mode::Copy, Mode::Move] {
            let report = transfer(&job(&[root.join("config")], &into, mode), Existing::Report, "copy", no_trash);
            assert_eq!(
                report.errors,
                [Failure {
                    name: "config".to_string(),
                    reason: Reason::IntoItself,
                }]
            );
        }
    }
    assert!(root.join("config").join("inner").is_dir(), "nothing moved");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_name_the_folder_holds_is_reported_and_left_alone() {
    let root = scratch("conflict");
    fs::create_dir(root.join("config")).expect("a folder");
    fs::write(root.join("a.toml"), "new").expect("a file");
    fs::write(root.join("b.toml"), "b").expect("a file");
    fs::write(root.join("config").join("a.toml"), "old").expect("a file");

    let sources = [root.join("a.toml"), root.join("b.toml")];
    let report = transfer(&job(&sources, &root.join("config"), Mode::Move), Existing::Report, "copy", no_trash);
    assert_eq!(report.conflicts, [root.join("a.toml")]);
    assert_eq!(
        report.written,
        [root.join("config").join("b.toml")],
        "the entry with a free name still moves"
    );
    assert_eq!(fs::read_to_string(root.join("config").join("a.toml")).expect("the old file"), "old");
    assert!(root.join("a.toml").is_file());
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn replacing_sends_the_old_entry_to_the_trash_first() {
    let root = scratch("replace");
    fs::create_dir(root.join("config")).expect("a folder");
    fs::write(root.join("a.toml"), "new").expect("a file");
    fs::write(root.join("config").join("a.toml"), "old").expect("a file");

    let trashed = RefCell::new(Vec::new());
    let report = transfer(
        &job(&[root.join("a.toml")], &root.join("config"), Mode::Move),
        Existing::Replace,
        "copy",
        trash_into(&trashed),
    );
    assert_eq!(report.written, [root.join("config").join("a.toml")]);
    assert_eq!(*trashed.borrow(), [root.join("config").join("a.toml")]);
    assert_eq!(fs::read_to_string(root.join("config").join("a.toml")).expect("the new file"), "new");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_trash_that_fails_leaves_both_entries_as_they_were() {
    let root = scratch("replace-fails");
    fs::create_dir(root.join("config")).expect("a folder");
    fs::write(root.join("a.toml"), "new").expect("a file");
    fs::write(root.join("config").join("a.toml"), "old").expect("a file");
    let report = transfer(
        &job(&[root.join("a.toml")], &root.join("config"), Mode::Move),
        Existing::Replace,
        "copy",
        no_trash,
    );
    assert_eq!(report.errors.len(), 1);
    assert_eq!(fs::read_to_string(root.join("config").join("a.toml")).expect("the old file"), "old");
    assert!(root.join("a.toml").is_file());
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn rename_keeps_the_entry_in_its_folder_and_refuses_a_taken_or_bad_name() {
    let root = scratch("rename");
    fs::write(root.join("a.toml"), "a").expect("a file");
    fs::write(root.join("b.toml"), "b").expect("a file");
    assert_eq!(rename(&root.join("a.toml"), "b.toml"), Err(MakeError::Taken));
    assert_eq!(rename(&root.join("a.toml"), "a:b"), Err(MakeError::Name(NameError::Character(':'))));
    assert_eq!(
        rename(&root.join("a.toml"), "a.toml"),
        Ok(root.join("a.toml")),
        "the same name changes nothing"
    );
    assert_eq!(rename(&root.join("a.toml"), "c.toml"), Ok(root.join("c.toml")));
    assert!(root.join("c.toml").is_file() && !root.join("a.toml").exists());
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn create_makes_an_empty_file_or_a_folder_under_the_next_free_name() {
    let root = scratch("create");
    assert_eq!(create(&root, "New folder", Kind::Folder), Ok(root.join("New folder")));
    assert_eq!(create(&root, "New folder", Kind::Folder), Ok(root.join("New folder 2")));
    assert!(root.join("New folder 2").is_dir());
    assert_eq!(create(&root, "New file.txt", Kind::File), Ok(root.join("New file.txt")));
    assert_eq!(fs::read(root.join("New file.txt")).expect("the file"), b"");
    let _ = fs::remove_dir_all(&root);
}
