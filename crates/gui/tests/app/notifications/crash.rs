use super::*;
use std::fs::{self, File};
use std::time::Duration;

/// A fresh instance folder named for `tag`, with an empty `crash-reports` folder inside it.
fn scratch(tag: &str) -> PathBuf {
    let folder = std::env::temp_dir().join(format!("lodestone-crash-test-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&folder);
    fs::create_dir_all(folder.join("crash-reports")).expect("the scratch folder is created");
    folder
}

/// Writes `name` into `folder`'s `crash-reports` folder, last changed at `modified`.
fn report(folder: &Path, name: &str, modified: SystemTime) -> PathBuf {
    let path = folder.join("crash-reports").join(name);
    File::create(&path)
        .expect("the report is written")
        .set_modified(modified)
        .expect("the report's time is set");
    path
}

#[test]
fn an_instance_without_a_reports_folder_has_no_report() {
    let folder = std::env::temp_dir().join(format!("lodestone-crash-test-missing-{}", std::process::id()));
    assert_eq!(newest_report_since(&folder, SystemTime::UNIX_EPOCH), None);
}

#[test]
fn the_newest_text_report_written_since_the_start_is_chosen() {
    let folder = scratch("newest");
    let start = SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    report(&folder, "before-the-run.txt", start - Duration::from_secs(60));
    report(&folder, "first.txt", start + Duration::from_secs(10));
    let newest = report(&folder, "second.txt", start + Duration::from_secs(20));
    report(&folder, "game.log", start + Duration::from_secs(30));
    assert_eq!(newest_report_since(&folder, start), Some(newest));
}

#[test]
fn a_run_that_wrote_no_report_finds_none() {
    let folder = scratch("stale");
    let start = SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    report(&folder, "earlier.txt", start - Duration::from_secs(5));
    assert_eq!(newest_report_since(&folder, start), None);
}
