use super::*;

/// A scratch folder for one test, emptied first.
fn scratch(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("lodestone-shared-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("scratch folder");
    path
}

#[test]
fn every_kind_of_content_has_its_own_shared_folder() {
    let kinds = [
        ContentType::Modpack,
        ContentType::ResourcePack,
        ContentType::Shader,
        ContentType::DataPack,
        ContentType::World,
    ];
    let names: std::collections::HashSet<&str> = kinds.into_iter().map(folder_name).collect();
    assert_eq!(names.len(), kinds.len());
    assert_eq!(folder_name(ContentType::ResourcePack), "resourcepacks");
}

#[test]
fn a_shared_folder_sits_under_the_launcher_root() {
    let paths = LauncherPaths::new("/data/lodestone");
    assert_eq!(folder(&paths, ContentType::Shader), PathBuf::from("/data/lodestone/shared/shaderpacks"));
}

#[test]
fn download_folders_keep_only_file_name_characters() {
    assert_eq!(download_folder("modrinth", "AANobbMI", "abc-123"), "modrinth-AANobbMI-abc-123");
    assert_eq!(download_folder("curseforge", "a/b c", "1.0"), "curseforge-a_b_c-1_0");
}

#[test]
fn loose_files_and_the_files_of_each_download_folder_are_the_entries_sorted_by_name() {
    let root = scratch("entries");
    std::fs::write(root.join("Loose.zip"), b"x").unwrap();
    std::fs::create_dir_all(root.join("modrinth-p1-v1")).unwrap();
    std::fs::write(root.join("modrinth-p1-v1").join("Alpha.zip"), b"x").unwrap();
    let listed = entries(&root);
    let names: Vec<&str> = listed.iter().map(|entry| entry.name.as_str()).collect();
    assert_eq!(names, ["Alpha.zip", "Loose.zip"]);
    assert_eq!(listed[0].download.as_deref(), Some("modrinth-p1-v1"));
    assert_eq!(listed[1].download, None);
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn a_folder_that_is_not_there_lists_nothing() {
    assert!(entries(&scratch("missing").join("absent")).is_empty());
}

#[test]
fn copying_an_entry_keeps_its_name_and_never_overwrites() {
    let root = scratch("copy");
    let from = root.join("source.zip");
    std::fs::write(&from, b"new").unwrap();
    let into = root.join("resourcepacks");
    std::fs::create_dir_all(&into).unwrap();
    std::fs::write(into.join("source.zip"), b"old").unwrap();
    assert_eq!(copy_into(&from, &into).unwrap(), "source.zip");
    assert_eq!(std::fs::read(into.join("source.zip")).unwrap(), b"old", "a pack already there is kept");
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn a_folder_is_copied_with_everything_in_it() {
    let root = scratch("folder");
    let from = root.join("Pack");
    std::fs::create_dir_all(from.join("assets")).unwrap();
    std::fs::write(from.join("pack.mcmeta"), b"{}").unwrap();
    std::fs::write(from.join("assets").join("icon.png"), b"png").unwrap();
    let into = root.join("shaderpacks");
    assert_eq!(copy_into(&from, &into).unwrap(), "Pack");
    assert_eq!(std::fs::read(into.join("Pack").join("assets").join("icon.png")).unwrap(), b"png");
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn deleting_a_file_from_a_download_takes_its_download_folder() {
    let entry = SharedEntry {
        name: "Chat.zip".to_string(),
        path: PathBuf::from("/shared/resourcepacks/modrinth-chat-1/Chat.zip"),
        is_dir: false,
        download: Some("modrinth-chat-1".to_string()),
    };
    assert_eq!(delete_target(&entry), PathBuf::from("/shared/resourcepacks/modrinth-chat-1"));
}

#[test]
fn deleting_a_loose_file_takes_only_that_file() {
    let entry = SharedEntry {
        name: "Loose.zip".to_string(),
        path: PathBuf::from("/shared/resourcepacks/Loose.zip"),
        is_dir: false,
        download: None,
    };
    assert_eq!(delete_target(&entry), PathBuf::from("/shared/resourcepacks/Loose.zip"));
}
