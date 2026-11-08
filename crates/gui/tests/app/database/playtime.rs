use chrono::TimeZone;
use sqlx::sqlite::SqlitePoolOptions;

use super::*;
use crate::app::pages::library::instance::{Loader, fixture};

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

/// A moment `seconds` after a fixed point, so that launches order by when they happened.
fn at(seconds: i64) -> DateTime<Local> {
    Local.timestamp_opt(1_700_000_000 + seconds, 0).single().expect("a valid local time")
}

#[tokio::test]
async fn a_launch_is_the_last_played_time_and_an_older_one_never_replaces_it() {
    let db = connect().await;
    let instance = fixture("0f3c-played", "Aether Legacy", Loader::Fabric);
    db.insert_instance(&instance, &[]).await.expect("insert succeeds");
    assert_eq!(db.instances().await.expect("instances load")[0].last_played, None);

    db.record_launch(&instance.id, at(100)).await.expect("the launch is recorded");
    db.record_launch(&instance.id, at(50)).await.expect("the older launch is recorded");
    let loaded = db.instances().await.expect("instances load");
    assert_eq!(loaded[0].last_played.map(|played| played.timestamp()), Some(1_700_000_100));
}

#[tokio::test]
async fn credited_seconds_add_up_and_leave_the_state_alone() {
    let db = connect().await;
    let instance = fixture("9a1b-idle", "Vanilla Survival", Loader::Vanilla);
    db.insert_instance(&instance, &[]).await.expect("insert succeeds");

    db.credit_playtime(&instance.id, 60).await.expect("the first credit lands");
    db.credit_playtime(&instance.id, 30).await.expect("the second credit lands");
    let loaded = db.instances().await.expect("instances load");
    assert_eq!(loaded[0].playtime_seconds, 90);
    assert_eq!(loaded[0].state, instance.state);
}
