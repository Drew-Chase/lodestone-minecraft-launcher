use std::path::Path;

use super::*;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lodestone-prism-{tag}-{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).expect("scratch folder created");
    dir
}

fn put(root: &Path, relative: &str, body: &str) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().expect("a file has a parent")).expect("folders made");
    std::fs::write(path, body).expect("file written");
}

/// An instance folder with the given components, named `name`, and a game folder called `game`.
fn make_instance(root: &Path, folder: &str, components: &str, game: &str) -> PathBuf {
    let dir = root.join(folder);
    put(&dir, "mmc-pack.json", &format!(r#"{{"formatVersion":1,"components":[{components}]}}"#));
    put(&dir, "instance.cfg", &format!("name={folder}\niconKey=default\n"));
    std::fs::create_dir_all(dir.join(game)).expect("game folder made");
    dir
}

#[test]
fn a_fabric_instance_reads_its_version_and_its_minecraft_folder() {
    let root = scratch("fabric");
    let dir = make_instance(
        &root,
        "Fabric Pack",
        r#"{"uid":"net.minecraft","version":"1.21.1"},{"uid":"net.fabric.fabric-loader","version":"0.16.10"}"#,
        "minecraft",
    );
    let meta = read(&dir).expect("the instance reads");
    assert_eq!(meta.name, "Fabric Pack");
    assert_eq!(meta.minecraft, "1.21.1");
    assert_eq!(meta.loader, Loader::Fabric);
    assert_eq!(meta.loader_version.as_deref(), Some("0.16.10"));
    assert_eq!(meta.game_dir, dir.join("minecraft"));
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn a_forge_instance_takes_the_minecraft_version_in_front_of_its_build() {
    let root = scratch("forge");
    let dir = make_instance(
        &root,
        "Old Forge",
        r#"{"uid":"net.minecraft","version":"1.20.1"},{"uid":"net.minecraftforge","version":"47.2.0"}"#,
        ".minecraft",
    );
    let meta = read(&dir).expect("the instance reads");
    assert_eq!(meta.loader, Loader::Forge);
    assert_eq!(meta.loader_version.as_deref(), Some("1.20.1-47.2.0"));
    assert_eq!(
        meta.game_dir,
        dir.join(".minecraft"),
        "an older instance keeps its game folder under .minecraft"
    );
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn a_liteloader_instance_is_refused_with_its_loader_named() {
    let root = scratch("liteloader");
    let dir = make_instance(
        &root,
        "Lite",
        r#"{"uid":"net.minecraft","version":"1.12.2"},{"uid":"com.mumfrey.liteloader","version":"1.12"}"#,
        "minecraft",
    );
    assert_eq!(read(&dir).err(), Some(unsupported_loader("LiteLoader")));
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn an_instance_without_a_minecraft_version_or_a_game_folder_is_refused() {
    let root = scratch("refused");
    let without_version = make_instance(
        &root,
        "No version",
        r#"{"uid":"net.fabric.fabric-loader","version":"0.16.10"}"#,
        "minecraft",
    );
    assert_eq!(read(&without_version).err(), Some(no_version()));
    let no_game = make_instance(&root, "No game", r#"{"uid":"net.minecraft","version":"1.21.1"}"#, "elsewhere");
    std::fs::remove_dir_all(no_game.join("elsewhere")).ok();
    assert_eq!(read(&no_game).err(), Some(no_game_folder()));
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn a_listing_shows_the_instances_that_hold_a_pack_file_and_names_each_by_its_cfg() {
    let root = scratch("list");
    make_instance(&root, "Alpha", r#"{"uid":"net.minecraft","version":"1.21.1"}"#, "minecraft");
    std::fs::create_dir_all(root.join("Not an instance")).expect("folder made");
    let listed = list(Launcher::Prism, &root).expect("the folder lists");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].name, "Alpha");
    assert_eq!(listed[0].summary, Ok("1.21.1 · VANILLA".to_string()));
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn the_instances_folder_is_found_under_the_launcher_folder() {
    let root = scratch("instances-in");
    make_instance(
        &root.join("instances"),
        "Alpha",
        r#"{"uid":"net.minecraft","version":"1.21.1"}"#,
        "minecraft",
    );
    assert_eq!(instances_in(&root), Some(root.join("instances")));
    assert_eq!(instances_in(&root.join("nothing-here")), None);
    std::fs::remove_dir_all(&root).ok();
}
