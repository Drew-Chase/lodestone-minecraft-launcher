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
    assert_eq!(classify(&folder, &folder.join("mods/sodium.jar")), None);
    assert_eq!(classify(&folder, Path::new("/elsewhere/saves/Hub")), None);
}

#[test]
fn the_two_watched_folders_are_recognised_directly_inside_the_instance() {
    let folder = folder();
    assert!(is_folder(&folder, &folder.join("saves")));
    assert!(is_folder(&folder, &folder.join("screenshots")));
    assert!(!is_folder(&folder, &folder.join("saves/Hub")));
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

/// A capture written into the watched folder reaches the page as a change, after the folder goes quiet.
#[test]
fn a_capture_written_to_the_folder_is_reported_as_a_change() {
    let root = scratch("capture");
    let shot = root.join("screenshots/2026-10-30_09.00.00.png");
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("a runtime starts");

    let reported = runtime.block_on(async {
        let (mut output, mut messages) = iced::futures::channel::mpsc::channel::<Message>(16);
        let watched = root.clone();
        let watch = tokio::spawn(async move { run(&watched, &mut output).await });
        // The watch registers its folders as it starts, so the capture is written once it has.
        tokio::time::sleep(Duration::from_millis(300)).await;
        std::fs::write(&shot, b"capture").expect("the capture writes");

        let found = tokio::time::timeout(Duration::from_secs(5), async {
            while let Some(message) = messages.next().await {
                if let Message::FilesChanged { changes, .. } = message
                    && changes.contains(&Change::Shot(shot.clone()))
                {
                    return true;
                }
            }
            false
        })
        .await;
        watch.abort();
        found
    });

    assert!(matches!(reported, Ok(true)), "the new capture is reported: {reported:?}");
    std::fs::remove_dir_all(&root).ok();
}
