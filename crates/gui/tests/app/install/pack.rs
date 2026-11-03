use lodestone_platforms::Modpack;

use super::*;

fn pack(name: &str, loader: Option<&'static str>) -> Modpack {
    Modpack {
        name: name.to_string(),
        version: None,
        minecraft: "1.20.1".to_string(),
        loader,
        loader_version: None,
        files: Vec::new(),
        overrides: None,
        overrides_skip: Vec::new(),
        archives: Vec::new(),
    }
}

fn paths() -> LauncherPaths {
    let root = std::env::temp_dir().join(format!("lodestone-pack-request-{}", std::process::id()));
    LauncherPaths::new(&root)
}

#[tokio::test]
async fn a_pack_read_from_a_file_records_no_page() {
    let install = request(&paths(), &[], pack("Better MC", Some("fabric")), None, None, None)
        .await
        .expect("a pack with no files needs no network");
    assert_eq!(install.name, "Better MC");
    assert_eq!(install.loader, Loader::Fabric);
    assert_eq!(install.source_url, None);
    assert_eq!(install.source_platform, None);
    assert_eq!(install.source_project_id, None);
    assert!(matches!(install.icon, IconSource::Composed { .. }));
}

#[tokio::test]
async fn a_pack_taken_from_a_page_records_the_page_and_its_project() {
    let page = Page {
        source: Source::CurseForge,
        project_id: "123".to_string(),
        url: Some("https://www.curseforge.com/minecraft/modpacks/better-mc".to_string()),
        icon_url: None,
    };
    let install = request(&paths(), &[], pack("Better MC", None), None, Some(Source::CurseForge), Some(page))
        .await
        .expect("a pack with no files needs no network");
    assert_eq!(install.source_platform.as_deref(), Some("curseforge"));
    assert_eq!(install.source_project_id.as_deref(), Some("123"));
    assert_eq!(
        install.source_url.as_deref(),
        Some("https://www.curseforge.com/minecraft/modpacks/better-mc")
    );
    assert_eq!(install.loader, Loader::Vanilla);
}

#[tokio::test]
async fn the_folder_avoids_the_names_already_taken() {
    let base = naming::clean_directory_name("Better MC");
    let install = request(&paths(), std::slice::from_ref(&base), pack("Better MC", None), None, None, None)
        .await
        .expect("a pack with no files needs no network");
    assert_ne!(install.directory_name, base);
}
