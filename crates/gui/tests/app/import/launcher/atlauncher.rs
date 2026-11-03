use std::path::Path;

use super::*;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lodestone-atlauncher-{tag}-{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).expect("scratch folder created");
    dir
}

/// An instance folder whose `instance.json` names it, gives its Minecraft version, and gives its loader.
fn make_instance(root: &Path, folder: &str, body: &str) -> PathBuf {
    let dir = root.join(folder);
    std::fs::create_dir_all(dir.join("mods")).expect("instance made");
    std::fs::write(dir.join("instance.json"), body).expect("instance.json written");
    dir
}

#[test]
fn a_fabric_instance_reads_its_name_version_and_loader_from_instance_json() {
    let root = scratch("fabric");
    let dir = make_instance(
        &root,
        "Alpha",
        r#"{"id":"1.20.1","launcher":{"name":"Alpha Pack","loaderVersion":{"type":"Fabric","version":"0.15.11"}}}"#,
    );
    let meta = read(&dir).expect("the instance reads");
    assert_eq!(meta.name, "Alpha Pack");
    assert_eq!(meta.minecraft, "1.20.1");
    assert_eq!(meta.loader, Loader::Fabric);
    assert_eq!(meta.loader_version.as_deref(), Some("0.15.11"));
    assert_eq!(meta.game_dir, dir, "ATLauncher keeps the game files beside instance.json");
    assert_eq!(meta.skip, vec!["instance.json".to_string()]);
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn a_forge_instance_takes_the_minecraft_version_in_front_of_its_build() {
    let root = scratch("forge");
    let dir = make_instance(
        &root,
        "Forge",
        r#"{"id":"1.20.1","launcher":{"name":"Forge Pack","loaderVersion":{"type":"Forge","version":"47.2.0"}}}"#,
    );
    assert_eq!(read(&dir).expect("reads").loader_version.as_deref(), Some("1.20.1-47.2.0"));
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn a_loader_this_app_cannot_install_is_named_as_the_reason() {
    let root = scratch("legacy");
    let dir = make_instance(
        &root,
        "Legacy",
        r#"{"id":"1.7.10","launcher":{"name":"Legacy","loaderVersion":{"type":"LegacyFabric","version":"1"}}}"#,
    );
    assert_eq!(read(&dir).err(), Some(unsupported_loader("LegacyFabric")));
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn the_instances_folder_is_found_under_the_launcher_folder_or_as_the_folder_itself() {
    let root = scratch("in-folder");
    make_instance(&root.join("instances"), "Alpha", r#"{"id":"1.20.1","launcher":{"name":"Alpha"}}"#);
    assert_eq!(instances_in(&root), Some(root.join("instances")));
    let direct = scratch("direct");
    make_instance(&direct, "Beta", r#"{"id":"1.20.1","launcher":{"name":"Beta"}}"#);
    assert_eq!(instances_in(&direct), Some(direct.clone()));
    std::fs::remove_dir_all(&root).ok();
    std::fs::remove_dir_all(&direct).ok();
}

#[test]
fn a_listing_names_each_instance_and_reports_a_missing_version_on_its_row() {
    let root = scratch("list");
    make_instance(&root, "Alpha", r#"{"id":"1.20.1","launcher":{"name":"Alpha"}}"#);
    make_instance(&root, "Blank", r#"{"id":"","launcher":{"name":"Blank"}}"#);
    let listed = list(Launcher::AtLauncher, &root).expect("the folder lists");
    assert_eq!(listed.len(), 2);
    let blank = listed.iter().find(|row| row.name == "Blank").expect("the blank row is listed");
    assert_eq!(blank.summary, Err(no_version()));
    std::fs::remove_dir_all(&root).ok();
}
