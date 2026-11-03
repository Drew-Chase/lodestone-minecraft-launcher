use std::path::Path;

use super::*;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lodestone-curseforge-{tag}-{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).expect("scratch folder created");
    dir
}

/// An instance folder under `Instances` with a `minecraftinstance.json` holding `body`, and a `mods` folder.
fn make_instance(root: &Path, folder: &str, body: &str) -> PathBuf {
    let dir = root.join("Instances").join(folder);
    std::fs::create_dir_all(dir.join("mods")).expect("instance made");
    std::fs::write(dir.join(METADATA), body).expect("metadata written");
    dir
}

#[test]
fn a_neoforge_instance_reads_its_version_from_its_base_loader() {
    let root = scratch("neoforge");
    let dir = make_instance(
        &root,
        "Essentials",
        r#"{"name":"Essentials Mods","gameVersion":"1.21.1","isVanilla":false,"baseModLoader":{"name":"neoforge-21.1.251","forgeVersion":"21.1.251","minecraftVersion":"1.21.1"}}"#,
    );
    let meta = read(&dir).expect("the instance reads");
    assert_eq!(meta.name, "Essentials Mods");
    assert_eq!(meta.minecraft, "1.21.1");
    assert_eq!(meta.loader, Loader::NeoForge);
    assert_eq!(meta.loader_version.as_deref(), Some("21.1.251"));
    assert_eq!(meta.game_dir, dir, "the CurseForge app keeps the game files at the folder root");
    assert!(meta.skip.iter().any(|name| name == "minecraftinstance.json"));
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn a_forge_instance_takes_the_minecraft_version_in_front_of_its_build() {
    let root = scratch("forge");
    let dir = make_instance(
        &root,
        "Forge",
        r#"{"name":"Forge Pack","gameVersion":"1.20.1","baseModLoader":{"name":"forge-47.2.0","forgeVersion":"47.2.0"}}"#,
    );
    let meta = read(&dir).expect("reads");
    assert_eq!(meta.loader, Loader::Forge);
    assert_eq!(meta.loader_version.as_deref(), Some("1.20.1-47.2.0"));
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn a_vanilla_instance_has_no_loader_and_a_missing_version_is_refused() {
    let root = scratch("vanilla");
    let vanilla = make_instance(&root, "Plain", r#"{"name":"Plain","gameVersion":"1.21.1","isVanilla":true}"#);
    let meta = read(&vanilla).expect("reads");
    assert_eq!(meta.loader, Loader::Vanilla);
    assert_eq!(meta.loader_version, None);
    let blank = make_instance(&root, "Blank", r#"{"name":"Blank"}"#);
    assert_eq!(read(&blank).err(), Some(no_version()));
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn the_instances_folder_is_found_and_its_creator_files_are_left_out_of_the_copy() {
    let root = scratch("in-folder");
    make_instance(
        &root,
        "Alpha",
        r#"{"name":"Alpha","gameVersion":"1.21.1","baseModLoader":{"name":"neoforge-21.1.251"}}"#,
    );
    assert_eq!(instances_in(&root), Some(root.join("Instances")));
    let listed = list(Launcher::CurseForge, &root.join("Instances")).expect("the folder lists");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].summary, Ok("1.21.1 · NEOFORGE 21.1.251".to_string()));
    std::fs::remove_dir_all(&root).ok();
}
