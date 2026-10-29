//! SQLite persistence for the launcher's data — instances live in a
//! database file next to the credential store, schema versioned through
//! sqlx migrations.

use chrono::{DateTime, Local, Utc};
use sqlx::Row;
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};

use crate::app::pages::library::instance::{Biome, Instance, InstanceState, Loader};

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
    pub async fn instances(&self) -> Result<Vec<Instance>, String> {
        let rows = sqlx::query(
            "SELECT id, name, mc_version, loader, loader_version, playtime_seconds, mod_count, \
             last_played_at, installed_at, pinned, state, biome, art_seed, directory_name \
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
                    directory_name: row.get("directory_name"),
                }
            })
            .collect())
    }
}

#[cfg(test)]
#[path = "../../tests/app/database.rs"]
mod tests;
