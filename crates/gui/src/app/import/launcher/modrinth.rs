//! The Modrinth app. Its instances are rows of `app.db`, a SQLite database in the app's data folder. A row names its folder
//! under `profiles`, and the content set the instance applies gives its Minecraft version and loader. Some files in a profile
//! are links into the app's shared `synced-options` folder, and the copy follows them. The database is opened read only, so
//! the running app is never written to.

use std::path::{Path, PathBuf};

use sqlx::Row as _;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

use super::{Instance, Launcher, Meta, forge_version, instance, loader_named, no_game_folder, no_version, unrecognised, unsupported_loader};
use crate::app::pages::library::instance::Loader;

/// The database file in the app's data folder.
const DATABASE: &str = "app.db";

/// The columns the import reads for one instance.
struct Listed {
    name: String,
    path: String,
    game_version: Option<String>,
    loader: Option<String>,
    loader_version: Option<String>,
}

/// The app's data folder for `folder`: the folder itself, or its `ModrinthApp` folder, when it holds the database.
pub(super) fn instances_in(folder: &Path) -> Option<PathBuf> {
    [folder.to_path_buf(), folder.join("ModrinthApp")]
        .into_iter()
        .find(|dir| dir.join(DATABASE).is_file())
}

/// The instances the database lists under the data folder `root`. Each one's source is its folder under `profiles`.
pub(super) fn list(launcher: Launcher, root: &Path) -> Result<Vec<Instance>, String> {
    Ok(query(&root.join(DATABASE), None)?
        .into_iter()
        .map(|row| {
            let folder = root.join("profiles").join(&row.path);
            let source = folder.to_string_lossy().into_owned();
            let read = meta(&row, folder);
            instance(launcher, source, row.name.clone(), read)
        })
        .collect())
}

/// Reads the instance in `folder`, which is a folder under `profiles` in the app's data folder.
pub(super) fn read(folder: &Path) -> Result<Meta, String> {
    let root = folder.parent().and_then(Path::parent).ok_or_else(unrecognised)?;
    let key = folder.file_name().and_then(|name| name.to_str()).ok_or_else(unrecognised)?;
    let row = query(&root.join(DATABASE), Some(key))?.into_iter().next().ok_or_else(unrecognised)?;
    meta(&row, folder.to_path_buf())
}

/// What an instance row says: its name, its Minecraft version and loader, and where its game files are.
fn meta(row: &Listed, game_dir: PathBuf) -> Result<Meta, String> {
    let minecraft = row
        .game_version
        .clone()
        .filter(|version| !version.trim().is_empty())
        .ok_or_else(no_version)?;
    let loader_name = row.loader.clone().unwrap_or_else(|| "vanilla".to_string());
    let loader = loader_named(&loader_name).ok_or_else(|| unsupported_loader(&loader_name))?;
    let loader_version = row.loader_version.clone().filter(|version| !version.trim().is_empty());
    let loader_version = match (loader, loader_version) {
        (Loader::Vanilla, _) => None,
        (Loader::Forge, Some(build)) => Some(forge_version(&minecraft, &build)),
        (_, version) => version,
    };
    if !game_dir.is_dir() {
        return Err(no_game_folder());
    }
    Ok(Meta {
        name: row.name.clone(),
        minecraft,
        loader,
        loader_version,
        game_dir,
        skip: Vec::new(),
    })
}

/// The instances in the database, or only the one at `path` under `profiles`. The database opens read only.
fn query(database: &Path, path: Option<&str>) -> Result<Vec<Listed>, String> {
    let database = database.to_path_buf();
    let path = path.map(str::to_string);
    tokio::runtime::Handle::current()
        .block_on(async move {
            let options = SqliteConnectOptions::new().filename(&database).read_only(true);
            let pool = SqlitePoolOptions::new().max_connections(1).connect_with(options).await?;
            let rows = sqlx::query(
                "SELECT i.name, i.path, s.game_version, s.loader, s.loader_version \
                 FROM instances i LEFT JOIN instance_content_sets s ON s.id = i.applied_content_set_id \
                 WHERE i.install_stage = 'installed' AND (?1 IS NULL OR i.path = ?1) ORDER BY i.name",
            )
            .bind(path)
            .fetch_all(&pool)
            .await;
            pool.close().await;
            rows
        })
        .map(|rows| {
            rows.into_iter()
                .map(|row| Listed {
                    name: row.get("name"),
                    path: row.get("path"),
                    game_version: row.get("game_version"),
                    loader: row.get("loader"),
                    loader_version: row.get("loader_version"),
                })
                .collect()
        })
        .map_err(|error| error.to_string())
}

#[cfg(test)]
#[path = "../../../../tests/app/import/launcher/modrinth.rs"]
mod tests;
