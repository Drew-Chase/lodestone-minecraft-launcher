//! SQLite persistence for the launcher's data — instances live in a
//! database file next to the credential store, schema versioned through
//! sqlx migrations.

use std::path::PathBuf;

use chrono::{DateTime, Local, Utc};
use sqlx::Row;
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};

use crate::app::pages::library::instance::{Biome, Instance, InstanceIcon, InstanceState, Loader};

/// How many icon recents are kept. The icon editor shows the newest few.
const RECENT_ICON_LIMIT: i64 = 8;

/// The launcher's database handle. Cheap to clone; the pool owns the
/// connections.
#[derive(Clone)]
pub struct Database {
    pool: SqlitePool,
}

impl std::fmt::Debug for Database {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Database").finish()
    }
}

impl Database {
    /// Opens (creating when missing) the launcher database and brings the
    /// schema up to date.
    pub async fn connect() -> Result<Self, String> {
        let mut path = dirs::config_dir().ok_or("no config directory")?;
        path.push("lodestone");
        std::fs::create_dir_all(&path).map_err(|error| format!("creating {}: {error}", path.display()))?;
        path.push("launcher.db");
        let url = format!("sqlite://{}?mode=rwc", path.display());
        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .connect(&url)
            .await
            .map_err(|error| error.to_string())?;
        sqlx::migrate!("./migrations").run(&pool).await.map_err(|error| error.to_string())?;
        Ok(Self { pool })
    }

    /// Loads every instance, most recently installed first.
    ///
    /// File paths come back relative to the instance folder; the root makes
    /// them absolute with [`Instance::resolve_files`].
    pub async fn instances(&self) -> Result<Vec<Instance>, String> {
        let rows = sqlx::query(
            "SELECT id, name, mc_version, loader, loader_version, playtime_seconds, mod_count, \
             last_played_at, installed_at, pinned, state, biome, art_seed, art_path, \
             icon_background, icon_symbol, icon_path, directory_name, java_component, version_id \
             FROM instances",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|error| error.to_string())?;
        Ok(rows
            .into_iter()
            .map(|row| {
                let last_played_at: Option<i64> = row.get("last_played_at");
                let last_played = last_played_at
                    .and_then(|seconds| DateTime::<Utc>::from_timestamp(seconds, 0))
                    .map(|utc| utc.with_timezone(&Local));
                let installed_at = DateTime::<Utc>::from_timestamp(row.get::<i64, _>("installed_at"), 0)
                    .map(|utc| utc.with_timezone(&Local))
                    .unwrap_or_else(Local::now);
                Instance {
                    id: row.get("id"),
                    name: row.get("name"),
                    mc_version: row.get("mc_version"),
                    loader: Loader::from_db(&row.get::<String, _>("loader")),
                    loader_version: row.get("loader_version"),
                    playtime_seconds: row.get::<i64, _>("playtime_seconds") as u32,
                    mod_count: row.get::<i64, _>("mod_count") as u32,
                    last_played,
                    installed_at,
                    pinned: row.get::<i64, _>("pinned") != 0,
                    state: InstanceState::from_db(&row.get::<String, _>("state")),
                    biome: Biome::from_db(&row.get::<String, _>("biome")),
                    art_seed: row.get::<i64, _>("art_seed") as u64,
                    art_path: row.get::<Option<String>, _>("art_path").map(PathBuf::from),
                    icon: InstanceIcon::from_db(row.get("icon_background"), row.get("icon_symbol"), row.get("icon_path")),
                    directory_name: row.get("directory_name"),
                    java_component: row.get("java_component"),
                    version_id: row.get("version_id"),
                }
            })
            .collect())
    }

    /// Writes a new instance row. The caller has already created the folder
    /// and written its files.
    pub async fn insert_instance(&self, instance: &Instance) -> Result<(), String> {
        let (icon_background, icon_symbol, icon_path) = instance.icon.to_db();
        sqlx::query(
            "INSERT INTO instances \
             (id, name, mc_version, loader, loader_version, playtime_seconds, mod_count, \
              last_played_at, installed_at, pinned, state, biome, art_seed, art_path, \
              icon_background, icon_symbol, icon_path, directory_name, java_component, version_id) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&instance.id)
        .bind(&instance.name)
        .bind(&instance.mc_version)
        .bind(instance.loader.to_db())
        .bind(&instance.loader_version)
        .bind(instance.playtime_seconds as i64)
        .bind(instance.mod_count as i64)
        .bind(instance.last_played.map(|at| at.timestamp()))
        .bind(instance.installed_at.timestamp())
        .bind(instance.pinned as i64)
        .bind(instance.state.to_db())
        .bind(instance.biome.to_db())
        .bind(instance.art_seed as i64)
        .bind(instance.art_path.as_ref().map(|path| path.to_string_lossy().into_owned()))
        .bind(icon_background)
        .bind(icon_symbol)
        .bind(icon_path)
        .bind(&instance.directory_name)
        .bind(&instance.java_component)
        .bind(&instance.version_id)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(|error| error.to_string())
    }

    /// Sets an instance's state: `playing` while its game runs.
    pub async fn set_state(&self, id: &str, state: InstanceState) -> Result<(), String> {
        sqlx::query("UPDATE instances SET state = ? WHERE id = ?")
            .bind(state.to_db())
            .bind(id)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    /// Ends a play session: the instance goes idle, its playtime grows by `seconds`, and it was
    /// last played when the session started.
    pub async fn finish_session(&self, id: &str, started: DateTime<Local>, seconds: u32) -> Result<(), String> {
        sqlx::query("UPDATE instances SET state = 'idle', playtime_seconds = playtime_seconds + ?, last_played_at = ? WHERE id = ?")
            .bind(i64::from(seconds))
            .bind(started.timestamp())
            .bind(id)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    /// Sets every instance a previous run left `playing` back to idle. The launcher cannot follow a
    /// game across a restart, so none can still be running.
    pub async fn reset_playing(&self) -> Result<(), String> {
        sqlx::query("UPDATE instances SET state = 'idle' WHERE state = 'playing'")
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    /// Deletes an instance's row. The caller removes its folder; nothing else refers to the row.
    pub async fn delete_instance(&self, id: &str) -> Result<(), String> {
        sqlx::query("DELETE FROM instances WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    /// The icon recents, most recently used first.
    pub async fn recent_icons(&self) -> Result<Vec<InstanceIcon>, String> {
        let rows = sqlx::query("SELECT background, symbol FROM icon_recents ORDER BY used_at DESC, id DESC LIMIT ?")
            .bind(RECENT_ICON_LIMIT)
            .fetch_all(&self.pool)
            .await
            .map_err(|error| error.to_string())?;
        Ok(rows
            .into_iter()
            .map(|row| InstanceIcon::Composed {
                background: row.get("background"),
                symbol: row.get("symbol"),
            })
            .collect())
    }

    /// Records a composed icon as the most recent recent. A repeat moves to
    /// the front, and the list is capped at [`RECENT_ICON_LIMIT`].
    pub async fn record_recent_icon(&self, background: &str, symbol: &str) -> Result<(), String> {
        let mut transaction = self.pool.begin().await.map_err(|error| error.to_string())?;
        sqlx::query("DELETE FROM icon_recents WHERE background = ? AND symbol = ?")
            .bind(background)
            .bind(symbol)
            .execute(&mut *transaction)
            .await
            .map_err(|error| error.to_string())?;
        sqlx::query("INSERT INTO icon_recents (background, symbol, used_at) VALUES (?, ?, ?)")
            .bind(background)
            .bind(symbol)
            .bind(Utc::now().timestamp())
            .execute(&mut *transaction)
            .await
            .map_err(|error| error.to_string())?;
        sqlx::query("DELETE FROM icon_recents WHERE id NOT IN (SELECT id FROM icon_recents ORDER BY used_at DESC, id DESC LIMIT ?)")
            .bind(RECENT_ICON_LIMIT)
            .execute(&mut *transaction)
            .await
            .map_err(|error| error.to_string())?;
        transaction.commit().await.map_err(|error| error.to_string())
    }
}

#[cfg(test)]
#[path = "../../tests/app/database.rs"]
mod tests;
