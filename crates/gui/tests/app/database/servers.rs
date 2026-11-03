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

/// A moment `seconds` after a fixed point, so that servers have distinct times.
fn at(seconds: i64) -> DateTime<Local> {
    Local.timestamp_opt(1_700_000_000 + seconds, 0).single().expect("a valid local time")
}

#[tokio::test]
async fn a_saved_server_reads_back_with_its_name_address_and_mark() {
    let db = connect().await;
    db.add_server("Hypixel", "mc.hypixel.net", true, at(0)).await.expect("saves");
    let saved = db.saved_servers().await.expect("reads");
    assert_eq!(saved.len(), 1);
    assert_eq!(saved[0].name, "Hypixel");
    assert_eq!(saved[0].address, "mc.hypixel.net");
    assert!(saved[0].favorite);
    assert_eq!(saved[0].added_at, at(0));
}

#[tokio::test]
async fn favorites_come_first_and_the_rest_are_by_name() {
    let db = connect().await;
    db.add_server("zeta", "z.example", false, at(0)).await.expect("saves");
    db.add_server("Alpha", "a.example", false, at(1)).await.expect("saves");
    db.add_server("Mid", "m.example", true, at(2)).await.expect("saves");
    let names: Vec<String> = db.saved_servers().await.expect("reads").into_iter().map(|server| server.name).collect();
    assert_eq!(names, ["Mid", "Alpha", "zeta"]);
}

#[tokio::test]
async fn an_address_is_saved_once() {
    let db = connect().await;
    db.add_server("One", "one.example", false, at(0)).await.expect("saves");
    assert!(db.add_server("Again", "one.example", false, at(1)).await.is_err());
}

#[tokio::test]
async fn updating_and_removing_change_the_saved_row() {
    let db = connect().await;
    db.add_server("Old", "old.example", false, at(0)).await.expect("saves");
    let id = db.saved_servers().await.expect("reads")[0].id;
    db.update_server(id, "New", "new.example:25566", true).await.expect("updates");
    let updated = db.saved_servers().await.expect("reads");
    assert_eq!(updated[0].name, "New");
    assert_eq!(updated[0].address, "new.example:25566");
    assert!(updated[0].favorite);
    db.remove_server(id).await.expect("removes");
    assert!(db.saved_servers().await.expect("reads").is_empty());
}
