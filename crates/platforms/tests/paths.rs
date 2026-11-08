use std::path::Path;

use super::*;

#[test]
fn plain_relative_paths_are_kept() {
    assert!(is_contained("mods/sodium.jar"));
    assert!(is_contained("config/a b/c.toml"));
    assert!(is_contained("options.txt"));
    assert_eq!(contained(Path::new("/game"), "mods/a.jar"), Some(Path::new("/game").join("mods/a.jar")));
}

#[test]
fn paths_that_leave_the_folder_are_refused() {
    for path in [
        "",
        "../x",
        "mods/../../x",
        "mods/..",
        "./mods/a.jar",
        "/etc/passwd",
        "//server/share/x",
        "C:\\Windows\\x",
        "C:/Windows/x",
        "mods\\..\\x",
        "mods//a.jar",
        "mods/",
        "file.txt:stream",
        "a\0b",
    ] {
        assert!(!is_contained(path), "{path:?} should be refused");
        assert_eq!(contained(Path::new("/game"), path), None);
    }
}
