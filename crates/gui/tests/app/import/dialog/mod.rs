use std::path::Path;

use chrono::Local;

use super::*;
use crate::app::import::record::ImportOrigin;

/// A checked file of `format`, named `name`, as the file tab holds it.
fn picked(name: &str, format: ImportFormat) -> Picked {
    Picked {
        path: PathBuf::from(format!("/packs/{name}.zip")),
        name: name.to_string(),
        format,
        bytes: 512,
    }
}

/// An import of `name` queued at the time it was read, as the recent list holds it.
fn recorded(name: &str) -> ImportRecord {
    ImportRecord {
        name: name.to_string(),
        format: ImportFormat::Mrpack,
        origin: ImportOrigin::File,
        source: format!("/packs/{name}.mrpack"),
        bytes: 84,
        imported_at: Local::now(),
    }
}

/// A read that is under way, as the root starts one: its abort handle.
fn reading() -> Handle {
    Task::<()>::none().abortable().1
}

#[test]
fn switching_tabs_clears_the_file_and_the_reason_the_last_import_failed() {
    let mut dialog = Dialog {
        picked: Some(picked("Better MC", ImportFormat::CurseForge)),
        error: Some("the last try failed".to_string()),
        ..Dialog::default()
    };
    let _ = dialog.update(Message::TabSelected(Tab::Backup));
    assert_eq!(dialog.tab, Tab::Backup);
    assert!(dialog.picked.is_none());
    assert!(dialog.error.is_none());
}

#[test]
fn a_file_is_lit_over_the_file_tabs_and_never_over_the_address_tab() {
    let mut dialog = Dialog::default();
    let _ = dialog.update(Message::Hovering(true));
    assert!(dialog.hovered, "a file over the file tab lights the zone");
    let _ = dialog.update(Message::TabSelected(Tab::Url));
    let _ = dialog.update(Message::Hovering(true));
    assert!(!dialog.hovered, "the address tab takes no file");
}

#[test]
fn a_dropped_file_is_not_taken_on_the_address_tab_and_its_zone_goes_dark() {
    let mut dialog = Dialog::default();
    let _ = dialog.update(Message::Hovering(true));
    let _ = dialog.update(Message::Dropped(PathBuf::from("/packs/better.zip")));
    assert!(!dialog.hovered, "a drop ends the highlight");
    let _ = dialog.update(Message::TabSelected(Tab::Url));
    let _ = dialog.update(Message::Dropped(PathBuf::from("/packs/better.zip")));
    assert!(dialog.picked.is_none());
}

#[test]
fn a_checked_file_is_held_and_a_refused_one_says_why() {
    let mut dialog = Dialog::default();
    let _ = dialog.update(Message::Inspected(Ok(picked("Better MC", ImportFormat::CurseForge))));
    assert_eq!(dialog.picked.as_ref().map(|picked| picked.name.as_str()), Some("Better MC"));
    let _ = dialog.update(Message::Inspected(Err("not a modpack".to_string())));
    assert!(dialog.picked.is_none());
    assert_eq!(dialog.error.as_deref(), Some("not a modpack"));
}

#[test]
fn import_is_lit_only_with_something_to_import_and_no_read_under_way() {
    let mut dialog = Dialog::default();
    assert!(!dialog.ready(), "the file tab holds nothing yet");
    let _ = dialog.update(Message::Inspected(Ok(picked("Better MC", ImportFormat::CurseForge))));
    assert!(dialog.ready());
    dialog.begin(reading());
    assert!(!dialog.ready(), "a read under way is not started again");

    let _ = dialog.update(Message::TabSelected(Tab::Url));
    assert!(!dialog.ready(), "an empty address imports nothing");
    let _ = dialog.update(Message::UrlChanged(
        "  https://www.curseforge.com/minecraft/modpacks/better-mc  ".to_string(),
    ));
    dialog.abandon();
    assert!(dialog.ready());
}

#[test]
fn the_work_of_each_tab_is_its_file_or_address_and_a_reimport_is_the_recorded_one() {
    let mut dialog = Dialog::default();
    let _ = dialog.update(Message::Inspected(Ok(picked("Better MC", ImportFormat::CurseForge))));
    assert!(matches!(dialog.work(None), Some(Work::File(path)) if path == Path::new("/packs/Better MC.zip")));

    let _ = dialog.update(Message::TabSelected(Tab::Backup));
    let _ = dialog.update(Message::Inspected(Ok(picked("Worlds", ImportFormat::Lodestone))));
    assert!(matches!(dialog.work(None), Some(Work::Backup(_))));

    let _ = dialog.update(Message::TabSelected(Tab::Url));
    let _ = dialog.update(Message::UrlChanged(
        " https://www.curseforge.com/minecraft/modpacks/better-mc ".to_string(),
    ));
    assert!(matches!(dialog.work(None), Some(Work::Url(address)) if address == "https://www.curseforge.com/minecraft/modpacks/better-mc"));

    let _ = dialog.update(Message::Recent(Ok(vec![recorded("Fabulously Optimized")])));
    assert!(matches!(dialog.work(Some(0)), Some(Work::Again(record)) if record.name == "Fabulously Optimized"));
    assert!(dialog.work(Some(4)).is_none(), "a row that is not listed starts nothing");
}

#[test]
fn a_failed_read_keeps_the_dialog_open_with_its_reason_and_ends_the_read() {
    let mut dialog = Dialog::default();
    dialog.begin(reading());
    assert!(dialog.busy());
    dialog.failed("the pack could not be downloaded".to_string());
    assert!(!dialog.busy());
    assert_eq!(dialog.error.as_deref(), Some("the pack could not be downloaded"));
}

#[test]
fn closing_during_a_read_stops_it() {
    let mut dialog = Dialog::default();
    dialog.begin(reading());
    dialog.abandon();
    assert!(!dialog.busy());
}

#[test]
fn the_recent_imports_are_listed_as_they_were_read() {
    let mut dialog = Dialog::default();
    let _ = dialog.update(Message::Recent(Ok(vec![recorded("One"), recorded("Two")])));
    assert_eq!(dialog.recent.len(), 2);
    assert!(matches!(dialog.work(Some(1)), Some(Work::Again(record)) if record.name == "Two"));
}
