use super::*;
use crate::app::pages::library::instance::{Loader, fixture};
use chrono::Local;

/// A kept notification of `kind`, numbered `id`, that has or has not been read.
fn kept(id: i64, kind: Kind, read: bool) -> Notification {
    Notification {
        id,
        kind,
        subject: format!("n{id}"),
        detail: String::new(),
        instance_id: None,
        target: None,
        created_at: Local::now(),
        read,
    }
}

#[test]
fn the_bell_counts_only_the_notifications_not_yet_read() {
    let state = Notifications {
        items: vec![kept(1, Kind::Crash, false), kept(2, Kind::InstallDone, true), kept(3, Kind::Crash, false)],
        open: false,
        checking: false,
        checked_at: None,
    };
    assert_eq!(state.unread(), 2);
}

#[test]
fn the_panel_starts_shut() {
    assert!(!Notifications::default().is_open());
}

#[test]
fn an_install_that_ended_keys_its_draft_by_its_job() {
    let instance = fixture("abc", "Aether", Loader::Fabric);
    let draft = install_done(7, &instance);
    assert_eq!(draft.kind, Kind::InstallDone);
    assert_eq!(draft.subject, "Aether");
    assert_eq!(draft.instance_id.as_deref(), Some("abc"));
    assert!(draft.key.starts_with("install:7:"), "{}", draft.key);
}

#[test]
fn a_failed_install_keeps_the_reason_it_gave() {
    let draft = install_failed(8, "Aether".to_owned(), "the server refused the jar");
    assert_eq!(draft.kind, Kind::InstallFailed);
    assert_eq!(draft.detail, "the server refused the jar");
    assert_eq!(draft.instance_id, None);
}

#[test]
fn an_export_names_its_file_and_points_at_it() {
    let draft = export_done(3, Path::new("exports/Pack.mrpack"));
    assert_eq!(draft.kind, Kind::ExportDone);
    assert_eq!(draft.subject, "Pack.mrpack");
    assert!(draft.target.as_deref().is_some_and(|target| target.ends_with("Pack.mrpack")));
}
