use super::*;

/// A world's zip in the download folder `download` of the shared worlds folder.
fn shared_zip(name: &str, download: &str) -> SharedEntry {
    SharedEntry {
        name: name.to_string(),
        path: PathBuf::from(format!("/shared/worlds/{download}/{name}")),
        is_dir: false,
        download: Some(download.to_string()),
    }
}

#[test]
fn a_shared_world_is_marked_when_a_save_of_its_import_name_is_there() {
    let worlds = vec![
        shared_zip("Aether Hub.zip", "curseforge-1-2"),
        shared_zip("Pixel Town.zip", "curseforge-3-4"),
    ];
    let saves = vec!["Aether Hub".to_string()];
    let marked: Vec<bool> = mark_present(worlds, &saves).into_iter().map(|(_, present)| present).collect();
    assert_eq!(marked, [true, false]);
}

#[test]
fn only_zip_archives_are_worlds() {
    assert!(is_world_archive(&shared_zip("Aether Hub.zip", "curseforge-1-2")));
    assert!(is_world_archive(&shared_zip("ARCHIVE.ZIP", "curseforge-1-2")));
    assert!(!is_world_archive(&shared_zip("notes.txt", "curseforge-1-2")));
    let folder = SharedEntry {
        name: "Aether Hub.zip".to_string(),
        path: PathBuf::from("/shared/worlds/Aether Hub.zip"),
        is_dir: true,
        download: None,
    };
    assert!(!is_world_archive(&folder));
}
