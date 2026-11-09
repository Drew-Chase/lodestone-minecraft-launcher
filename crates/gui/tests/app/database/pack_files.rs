use super::*;
use crate::app::pages::library::instance::{Loader, fixture};
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

/// A file the release of `instance` shipped at `path`, with the hash `sha1`.
fn shipped(instance: &str, path: &str, sha1: Option<&str>) -> PackFile {
    PackFile {
        instance: instance.to_string(),
        path: path.to_string(),
        sha1: sha1.map(str::to_string),
    }
}

#[tokio::test]
async fn the_files_of_an_install_are_listed_in_path_order() {
    let db = connect_in_memory().await;
    let mut installed = fixture("pk", "Pack", Loader::Fabric);
    installed.source_platform = Some("modrinth".to_string());
    installed.source_project_id = Some("p1".to_string());
    installed.source_version = Some("1.0".to_string());
    installed.source_version_id = Some("v1".to_string());
    let files = vec![shipped("pk", "mods/b.jar", Some("bb")), shipped("pk", "config/a.toml", None)];
    db.insert_instance_with_files(&installed, &[], &files).await.expect("insert succeeds");
    let listed = db.pack_files_of("pk").await.expect("listed");
    assert_eq!(
        listed,
        vec![shipped("pk", "config/a.toml", None), shipped("pk", "mods/b.jar", Some("bb"))]
    );
}

#[tokio::test]
async fn the_installed_release_id_is_kept_apart_from_a_baseline_number() {
    let db = connect_in_memory().await;
    // A baseline: the update check recorded a number, and no install ever recorded an id.
    let mut baseline = fixture("bl", "Baseline", Loader::Fabric);
    baseline.source_version = Some("2.0".to_string());
    let mut installed = fixture("in", "Installed", Loader::Fabric);
    installed.source_version = Some("1.0".to_string());
    installed.source_version_id = Some("v-1".to_string());
    db.insert_instance(&baseline, &[]).await.expect("insert succeeds");
    db.insert_instance(&installed, &[]).await.expect("insert succeeds");
    let listed = db.instances().await.expect("listed");
    let find = |id: &str| listed.iter().find(|instance| instance.id == id).expect("the row is listed").clone();
    assert_eq!(find("bl").source_version.as_deref(), Some("2.0"));
    assert_eq!(find("bl").source_version_id, None);
    assert_eq!(find("in").source_version_id.as_deref(), Some("v-1"));
}

#[tokio::test]
async fn a_path_written_again_takes_its_new_hash() {
    let db = connect_in_memory().await;
    let installed = fixture("re", "Re", Loader::Vanilla);
    db.insert_instance_with_files(&installed, &[], &[shipped("re", "config/a.toml", Some("old"))])
        .await
        .expect("insert succeeds");
    let mut transaction = db.pool.begin().await.expect("transaction opens");
    write_pack_files(&mut transaction, "re", &[shipped("re", "config/a.toml", Some("new"))])
        .await
        .expect("written");
    transaction.commit().await.expect("committed");
    assert_eq!(
        db.pack_files_of("re").await.expect("listed"),
        vec![shipped("re", "config/a.toml", Some("new"))]
    );
}

#[tokio::test]
async fn deleting_an_instance_forgets_its_files_and_no_others() {
    let db = connect_in_memory().await;
    db.insert_instance_with_files(&fixture("gone", "Gone", Loader::Vanilla), &[], &[shipped("gone", "mods/a.jar", None)])
        .await
        .expect("insert succeeds");
    db.insert_instance_with_files(&fixture("kept", "Kept", Loader::Vanilla), &[], &[shipped("kept", "mods/a.jar", None)])
        .await
        .expect("insert succeeds");
    db.delete_instance("gone").await.expect("deleted");
    assert!(db.pack_files_of("gone").await.expect("listed").is_empty());
    assert_eq!(db.pack_files_of("kept").await.expect("listed").len(), 1);
}
