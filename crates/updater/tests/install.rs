use super::*;
use std::fs;
use std::path::PathBuf;

/// A fresh folder under the system temp directory, named for the test.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lodestone-updater-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("creating the scratch folder");
    dir
}

#[test]
fn a_launcher_beside_an_uninstaller_came_from_the_installer() {
    let dir = scratch("installed");
    fs::write(dir.join("uninstall.exe"), b"").expect("writing the uninstaller");
    assert!(installed_with_installer(&dir.join("lodestone_launcher.exe")));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_portable_copy_has_no_uninstaller() {
    let dir = scratch("portable");
    assert!(!installed_with_installer(&dir.join("lodestone_launcher.exe")));
    let _ = fs::remove_dir_all(&dir);
}
