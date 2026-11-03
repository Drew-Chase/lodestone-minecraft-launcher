use super::*;
use chrono::TimeZone;
use sqlx::sqlite::SqlitePoolOptions;

/// Opens an in-memory database with the migrations applied.
async fn connect() -> Database {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("an in-memory database opens");
    sqlx::migrate!("./migrations").run(&pool).await.expect("the migrations apply");
    Database { pool }
}

/// A moment `seconds` after a fixed point, so that rows sort by when they were made.
fn at(seconds: i64) -> DateTime<Local> {
    Local.timestamp_opt(1_700_000_000 + seconds, 0).single().expect("a valid local time")
}

/// A draft of `kind` about `subject`, kept under `key` and made at `created`.
fn draft(key: &str, kind: Kind, subject: &str, created: DateTime<Local>) -> Draft {
    Draft {
        key: key.to_owned(),
        kind,
        subject: subject.to_owned(),
        detail: String::new(),
        instance_id: None,
        target: None,
        created_at: created,
    }
}

#[tokio::test]
async fn an_event_recorded_twice_is_kept_once_with_its_first_row() {
    let db = connect().await;
    db.record_notification(&draft("launcher_update:v2.0.0", Kind::LauncherUpdate, "v2.0.0", at(0)))
        .await
        .expect("records");
    db.record_notification(&draft("launcher_update:v2.0.0", Kind::LauncherUpdate, "v2.0.0", at(9)))
        .await
        .expect("records again");
    let kept = db.notifications().await.expect("reads");
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].created_at, at(0));
}

#[tokio::test]
async fn the_newest_come_first_and_marking_them_read_clears_the_unread() {
    let db = connect().await;
    db.record_notification(&draft("crash:a:1", Kind::Crash, "Old", at(0)))
        .await
        .expect("records");
    db.record_notification(&draft("install:2:1", Kind::InstallDone, "New", at(10)))
        .await
        .expect("records");
    let kept = db.notifications().await.expect("reads");
    let subjects: Vec<&str> = kept.iter().map(|notification| notification.subject.as_str()).collect();
    assert_eq!(subjects, ["New", "Old"]);
    assert!(kept.iter().all(|notification| !notification.read));
    db.mark_notifications_read(at(20)).await.expect("marks");
    assert!(db.notifications().await.expect("reads").iter().all(|notification| notification.read));
}

#[tokio::test]
async fn dismissing_removes_one_row_and_clearing_removes_the_rest() {
    let db = connect().await;
    db.record_notification(&draft("crash:a:1", Kind::Crash, "One", at(0)))
        .await
        .expect("records");
    db.record_notification(&draft("crash:b:1", Kind::Crash, "Two", at(1)))
        .await
        .expect("records");
    let older = db.notifications().await.expect("reads")[1].id;
    db.dismiss_notification(older).await.expect("dismisses");
    let kept = db.notifications().await.expect("reads");
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].subject, "Two");
    db.clear_notifications().await.expect("clears");
    assert!(db.notifications().await.expect("reads").is_empty());
}

#[tokio::test]
async fn only_the_newest_rows_are_kept() {
    let db = connect().await;
    for index in 0..KEPT + 5 {
        let row = draft(&format!("crash:{index}"), Kind::Crash, &format!("n{index}"), at(index));
        db.record_notification(&row).await.expect("records");
    }
    let kept = db.notifications().await.expect("reads");
    assert_eq!(kept.len(), usize::try_from(KEPT).expect("a small count"));
    assert_eq!(kept[0].subject, format!("n{}", KEPT + 4));
}
