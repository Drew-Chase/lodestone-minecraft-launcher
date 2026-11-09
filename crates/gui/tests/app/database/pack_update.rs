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

/// A file a release of `instance` shipped at `path`, with the hash `sha1`.
fn shipped(instance: &str, path: &str, sha1: Option<&str>) -> PackFile {
    PackFile {
        instance: instance.to_string(),
        path: path.to_string(),
        sha1: sha1.map(str::to_string),
    }
}

/// A mod recorded against `instance`, as the jar `file_name` of the project `project_id`.
fn source(instance: &str, project_id: &str, file_name: &str) -> ModSource {
    ModSource {
        instance: instance.to_string(),
        platform: "modrinth".to_string(),
        project_id: project_id.to_string(),
        version_id: format!("{project_id}-v2"),
        name: format!("Project {project_id}"),
        file_name: file_name.to_string(),
        sha1: Some("ab".repeat(20)),
    }
}

/// An instance installed from a platform on release 1.0, with `old.jar` and `keep.jar` of its pack.
async fn installed_on_one(db: &Database) -> Instance {
    let mut installed = fixture("u", "Pack", Loader::Fabric);
    installed.source_platform = Some("modrinth".to_string());
    installed.source_project_id = Some("P".to_string());
    installed.source_version = Some("1.0".to_string());
    installed.source_version_id = Some("v1".to_string());
    let files = vec![shipped("u", "mods/old.jar", Some("o")), shipped("u", "config/a.toml", Some("c1"))];
    let sources = vec![source("u", "P", "old.jar"), source("u", "Q", "keep.jar")];
    db.insert_instance_with_files(&installed, &sources, &files)
        .await
        .expect("insert succeeds");
    installed
}

#[tokio::test]
async fn a_commit_moves_the_row_replaces_the_files_and_drops_the_removed_jar_and_records_the_token() {
    let db = connect_in_memory().await;
    let installed = installed_on_one(&db).await;

    let mut updated = installed.clone();
    updated.mc_version = "1.21.1".to_string();
    updated.version_id = Some("fabric-1.21.1".to_string());
    updated.source_version = Some("2.0".to_string());
    updated.source_version_id = Some("v2".to_string());
    updated.mod_count = 2;
    let files = vec![shipped("u", "mods/new.jar", Some("n")), shipped("u", "config/a.toml", Some("c2"))];
    let sources = vec![source("u", "P", "new.jar")];
    db.commit_pack_update(&updated, &files, &["old.jar".to_string()], &sources, &[], "tok-2")
        .await
        .expect("committed");

    let listed = db.instances().await.expect("listed");
    let row = listed.iter().find(|instance| instance.id == "u").expect("the row is listed");
    assert_eq!(row.mc_version, "1.21.1");
    assert_eq!(row.version_id.as_deref(), Some("fabric-1.21.1"));
    assert_eq!(row.source_version.as_deref(), Some("2.0"));
    assert_eq!(row.source_version_id.as_deref(), Some("v2"));
    assert_eq!(row.mod_count, 2);
    assert_eq!(
        db.pack_files_of("u").await.expect("listed"),
        vec![shipped("u", "config/a.toml", Some("c2")), shipped("u", "mods/new.jar", Some("n"))]
    );
    let records = db.mod_sources_of("u").await.expect("listed");
    assert!(
        records.iter().all(|record| record.file_name != "old.jar"),
        "the removed jar's record is gone"
    );
    assert!(records.iter().any(|record| record.file_name == "new.jar"));
    assert!(
        records.iter().any(|record| record.file_name == "keep.jar"),
        "a jar the update did not touch keeps its record"
    );
    assert_eq!(
        db.committed_pack_updates().await.expect("tokens load"),
        BTreeMap::from([("u".to_string(), "tok-2".to_string())])
    );
}

#[tokio::test]
async fn a_later_commit_replaces_the_token_the_instance_recorded() {
    let db = connect_in_memory().await;
    let installed = installed_on_one(&db).await;
    db.commit_pack_update(&installed, &[], &[], &[], &[], "first").await.expect("committed");
    db.commit_pack_update(&installed, &[], &[], &[], &[], "second").await.expect("committed");
    assert_eq!(
        db.committed_pack_updates().await.expect("tokens load"),
        BTreeMap::from([("u".to_string(), "second".to_string())])
    );
}

#[tokio::test]
async fn deleting_an_instance_forgets_its_commit_token() {
    let db = connect_in_memory().await;
    let installed = installed_on_one(&db).await;
    db.commit_pack_update(&installed, &[], &[], &[], &[], "tok").await.expect("committed");
    db.delete_instance("u").await.expect("deleted");
    assert!(db.committed_pack_updates().await.expect("tokens load").is_empty());
}
