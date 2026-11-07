use super::*;
use std::path::PathBuf;

/// A scratch folder for one test, emptied first.
fn scratch(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("lodestone-datapacks-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("scratch folder");
    path
}

#[test]
fn the_dialog_lists_the_world_datapacks_and_marks_the_shared_ones_the_world_has() {
    let root = scratch("read");
    let world = root.join("saves").join("One");
    std::fs::create_dir_all(world.join("datapacks")).unwrap();
    std::fs::write(world.join("datapacks").join("Mine.zip"), b"x").unwrap();
    let shared = root.join("shared").join("datapacks");
    std::fs::create_dir_all(shared.join("modrinth-p1-v1")).unwrap();
    std::fs::write(shared.join("modrinth-p1-v1").join("Mine.zip"), b"x").unwrap();
    std::fs::write(shared.join("Other.zip"), b"x").unwrap();

    let dialog = build_dialog(&shared, "a".to_string(), world, "One".to_string());
    let installed: Vec<&str> = dialog.installed.iter().map(|pack| pack.name.as_str()).collect();
    assert_eq!(installed, ["Mine"]);
    let shared_state: Vec<(&str, bool)> = dialog.shared.iter().map(|(entry, present)| (entry.name.as_str(), *present)).collect();
    assert_eq!(shared_state, [("Mine.zip", true), ("Other.zip", false)]);
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn a_world_with_no_datapacks_and_no_shared_ones_lists_none() {
    let root = scratch("empty");
    let dialog = build_dialog(&root.join("shared"), "a".to_string(), root.join("saves").join("Bare"), "Bare".to_string());
    assert!(dialog.installed.is_empty());
    assert!(dialog.shared.is_empty());
    std::fs::remove_dir_all(&root).ok();
}
