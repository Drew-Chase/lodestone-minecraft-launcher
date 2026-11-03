use std::path::Path;

use super::*;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lodestone-official-{tag}-{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).expect("scratch folder created");
    dir
}

/// A `.minecraft` folder whose profiles file holds `profiles`.
fn dot_minecraft(root: &Path, profiles: &str) -> PathBuf {
    let dir = root.join(".minecraft");
    std::fs::create_dir_all(dir.join("mods")).expect("game folder made");
    std::fs::write(
        dir.join("launcher_profiles.json"),
        format!(r#"{{"profiles":{{{profiles}}},"version":3}}"#),
    )
    .expect("profiles written");
    dir
}

#[test]
fn a_fabric_profile_reads_its_loader_and_version_from_its_version_id() {
    let root = scratch("fabric");
    let dir = dot_minecraft(
        &root,
        r#""p1":{"name":"Fabric Pack","type":"custom","lastVersionId":"fabric-loader-0.16.10-1.20.1"}"#,
    );
    let meta = read(&dir, "p1").expect("the profile reads");
    assert_eq!(meta.name, "Fabric Pack");
    assert_eq!(meta.minecraft, "1.20.1");
    assert_eq!(meta.loader, Loader::Fabric);
    assert_eq!(meta.loader_version.as_deref(), Some("0.16.10"));
    assert_eq!(meta.game_dir, dir, "a profile without a gameDir uses the .minecraft folder");
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn a_forge_profile_and_a_neoforge_profile_name_their_minecraft_versions() {
    let root = scratch("forge");
    let dir = dot_minecraft(
        &root,
        r#""f":{"name":"Forge","type":"custom","lastVersionId":"1.20.1-forge-47.2.0"},"n":{"name":"Neo","type":"custom","lastVersionId":"neoforge-21.1.251"}"#,
    );
    let forge = read(&dir, "f").expect("the forge profile reads");
    assert_eq!(forge.loader, Loader::Forge);
    assert_eq!(forge.minecraft, "1.20.1");
    assert_eq!(forge.loader_version.as_deref(), Some("1.20.1-47.2.0"));
    let neo = read(&dir, "n").expect("the neoforge profile reads");
    assert_eq!(neo.loader, Loader::NeoForge);
    assert_eq!(neo.minecraft, "1.21.1", "a NeoForge build of 21.1 runs on Minecraft 1.21.1");
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn the_built_in_profiles_are_not_listed_and_each_custom_profile_is_named_by_its_source() {
    let root = scratch("list");
    let dir = dot_minecraft(
        &root,
        r#""latest-release":{"name":"","type":"latest-release","lastVersionId":"1.21.1"},"mine":{"name":"My Pack","type":"custom","lastVersionId":"1.20.1"}"#,
    );
    let listed = list(Launcher::Minecraft, &dir).expect("the folder lists");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].name, "My Pack");
    assert_eq!(listed[0].source, format!("{}#mine", dir.to_string_lossy()));
    assert_eq!(listed[0].summary, Ok("1.20.1 · VANILLA".to_string()));
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn a_profile_with_its_own_game_folder_copies_from_that_folder() {
    let root = scratch("game-dir");
    let game = root.join("GameFolder");
    std::fs::create_dir_all(&game).expect("game folder made");
    let dir = dot_minecraft(
        &root,
        &format!(
            r#""p":{{"name":"Own","type":"custom","lastVersionId":"1.20.1","gameDir":"{}"}}"#,
            game.to_string_lossy().replace('\\', "\\\\")
        ),
    );
    assert_eq!(read(&dir, "p").expect("reads").game_dir, game);
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn an_unrecognised_version_id_is_refused_with_the_reason() {
    let root = scratch("unknown");
    let dir = dot_minecraft(&root, r#""p":{"name":"Odd","type":"custom","lastVersionId":"beta-thing-1"}"#);
    assert_eq!(read(&dir, "p").err(), Some(unrecognised()));
    std::fs::remove_dir_all(&root).ok();
}
