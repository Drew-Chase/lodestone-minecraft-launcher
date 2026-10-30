use std::time::Instant;

use super::download::Part;
use super::*;

/// A fresh folder for one test, named after the test so parallel runs do not share it.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lodestone-install-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the scratch folder creates");
    dir
}

#[test]
fn every_stage_has_a_label() {
    for stage in [Stage::Resolve, Stage::Download, Stage::Loader, Stage::Finish] {
        assert!(!stage.label().is_empty(), "{stage:?} needs a label");
    }
}

#[test]
fn the_download_parts_add_up_to_one_bar() {
    let mut totals = Totals::default();
    let libraries = totals.apply(PartReport {
        part: Part::Libraries,
        done: 5,
        total: 10,
    });
    assert_eq!(libraries, (5, 10));
    let (done, total) = totals.apply(PartReport {
        part: Part::Assets,
        done: 10,
        total: 30,
    });
    assert_eq!((done, total), (15, 40), "the parts count their files together");
    assert_eq!(fraction(done, total), Some(15.0 / 40.0));
}

#[test]
fn fractions_are_unknown_without_a_total() {
    assert_eq!(fraction(0, 0), None);
    assert_eq!(fraction(5, 10), Some(0.5));
    assert_eq!(fraction(12, 10), Some(1.0), "an over-count never passes 100%");
}

#[test]
fn ids_have_the_uuid_shape() {
    let id = new_id();
    let lengths: Vec<usize> = id.split('-').map(str::len).collect();
    assert_eq!(lengths, [8, 4, 4, 4, 12]);
    assert!(id.chars().all(|c| c == '-' || c.is_ascii_hexdigit()), "{id} is hexadecimal");
    assert_ne!(new_id(), id);
}

#[test]
fn a_folder_is_created_once_and_never_reused() {
    let root = scratch("folder-create");
    let path = root.join("Dragon Survival v2");
    let mut folder = Folder::create(&path).expect("a fresh folder creates");
    assert!(path.is_dir());
    assert!(Folder::create(&path).is_err(), "an existing folder is never taken over");
    folder.keep();
    drop(folder);
    assert!(path.is_dir(), "a kept folder survives the drop");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_dropped_folder_is_removed() {
    let root = scratch("folder-drop");
    let path = root.join("Partial");
    let folder = Folder::create(&path).expect("the folder creates");
    std::fs::write(path.join("half.jar"), b"partial").expect("the partial file writes");
    drop(folder);

    // The removal runs on a helper thread a couple of seconds later, once a stopped
    // installer has had time to exit.
    let deadline = Instant::now() + Duration::from_secs(10);
    while path.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(100));
    }
    assert!(!path.exists(), "the partial instance is removed");
    let _ = std::fs::remove_dir_all(&root);
}
