use std::sync::atomic::AtomicBool;
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

/// A keep flag that nothing has set.
fn unkept() -> Arc<AtomicBool> {
    Arc::new(AtomicBool::new(false))
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
    let libraries = totals.apply(PartReport::counted(Part::Libraries, 5, 10));
    assert_eq!((libraries.done, libraries.total), (5, 10));
    let sum = totals.apply(PartReport::counted(Part::Assets, 10, 30));
    assert_eq!((sum.done, sum.total), (15, 40), "the parts count their files together");
    assert_eq!(fraction(sum.done, sum.total), Some(15.0 / 40.0));
}

#[test]
fn the_download_parts_add_up_their_bytes_and_rates() {
    let mut totals = Totals::default();
    totals.apply(PartReport {
        part: Part::Libraries,
        done: 1,
        total: 2,
        transfer: Transfer {
            done: 400,
            total: 1_000,
            per_second: 250,
        },
    });
    let sum = totals.apply(PartReport {
        part: Part::Assets,
        done: 3,
        total: 4,
        transfer: Transfer {
            done: 100,
            total: 600,
            per_second: 50,
        },
    });
    assert_eq!(
        sum.transfer,
        Transfer {
            done: 500,
            total: 1_600,
            per_second: 300,
        }
    );
}

#[test]
fn a_part_reports_its_latest_count_not_a_running_total() {
    let mut totals = Totals::default();
    totals.apply(PartReport::counted(Part::Java, 2, 4));
    let sum = totals.apply(PartReport::counted(Part::Java, 3, 4));
    assert_eq!((sum.done, sum.total), (3, 4));
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
fn a_folder_is_opened_for_its_install_and_refused_when_it_is_not_its_own() {
    let root = scratch("folder-open");
    let path = root.join("Dragon Survival v2");
    let mut folder = Folder::open(&path, unkept()).expect("a fresh folder opens");
    assert!(path.join(INSTALLING).is_file(), "the install marks its folder");
    folder.keep();
    drop(folder);
    assert!(path.is_dir(), "a kept folder survives the drop");

    let stranger = root.join("Someone else");
    std::fs::create_dir_all(&stranger).expect("the other folder creates");
    assert!(
        Folder::open(&stranger, unkept()).is_err(),
        "a folder without the marker is never taken over"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_paused_folder_is_taken_over_by_its_resume() {
    let root = scratch("folder-resume");
    let path = root.join("Paused");
    let keep = Arc::new(AtomicBool::new(false));
    let folder = Folder::open(&path, keep.clone()).expect("the folder opens");
    keep.store(true, Ordering::SeqCst);
    drop(folder);
    assert!(path.is_dir(), "a paused folder is kept");

    let mut resumed = Folder::open(&path, unkept()).expect("the resume takes the folder over");
    resumed.keep();
    drop(resumed);
    assert!(path.is_dir(), "the resumed folder is still there");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_dropped_folder_is_removed() {
    let root = scratch("folder-drop");
    let path = root.join("Partial");
    let folder = Folder::open(&path, unkept()).expect("the folder opens");
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
