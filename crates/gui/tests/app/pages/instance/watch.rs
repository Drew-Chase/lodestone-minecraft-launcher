//! Tests for `pages/instance/watch.rs`, kept out of the source file. They are compiled as the `tests` child
//! module of `watch`, so they can reach its private helpers.

use std::path::{Path, PathBuf};

use iced::futures::StreamExt;
use notify::event::CreateKind;

use super::*;

fn folder() -> PathBuf {
    PathBuf::from("/instances/aether")
}

#[test]
fn a_change_inside_a_save_names_the_save() {
    let folder = folder();
    let save = folder.join("saves/Hub");
    assert_eq!(classify(&folder, &save.join("level.dat")), Some(Change::World(save.clone())));
    assert_eq!(classify(&folder, &save.join("region/r.0.0.mca")), Some(Change::World(save.clone())));
    assert_eq!(classify(&folder, &save), Some(Change::World(save.clone())));
    assert_eq!(classify(&folder, &folder.join("saves")), Some(Change::Worlds));
}

#[test]
fn the_lock_a_running_game_holds_is_not_a_change_to_its_save() {
    let folder = folder();
    let save = folder.join("saves/Hub");
    assert_eq!(classify(&folder, &save.join("session.lock")), None);
    assert_eq!(classify(&folder, &save.join("level.dat")), Some(Change::World(save.clone())));
    assert_eq!(classify(&folder, &save.join("region/session.lock")), Some(Change::World(save)));
}

#[test]
fn a_change_to_the_server_list_names_the_list_itself() {
    let folder = folder();
    assert_eq!(classify(&folder, &folder.join("servers.dat")), Some(Change::Servers));
    assert_eq!(classify(&folder, &folder.join("servers.dat_old")), None);
    assert_eq!(classify(&folder, &folder.join("servers.dat/inner")), None);
}

#[test]
fn a_capture_change_names_the_png_file_directly_inside_the_folder() {
    let folder = folder();
    let shot = folder.join("screenshots/2026-10-30_09.00.00.png");
    assert_eq!(classify(&folder, &shot), Some(Change::Shot(shot.clone())));
    assert_eq!(classify(&folder, &folder.join("screenshots/notes.txt")), None);
    assert_eq!(classify(&folder, &folder.join("screenshots/deep/x.png")), None);
    assert_eq!(classify(&folder, &folder.join("screenshots")), Some(Change::Shots));
}

#[test]
fn everything_else_in_the_instance_is_ignored() {
    let folder = folder();
    assert_eq!(classify(&folder, &folder.join("logs/latest.log")), None);
    assert_eq!(classify(&folder, &folder.join("mods/notes.txt")), None);
    assert_eq!(classify(&folder, &folder.join("mods/deep/sodium.jar")), None);
    assert_eq!(classify(&folder, Path::new("/elsewhere/saves/Hub")), None);
}

#[test]
fn a_mod_jar_change_names_the_mods_folder_enabled_or_disabled() {
    let folder = folder();
    assert_eq!(classify(&folder, &folder.join("mods/sodium.jar")), Some(Change::Mods));
    assert_eq!(classify(&folder, &folder.join("mods/iris.jar.disabled")), Some(Change::Mods));
    assert_eq!(classify(&folder, &folder.join("mods/UPPER.JAR")), Some(Change::Mods));
    assert_eq!(classify(&folder, &folder.join("mods")), Some(Change::Mods));
}

#[test]
fn a_pack_change_names_a_pack_folder_or_an_entry_directly_inside_it() {
    let folder = folder();
    assert_eq!(classify(&folder, &folder.join("resourcepacks/Faithful.zip")), Some(Change::Packs));
    assert_eq!(classify(&folder, &folder.join("resourcepacks/Faithful")), Some(Change::Packs));
    assert_eq!(classify(&folder, &folder.join("shaderpacks/Complementary.zip")), Some(Change::Packs));
    assert_eq!(classify(&folder, &folder.join("shaderpacks")), Some(Change::Packs));
    assert_eq!(classify(&folder, &folder.join("resourcepacks")), Some(Change::Packs));
    assert_eq!(classify(&folder, &folder.join("resourcepacks/Faithful/pack.mcmeta")), None);
    assert_eq!(classify(&folder, &folder.join("shaderpacks/Complementary/shaders/x.glsl")), None);
}

#[test]
fn the_watched_folders_are_recognised_directly_inside_the_instance() {
    let folder = folder();
    assert!(is_folder(&folder, &folder.join("saves")));
    assert!(is_folder(&folder, &folder.join("screenshots")));
    assert!(is_folder(&folder, &folder.join("mods")));
    assert!(is_folder(&folder, &folder.join("resourcepacks")));
    assert!(is_folder(&folder, &folder.join("shaderpacks")));
    assert!(!is_folder(&folder, &folder.join("saves/Hub")));
    assert!(!is_folder(&folder, &folder.join("resourcepacks/Faithful.zip")));
    assert!(!is_folder(&folder, &folder.join("logs")));
}

#[test]
fn a_full_listing_covers_the_changes_inside_it() {
    let folder = folder();
    let shot = folder.join("screenshots/a.png");
    let changes = coalesce([
        Change::World(folder.join("saves/Hub")),
        Change::Worlds,
        Change::Shot(shot),
        Change::Shots,
        Change::World(folder.join("saves/Pixel")),
    ]);
    assert_eq!(changes, [Change::Worlds, Change::Shots]);
}

#[test]
fn separate_saves_and_captures_are_each_read_once() {
    let folder = folder();
    let save = folder.join("saves/Hub");
    let shot = folder.join("screenshots/a.png");
    let changes = coalesce([Change::World(save.clone()), Change::World(save.clone()), Change::Shot(shot.clone())]);
    assert_eq!(changes, [Change::World(save), Change::Shot(shot)]);
}

#[test]
fn a_mods_change_is_read_once_and_kept_beside_the_others() {
    let changes = coalesce([Change::Mods, Change::Mods, Change::Shots]);
    assert_eq!(changes, [Change::Shots, Change::Mods]);
}

#[test]
fn a_packs_change_is_read_once_and_kept_beside_the_others() {
    let changes = coalesce([Change::Packs, Change::Packs, Change::Mods]);
    assert_eq!(changes, [Change::Mods, Change::Packs]);
}

#[test]
fn writes_are_changes_and_reads_are_not() {
    assert!(relevant(&EventKind::Create(CreateKind::File)));
    assert!(relevant(&EventKind::Access(AccessKind::Close(AccessMode::Write))));
    assert!(!relevant(&EventKind::Access(AccessKind::Open(AccessMode::Read))));
    assert!(!relevant(&EventKind::Access(AccessKind::Close(AccessMode::Read))));
}

#[test]
fn a_resolved_path_is_written_back_under_the_folder_the_page_knows() {
    let known = Path::new("/links/aether");
    let resolved = Path::new("/real/aether");
    assert_eq!(
        rebase(known, resolved, &resolved.join("saves/Hub/level.dat")),
        known.join("saves/Hub/level.dat")
    );
    assert_eq!(
        rebase(known, known, &known.join("saves")),
        known.join("saves"),
        "an unresolved folder is left alone"
    );
}

/// A fresh instance folder under the system temp directory, with its `screenshots/` folder.
fn scratch(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("lodestone-watch-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(path.join("screenshots")).expect("scratch folder");
    std::fs::canonicalize(&path).expect("scratch folder resolves")
}

/// Watches `root`, writes `file` once the watch has registered its folders, and reports whether a batch carrying `change`
/// reaches the page after the folder goes quiet. The folder `file` is in must already exist.
fn reports_written(root: &Path, file: PathBuf, change: Change) -> bool {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("a runtime starts");

    runtime.block_on(async {
        let (mut output, mut messages) = iced::futures::channel::mpsc::channel::<Message>(16);
        let watched = root.to_path_buf();
        let watch = tokio::spawn(async move { run(&watched, &mut output).await });
        // The watch registers its folders as it starts, so the file is written once it has.
        tokio::time::sleep(Duration::from_millis(300)).await;
        std::fs::write(&file, b"written").expect("the file writes");

        let found = tokio::time::timeout(Duration::from_secs(5), async {
            while let Some(message) = messages.next().await {
                if let Message::FilesChanged { changes, .. } = message
                    && changes.contains(&change)
                {
                    return true;
                }
            }
            false
        })
        .await;
        watch.abort();
        matches!(found, Ok(true))
    })
}

/// A capture written into the watched folder reaches the page as a change, after the folder goes quiet.
#[test]
fn a_capture_written_to_the_folder_is_reported_as_a_change() {
    let root = scratch("capture");
    let shot = root.join("screenshots/2026-10-30_09.00.00.png");
    let reported = reports_written(&root, shot.clone(), Change::Shot(shot));
    assert!(reported, "the new capture is reported");
    std::fs::remove_dir_all(&root).ok();
}

/// A mod jar written into the watched `mods/` folder reaches the page as a change, after the folder goes quiet.
#[test]
fn a_mod_jar_written_to_the_folder_is_reported_as_a_change() {
    let root = scratch("mods");
    std::fs::create_dir_all(root.join("mods")).expect("the mods folder is made");
    let jar = root.join("mods/sodium.jar");
    let reported = reports_written(&root, jar, Change::Mods);
    assert!(reported, "the new jar is reported");
    std::fs::remove_dir_all(&root).ok();
}

/// A resource pack written into the watched `resourcepacks/` folder reaches the page as a change, so the Content tab's
/// list follows the folder as it follows the mods.
#[test]
fn a_resource_pack_written_to_the_folder_is_reported_as_a_change() {
    let root = scratch("resource-packs");
    std::fs::create_dir_all(root.join("resourcepacks")).expect("the packs folder is made");
    let pack = root.join("resourcepacks/Faithful.zip");
    let reported = reports_written(&root, pack, Change::Packs);
    assert!(reported, "the new resource pack is reported");
    std::fs::remove_dir_all(&root).ok();
}
