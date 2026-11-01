use super::*;
use crate::support::{put, scratch};

fn children(folder: &str) -> Vec<String> {
    match folder {
        "mods" => vec!["mods/a.jar".to_string(), "mods/b.jar".to_string()],
        "config" => vec!["config/a.toml".to_string()],
        _ => Vec::new(),
    }
}

#[test]
fn a_new_export_leaves_out_the_default_top_level_folders() {
    let selection = Selection::new_export();
    assert!(!selection.is_included("logs"));
    assert!(!selection.is_included("saves"));
    assert!(selection.is_included("mods"));
    assert!(selection.is_included("config/options.toml"));
}

#[test]
fn a_path_under_an_excluded_folder_is_excluded() {
    let selection = Selection::from_excluded(["mods".to_string()]);
    assert!(!selection.is_included("mods/sodium.jar"));
    assert_eq!(selection.state("mods"), CheckState::Unchecked);
}

#[test]
fn an_excluded_child_makes_its_folder_partial() {
    let selection = Selection::from_excluded(["config/sodium.json".to_string()]);
    assert_eq!(selection.state("config"), CheckState::Partial);
    assert_eq!(selection.state("config/sodium.json"), CheckState::Unchecked);
    assert_eq!(selection.state("mods"), CheckState::Checked);
}

#[test]
fn including_a_file_under_an_excluded_folder_turns_its_other_children_off() {
    let mut selection = Selection::from_excluded(["mods".to_string()]);
    selection.set("mods/b.jar", true, children);
    assert!(selection.is_included("mods/b.jar"));
    assert!(!selection.is_included("mods/a.jar"));
    assert_eq!(selection.state("mods"), CheckState::Partial);
}

#[test]
fn including_a_folder_clears_what_was_turned_off_inside_it() {
    let mut selection = Selection::from_excluded(["config/a.toml".to_string()]);
    selection.set("config", true, children);
    assert_eq!(selection.state("config"), CheckState::Checked);
}

#[test]
fn excluding_a_folder_drops_the_exclusions_inside_it() {
    let mut selection = Selection::from_excluded(["config/a.toml".to_string()]);
    selection.set("config", false, children);
    assert_eq!(selection.state("config"), CheckState::Unchecked);
    selection.set("config", true, children);
    assert!(selection.is_included("config/a.toml"));
}

#[test]
fn expand_lists_the_taken_files_sorted_and_skips_leftovers_and_excluded_folders() {
    let root = scratch("plan-expand");
    put(&root, "mods/b.jar", b"b");
    put(&root, "mods/a.jar", b"aa");
    put(&root, "config/options.toml", b"ccc");
    put(&root, "logs/latest.log", b"skipped");
    put(&root, "saves/world/level.dat", b"skipped");
    put(&root, ".installing", b"");
    let selection = Selection::new_export();
    let planned = expand(&root, &selection).expect("lists");
    let paths: Vec<&str> = planned.iter().map(|file| file.path.as_str()).collect();
    assert_eq!(paths, ["config/options.toml", "mods/a.jar", "mods/b.jar"]);
    assert_eq!(planned[1].size, 2);
}

#[cfg(unix)]
#[test]
fn expand_skips_symbolic_links() {
    let root = scratch("plan-links");
    put(&root, "config/options.toml", b"x");
    std::os::unix::fs::symlink(root.join("config/options.toml"), root.join("config/link.toml")).expect("links");
    let planned = expand(&root, &Selection::new_export()).expect("lists");
    let paths: Vec<&str> = planned.iter().map(|file| file.path.as_str()).collect();
    assert_eq!(paths, ["config/options.toml"]);
}
