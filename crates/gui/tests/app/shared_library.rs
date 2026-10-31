use super::*;

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
