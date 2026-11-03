use super::*;
use chrono::{Duration, TimeZone};

/// A notification of `kind` about "Aether", with no instance, target or detail.
fn item(kind: Kind) -> Notification {
    Notification {
        id: 1,
        kind,
        subject: "Aether".to_owned(),
        detail: String::new(),
        instance_id: None,
        target: None,
        created_at: Local::now(),
        read: false,
    }
}

#[test]
fn a_crash_offers_its_log_and_its_report() {
    let crash = Notification {
        instance_id: Some("abc".to_owned()),
        target: Some("crash-reports/crash-1.txt".to_owned()),
        ..item(Kind::Crash)
    };
    assert_eq!(
        actions(&crash),
        vec![
            ("notifications.action.view_log", Action::Logs("abc".to_owned())),
            (
                "notifications.action.open_report",
                Action::OpenFile(PathBuf::from("crash-reports/crash-1.txt"))
            ),
        ]
    );
}

#[test]
fn a_crash_with_no_report_offers_only_the_log() {
    let crash = Notification {
        instance_id: Some("abc".to_owned()),
        ..item(Kind::Crash)
    };
    assert_eq!(actions(&crash), vec![("notifications.action.view_log", Action::Logs("abc".to_owned()))]);
}

#[test]
fn a_modpack_update_opens_the_pack_on_its_platform() {
    let update = Notification {
        target: Some("modrinth/abc123".to_owned()),
        ..item(Kind::ModpackUpdate)
    };
    assert_eq!(
        actions(&update),
        vec![(
            "notifications.action.open_modpack",
            Action::Modpack {
                platform: "modrinth".to_owned(),
                project: "abc123".to_owned(),
                name: "Aether".to_owned(),
            },
        )]
    );
}

#[test]
fn a_mod_update_reviews_the_instances_mods() {
    let update = Notification {
        instance_id: Some("abc".to_owned()),
        ..item(Kind::ModUpdate)
    };
    assert_eq!(
        actions(&update),
        vec![("notifications.action.review_mods", Action::Mods("abc".to_owned()))]
    );
}

#[test]
fn a_launcher_update_offers_the_update() {
    assert_eq!(
        actions(&item(Kind::LauncherUpdate)),
        vec![("notifications.action.update", Action::Update)]
    );
}

#[test]
fn a_failed_install_or_export_offers_nothing() {
    assert!(actions(&item(Kind::InstallFailed)).is_empty());
    assert!(actions(&item(Kind::ExportFailed)).is_empty());
}

#[test]
fn an_age_reads_in_minutes_hours_and_days() {
    let now = Local.timestamp_opt(1_700_100_000, 0).single().expect("a valid local time");
    assert_eq!(ago(now, now), lang::t("notifications.ago.now"));
    assert_eq!(ago(now - Duration::minutes(5), now), lang::t_count("notifications.ago.minutes", 5));
    assert_eq!(ago(now - Duration::hours(3), now), lang::t_count("notifications.ago.hours", 3));
    assert_eq!(ago(now - Duration::days(2), now), lang::t_count("notifications.ago.days", 2));
}
