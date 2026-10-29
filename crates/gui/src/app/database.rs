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

    /// Whether the instances table has no rows (first boot — seed it).
    pub async fn instances_empty(&self) -> Result<bool, String> {
        let row = sqlx::query("SELECT COUNT(*) AS count FROM instances")
            .fetch_one(&self.pool)
            .await
            .map_err(|error| error.to_string())?;
        let count: i64 = row.get("count");
        Ok(count == 0)
    }

    /// Inserts the seed instances and creates their directories.
    pub async fn seed(&self, instances: &[Instance], instances_root: &std::path::Path) -> Result<(), String> {
        std::fs::create_dir_all(instances_root).map_err(|error| format!("creating {}: {error}", instances_root.display()))?;
        let mut transaction = self.pool.begin().await.map_err(|error| error.to_string())?;
        for instance in instances {
            sqlx::query(
                "INSERT INTO instances \
                 (id, name, mc_version, loader, loader_version, playtime_seconds, mod_count, \
                  last_played_at, installed_at, pinned, state, biome, art_seed, directory_name) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
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
            .bind(&instance.directory_name)
            .execute(&mut *transaction)
            .await
            .map_err(|error| error.to_string())?;
            let directory = instances_root.join(&instance.directory_name);
            std::fs::create_dir_all(&directory).map_err(|error| format!("creating {}: {error}", directory.display()))?;
        }
        transaction.commit().await.map_err(|error| error.to_string())
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
