use super::*;
use sqlx::sqlite::SqlitePoolOptions;

/// Opens an in-memory database with the migrations applied, as the launcher does at start.
async fn connect() -> Database {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("an in-memory database opens");
    sqlx::migrate!("./migrations").run(&pool).await.expect("the migrations apply");
    Database { pool }
}

#[tokio::test]
async fn a_remembered_choice_is_stored_read_back_changed_and_forgotten() {
    let db = connect().await;
    assert!(db.server_pack_choices().await.expect("the choices read").is_empty());

    db.set_server_pack_choice("sodium", Some("include")).await.expect("the choice is stored");
    assert_eq!(
        db.server_pack_choices().await.expect("reads"),
        vec![("sodium".to_string(), "include".to_string())]
    );

    db.set_server_pack_choice("sodium", Some("exclude")).await.expect("the choice is changed");
    assert_eq!(
        db.server_pack_choices().await.expect("reads"),
        vec![("sodium".to_string(), "exclude".to_string())]
    );

    db.set_server_pack_choice("sodium", None).await.expect("the choice is forgotten");
    assert!(db.server_pack_choices().await.expect("reads").is_empty());
}

#[tokio::test]
async fn choices_are_kept_per_mod_id() {
    let db = connect().await;
    db.set_server_pack_choice("lithium", Some("include")).await.expect("stored");
    db.set_server_pack_choice("iris", Some("exclude")).await.expect("stored");
    let mut choices = db.server_pack_choices().await.expect("reads");
    choices.sort();
    assert_eq!(
        choices,
        vec![
            ("iris".to_string(), "exclude".to_string()),
            ("lithium".to_string(), "include".to_string())
        ]
    );
}
