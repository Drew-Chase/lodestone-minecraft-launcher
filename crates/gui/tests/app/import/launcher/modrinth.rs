use std::path::Path;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

use super::*;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lodestone-modrinth-{tag}-{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(dir.join("profiles")).expect("app folder made");
    dir
}

/// Writes an `app.db` with the two tables the import reads, and the rows `statements` add.
async fn database(root: &Path, statements: &[&str]) {
    let options = SqliteConnectOptions::new().filename(root.join(DATABASE)).create_if_missing(true);
    let pool = SqlitePoolOptions::new().connect_with(options).await.expect("database created");
    for statement in [
        "CREATE TABLE instances (id TEXT, path TEXT, name TEXT, install_stage TEXT, applied_content_set_id TEXT)",
        "CREATE TABLE instance_content_sets (id TEXT, instance_id TEXT, game_version TEXT, loader TEXT, loader_version TEXT)",
    ]
    .into_iter()
    .chain(statements.iter().copied())
    {
        sqlx::query(statement).execute(&pool).await.expect("statement runs");
    }
    pool.close().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn the_installed_profiles_are_listed_with_their_applied_versions() {
    let root = scratch("list");
    std::fs::create_dir_all(root.join("profiles").join("Fabulously Optimized").join("mods")).expect("profile made");
    std::fs::create_dir_all(root.join("profiles").join("Blank").join("mods")).expect("profile made");
    std::fs::create_dir_all(root.join("profiles").join("Stuck").join("mods")).expect("profile made");
    database(
        &root,
        &[
            "INSERT INTO instances VALUES ('i1', 'Fabulously Optimized', 'Fabulously Optimized', 'installed', 's1')",
            "INSERT INTO instances VALUES ('i2', 'Blank', 'Blank', 'installed', NULL)",
            "INSERT INTO instances VALUES ('i3', 'Stuck', 'Stuck', 'failed', 's3')",
            "INSERT INTO instance_content_sets VALUES ('s1', 'i1', '1.21.1', 'fabric', '0.16.10')",
            "INSERT INTO instance_content_sets VALUES ('s3', 'i3', '1.20.1', 'fabric', '0.15.0')",
        ],
    )
    .await;

    let listed = {
        let root = root.clone();
        tokio::task::spawn_blocking(move || list(Launcher::Modrinth, &root))
            .await
            .expect("the list runs")
            .expect("the database lists")
    };
    assert_eq!(listed.len(), 2, "a profile that did not finish installing is not listed");
    let applied = listed
        .iter()
        .find(|row| row.name == "Fabulously Optimized")
        .expect("the applied row is listed");
    assert_eq!(applied.summary, Ok("1.21.1 · FABRIC 0.16.10".to_string()));
    assert_eq!(applied.source, root.join("profiles").join("Fabulously Optimized").to_string_lossy());
    let blank = listed.iter().find(|row| row.name == "Blank").expect("the row without a set is listed");
    assert_eq!(blank.summary, Err(no_version()));
    std::fs::remove_dir_all(&root).ok();
}

#[tokio::test(flavor = "multi_thread")]
async fn one_profile_is_read_from_its_folder_under_profiles() {
    let root = scratch("read");
    let folder = root.join("profiles").join("Fabulously Optimized");
    std::fs::create_dir_all(folder.join("mods")).expect("profile made");
    database(
        &root,
        &[
            "INSERT INTO instances VALUES ('i1', 'Fabulously Optimized', 'Fabulously Optimized', 'installed', 's1')",
            "INSERT INTO instance_content_sets VALUES ('s1', 'i1', '26.3', 'fabric', '0.20.1')",
        ],
    )
    .await;
    let meta = {
        let folder = folder.clone();
        tokio::task::spawn_blocking(move || read(&folder))
            .await
            .expect("the read runs")
            .expect("the profile reads")
    };
    assert_eq!(meta.name, "Fabulously Optimized");
    assert_eq!(meta.minecraft, "26.3");
    assert_eq!(meta.loader, Loader::Fabric);
    assert_eq!(meta.loader_version.as_deref(), Some("0.20.1"));
    assert_eq!(meta.game_dir, folder);
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn the_data_folder_is_found_where_its_database_is() {
    let root = scratch("found");
    assert_eq!(instances_in(&root), None, "a folder with no database is not the app's");
    std::fs::write(root.join(DATABASE), b"").expect("database placeholder written");
    assert_eq!(instances_in(&root), Some(root.clone()));
    std::fs::remove_dir_all(&root).ok();
}
