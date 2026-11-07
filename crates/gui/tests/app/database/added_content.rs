use super::*;
use sqlx::sqlite::SqlitePoolOptions;

/// An in-memory database with the migrations applied.
async fn connect_in_memory() -> Database {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("in-memory db opens");
    sqlx::migrate!("./migrations").run(&pool).await.expect("migrations apply");
    Database { pool }
}

/// A pack or datapack added to `instance` under `scope`, as the release `version_id` written as `file_name`.
fn added(instance: &str, scope: &str, project_id: &str, version_id: &str, file_name: &str) -> AddedContent {
    AddedContent {
        instance: instance.to_string(),
        scope: scope.to_string(),
        kind: "resourcepacks".to_string(),
        platform: "modrinth".to_string(),
        project_id: project_id.to_string(),
        version_id: version_id.to_string(),
        name: "Fresh Animations".to_string(),
        file_name: file_name.to_string(),
    }
}

#[tokio::test]
async fn added_content_is_listed_once_it_is_recorded() {
    let db = connect_in_memory().await;
    db.record_added(&added("a", "", "p1", "v1", "fresh.zip")).await.expect("recorded");
    assert_eq!(db.added_content().await.expect("listed"), vec![added("a", "", "p1", "v1", "fresh.zip")]);
}

#[tokio::test]
async fn a_newer_release_of_the_same_project_replaces_its_record() {
    let db = connect_in_memory().await;
    db.record_added(&added("a", "", "p1", "v1", "fresh-1.zip")).await.expect("recorded");
    db.record_added(&added("a", "", "p1", "v2", "fresh-2.zip")).await.expect("recorded again");
    let listed = db.added_content().await.expect("listed");
    assert_eq!(listed, vec![added("a", "", "p1", "v2", "fresh-2.zip")]);
}

#[tokio::test]
async fn the_same_project_in_another_scope_is_a_record_of_its_own() {
    let db = connect_in_memory().await;
    db.record_added(&added("a", "", "p1", "v1", "fresh.zip")).await.expect("recorded");
    db.record_added(&added("a", "world-one", "p1", "v1", "fresh.zip"))
        .await
        .expect("recorded");
    assert_eq!(db.added_content().await.expect("listed").len(), 2);
}

#[tokio::test]
async fn forgetting_a_file_removes_only_its_record() {
    let db = connect_in_memory().await;
    db.record_added(&added("a", "", "p1", "v1", "fresh.zip")).await.expect("recorded");
    db.record_added(&added("a", "", "p2", "v1", "other.zip")).await.expect("recorded");
    db.forget_added("a", "", "resourcepacks", "fresh.zip").await.expect("forgotten");
    assert_eq!(db.added_content().await.expect("listed"), vec![added("a", "", "p2", "v1", "other.zip")]);
}

#[tokio::test]
async fn deleting_an_instance_removes_its_added_content_and_no_others() {
    let db = connect_in_memory().await;
    db.record_added(&added("a", "", "p1", "v1", "fresh.zip")).await.expect("recorded");
    db.record_added(&added("b", "", "p1", "v1", "fresh.zip")).await.expect("recorded");
    db.delete_instance("a").await.expect("deleted");
    assert_eq!(db.added_content().await.expect("listed"), vec![added("b", "", "p1", "v1", "fresh.zip")]);
}
