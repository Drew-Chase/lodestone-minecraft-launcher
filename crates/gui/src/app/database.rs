//! SQLite persistence for the launcher's data — instances live in a
//! database file next to the credential store, schema versioned through
//! sqlx migrations.

mod added_content;
mod notifications;
pub mod servers;

use std::path::{Path, PathBuf};

use chrono::{DateTime, Local, Utc};
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions, SqliteRow};
use sqlx::{Row, Sqlite, Transaction};

use crate::app::import::record::{ImportFormat, ImportOrigin, ImportRecord};
use crate::app::launch_settings::InstanceSettings;
use crate::app::pages::library::instance::{Biome, Instance, InstanceIcon, InstanceState, Loader};
use crate::app::shared_library::{JarHash, ModSource, SharedRecord};

/// How many icon recents are kept. The icon editor shows the newest few.
const RECENT_ICON_LIMIT: i64 = 8;

/// How many imports are kept. The Import dialog lists the newest few of them.
const IMPORTS_KEPT: i64 = 20;

/// The bytes the launcher downloaded today and over the last seven days, counting today.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DownloadTotals {
    pub today: u64,
    pub week: u64,
}

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
        let mut path = crate::app::paths::config_dir().ok_or("no config directory")?;
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
             icon_background, icon_symbol, icon_path, directory_name, java_component, version_id, \
             source_url, source_platform, source_project_id, source_version, settings FROM instances",
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
                    source_url: row.get("source_url"),
                    source_platform: row.get("source_platform"),
                    source_project_id: row.get("source_project_id"),
                    source_version: row.get("source_version"),
                    settings: settings_of(row.get("settings")),
                }
            })
            .collect())
    }

    /// Saves what the instance page edits: the name, the versions, the banner, the icon and the launch
    /// overrides. Paths are stored relative to the instance `folder`, as the install writes them.
    pub async fn save_instance(&self, instance: &Instance, folder: &Path) -> Result<(), String> {
        let icon = match &instance.icon {
            InstanceIcon::Custom(path) => InstanceIcon::Custom(relative_to(path, folder)),
            composed => composed.clone(),
        };
        let (icon_background, icon_symbol, icon_path) = icon.to_db();
        let art_path = instance
            .art_path
            .as_ref()
            .map(|path| relative_to(path, folder).to_string_lossy().into_owned());
        sqlx::query(
            "UPDATE instances SET name = ?, mc_version = ?, loader_version = ?, version_id = ?, biome = ?, art_seed = ?, \
             art_path = ?, icon_background = ?, icon_symbol = ?, icon_path = ?, settings = ? WHERE id = ?",
        )
        .bind(&instance.name)
        .bind(&instance.mc_version)
        .bind(&instance.loader_version)
        .bind(&instance.version_id)
        .bind(instance.biome.to_db())
        .bind(instance.art_seed as i64)
        .bind(art_path)
        .bind(icon_background)
        .bind(icon_symbol)
        .bind(icon_path)
        .bind(settings_json(&instance.settings))
        .bind(&instance.id)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(|error| error.to_string())
    }

    /// Writes a new instance row and the mods it was installed with, in one transaction. The caller has already
    /// created the folder and written its files.
    pub async fn insert_instance(&self, instance: &Instance, sources: &[ModSource]) -> Result<(), String> {
        let (icon_background, icon_symbol, icon_path) = instance.icon.to_db();
        let mut transaction = self.pool.begin().await.map_err(|error| error.to_string())?;
        sqlx::query(
            "INSERT INTO instances \
             (id, name, mc_version, loader, loader_version, playtime_seconds, mod_count, \
              last_played_at, installed_at, pinned, state, biome, art_seed, art_path, \
              icon_background, icon_symbol, icon_path, directory_name, java_component, version_id, \
              source_url, source_platform, source_project_id, source_version, settings) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
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
        .bind(&instance.source_url)
        .bind(&instance.source_platform)
        .bind(&instance.source_project_id)
        .bind(&instance.source_version)
        .bind(settings_json(&instance.settings))
        .execute(&mut *transaction)
        .await
        .map_err(|error| error.to_string())?;
        write_mod_sources(&mut transaction, sources).await?;
        transaction.commit().await.map_err(|error| error.to_string())
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

    /// Ends a run: the instance's playtime grows by `seconds`, and it was last played when the run
    /// started. The state is left alone, because the instance may have other runs still going.
    pub async fn finish_session(&self, id: &str, started: DateTime<Local>, seconds: u32) -> Result<(), String> {
        sqlx::query("UPDATE instances SET playtime_seconds = playtime_seconds + ?, last_played_at = ? WHERE id = ?")
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

    /// Deletes an instance's row, and the mod records, cached hashes, world pins and added content it holds. The caller
    /// removes its folder; nothing else refers to the row.
    pub async fn delete_instance(&self, id: &str) -> Result<(), String> {
        sqlx::query("DELETE FROM mod_sources WHERE instance_id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|error| error.to_string())?;
        sqlx::query("DELETE FROM added_content WHERE instance_id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|error| error.to_string())?;
        sqlx::query("DELETE FROM jar_hashes WHERE instance_id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|error| error.to_string())?;
        sqlx::query("DELETE FROM world_pins WHERE instance_id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|error| error.to_string())?;
        sqlx::query("DELETE FROM instances WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    /// Every pinned world, as (instance id, save folder).
    pub async fn world_pins(&self) -> Result<Vec<(String, String)>, String> {
        let rows = sqlx::query("SELECT instance_id, folder FROM world_pins")
            .fetch_all(&self.pool)
            .await
            .map_err(|error| error.to_string())?;
        Ok(rows.into_iter().map(|row| (row.get("instance_id"), row.get("folder"))).collect())
    }

    /// Pins or unpins the save `folder` of the instance `instance_id`.
    pub async fn set_world_pin(&self, instance_id: &str, folder: &str, pinned: bool) -> Result<(), String> {
        let statement = if pinned {
            "INSERT OR IGNORE INTO world_pins (instance_id, folder) VALUES (?, ?)"
        } else {
            "DELETE FROM world_pins WHERE instance_id = ? AND folder = ?"
        };
        sqlx::query(statement)
            .bind(instance_id)
            .bind(folder)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    /// Records the modpack release an instance is at, as the update check finds it when the instance has none recorded yet.
    pub async fn set_source_version(&self, id: &str, version: &str) -> Result<(), String> {
        sqlx::query("UPDATE instances SET source_version = ? WHERE id = ?")
            .bind(version)
            .bind(id)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    /// The imports queued at or after `since`, the newest first, at most `limit` of them.
    pub async fn recent_imports(&self, since: DateTime<Local>, limit: i64) -> Result<Vec<ImportRecord>, String> {
        let rows = sqlx::query(
            "SELECT name, format, origin, source, bytes, imported_at FROM imports \
             WHERE imported_at >= ? ORDER BY imported_at DESC, id DESC LIMIT ?",
        )
        .bind(since.timestamp())
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|error| error.to_string())?;
        rows.iter().map(import_from_row).collect()
    }

    /// Writes an import as it is queued, then keeps only the newest `IMPORTS_KEPT` imports.
    pub async fn record_import(&self, import: &ImportRecord) -> Result<(), String> {
        let mut transaction = self.pool.begin().await.map_err(|error| error.to_string())?;
        sqlx::query("INSERT INTO imports (name, format, origin, source, bytes, imported_at) VALUES (?, ?, ?, ?, ?, ?)")
            .bind(&import.name)
            .bind(import.format.to_db())
            .bind(import.origin.to_db())
            .bind(&import.source)
            .bind(import.bytes as i64)
            .bind(import.imported_at.timestamp())
            .execute(&mut *transaction)
            .await
            .map_err(|error| error.to_string())?;
        sqlx::query("DELETE FROM imports WHERE id NOT IN (SELECT id FROM imports ORDER BY id DESC LIMIT ?)")
            .bind(IMPORTS_KEPT)
            .execute(&mut *transaction)
            .await
            .map_err(|error| error.to_string())?;
        transaction.commit().await.map_err(|error| error.to_string())
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

    /// Adds `bytes` to the download total of `day`, a local date as `YYYY-MM-DD`.
    pub async fn add_download_bytes(&self, day: &str, bytes: u64) -> Result<(), String> {
        sqlx::query(
            "INSERT INTO download_stats (day, bytes) VALUES (?, ?) \
             ON CONFLICT(day) DO UPDATE SET bytes = bytes + excluded.bytes",
        )
        .bind(day)
        .bind(bytes as i64)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(|error| error.to_string())
    }

    /// The downloads of `today` and of the days from `week_start` on. Both are local dates as
    /// `YYYY-MM-DD`, which compare correctly as text.
    pub async fn download_totals(&self, today: &str, week_start: &str) -> Result<DownloadTotals, String> {
        let row = sqlx::query(
            "SELECT COALESCE(SUM(CASE WHEN day = ? THEN bytes END), 0) AS today, \
             COALESCE(SUM(bytes), 0) AS week \
             FROM download_stats WHERE day >= ?",
        )
        .bind(today)
        .bind(week_start)
        .fetch_one(&self.pool)
        .await
        .map_err(|error| error.to_string())?;
        Ok(DownloadTotals {
            today: row.get::<i64, _>("today") as u64,
            week: row.get::<i64, _>("week") as u64,
        })
    }

    /// Every file the Discover page has downloaded into the shared folders.
    pub async fn shared_content(&self) -> Result<Vec<SharedRecord>, String> {
        let rows = sqlx::query("SELECT kind, platform, project_id, version_id, name, path, size FROM shared_content ORDER BY added_at DESC, id DESC")
            .fetch_all(&self.pool)
            .await
            .map_err(|error| error.to_string())?;
        Ok(rows
            .into_iter()
            .map(|row| SharedRecord {
                kind: row.get("kind"),
                platform: row.get("platform"),
                project_id: row.get("project_id"),
                version_id: row.get("version_id"),
                name: row.get("name"),
                path: row.get("path"),
                size: row.get::<Option<i64>, _>("size").map(|size| size as u64),
            })
            .collect())
    }

    /// The mods the launcher knows came from a platform, across every instance, most recent first.
    pub async fn mod_sources(&self) -> Result<Vec<ModSource>, String> {
        let rows = sqlx::query("SELECT instance_id, platform, project_id, version_id, name, file_name, sha1 FROM mod_sources ORDER BY added_at DESC")
            .fetch_all(&self.pool)
            .await
            .map_err(|error| error.to_string())?;
        Ok(rows.into_iter().map(mod_source_of).collect())
    }

    /// The mods of one instance that came from a platform.
    pub async fn mod_sources_of(&self, instance: &str) -> Result<Vec<ModSource>, String> {
        let rows = sqlx::query("SELECT instance_id, platform, project_id, version_id, name, file_name, sha1 FROM mod_sources WHERE instance_id = ?")
            .bind(instance)
            .fetch_all(&self.pool)
            .await
            .map_err(|error| error.to_string())?;
        Ok(rows.into_iter().map(mod_source_of).collect())
    }

    /// Records mods installed from a platform. A jar already recorded under the same file name is replaced.
    pub async fn record_mod_sources(&self, records: &[ModSource]) -> Result<(), String> {
        let mut transaction = self.pool.begin().await.map_err(|error| error.to_string())?;
        write_mod_sources(&mut transaction, records).await?;
        transaction.commit().await.map_err(|error| error.to_string())
    }

    /// Replaces the record of the jar `old_file` with `record`, the jar that took its place. The cached hash of the
    /// old jar goes with it, and both changes are one transaction.
    pub async fn replace_mod_source(&self, old_file: &str, record: &ModSource) -> Result<(), String> {
        let mut transaction = self.pool.begin().await.map_err(|error| error.to_string())?;
        delete_jar(&mut transaction, &record.instance, old_file).await?;
        write_mod_sources(&mut transaction, std::slice::from_ref(record)).await?;
        transaction.commit().await.map_err(|error| error.to_string())
    }

    /// Renames the record and the cached hash of the jar `from` to `to`, in one transaction. A record still held under
    /// `to` belongs to a jar that is no longer in the folder, so it is dropped first.
    pub async fn rename_mod(&self, instance: &str, from: &str, to: &str) -> Result<(), String> {
        let mut transaction = self.pool.begin().await.map_err(|error| error.to_string())?;
        delete_jar(&mut transaction, instance, to).await?;
        sqlx::query("UPDATE mod_sources SET file_name = ? WHERE instance_id = ? AND file_name = ?")
            .bind(to)
            .bind(instance)
            .bind(from)
            .execute(&mut *transaction)
            .await
            .map_err(|error| error.to_string())?;
        sqlx::query("UPDATE jar_hashes SET file_name = ? WHERE instance_id = ? AND file_name = ?")
            .bind(to)
            .bind(instance)
            .bind(from)
            .execute(&mut *transaction)
            .await
            .map_err(|error| error.to_string())?;
        transaction.commit().await.map_err(|error| error.to_string())
    }

    /// Forgets the record and the cached hash of a jar that is gone.
    pub async fn forget_mod_source(&self, instance: &str, file_name: &str) -> Result<(), String> {
        let mut transaction = self.pool.begin().await.map_err(|error| error.to_string())?;
        delete_jar(&mut transaction, instance, file_name).await?;
        transaction.commit().await.map_err(|error| error.to_string())
    }

    /// The cached hashes of one instance's jars.
    pub async fn jar_hashes(&self, instance: &str) -> Result<Vec<JarHash>, String> {
        let rows = sqlx::query("SELECT file_name, size, modified_at, sha1 FROM jar_hashes WHERE instance_id = ?")
            .bind(instance)
            .fetch_all(&self.pool)
            .await
            .map_err(|error| error.to_string())?;
        Ok(rows
            .into_iter()
            .map(|row| JarHash {
                file_name: row.get("file_name"),
                size: row.get::<i64, _>("size") as u64,
                modified_at: row.get("modified_at"),
                sha1: row.get("sha1"),
            })
            .collect())
    }

    /// Caches the hashes of one instance's jars. A hash cached under the same file name is replaced.
    pub async fn record_jar_hashes(&self, instance: &str, hashes: &[JarHash]) -> Result<(), String> {
        let mut transaction = self.pool.begin().await.map_err(|error| error.to_string())?;
        for hash in hashes {
            sqlx::query("INSERT OR REPLACE INTO jar_hashes (instance_id, file_name, size, modified_at, sha1) VALUES (?, ?, ?, ?, ?)")
                .bind(instance)
                .bind(&hash.file_name)
                .bind(hash.size as i64)
                .bind(hash.modified_at)
                .bind(&hash.sha1)
                .execute(&mut *transaction)
                .await
                .map_err(|error| error.to_string())?;
        }
        transaction.commit().await.map_err(|error| error.to_string())
    }

    /// Records a file downloaded into the shared folders. Downloading the same version again replaces its row.
    pub async fn record_shared_content(&self, record: &SharedRecord) -> Result<(), String> {
        sqlx::query(
            "INSERT OR REPLACE INTO shared_content (kind, platform, project_id, version_id, name, path, size, added_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&record.kind)
        .bind(&record.platform)
        .bind(&record.project_id)
        .bind(&record.version_id)
        .bind(&record.name)
        .bind(&record.path)
        .bind(record.size.map(|size| size as i64))
        .bind(Utc::now().timestamp())
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(|error| error.to_string())
    }
}

/// The launch overrides a row holds. A missing or unreadable value means none, so the instance inherits every
/// setting and a damaged row never blocks a launch.
fn settings_of(json: Option<String>) -> InstanceSettings {
    let Some(json) = json else {
        return InstanceSettings::default();
    };
    serde_json::from_str(&json).unwrap_or_else(|error| {
        tracing::warn!("instance settings are unreadable, so every setting is inherited: {error}");
        InstanceSettings::default()
    })
}

/// The launch overrides as the row stores them.
fn settings_json(settings: &InstanceSettings) -> String {
    serde_json::to_string(settings).unwrap_or_else(|_| "{}".to_string())
}

/// `path` relative to the instance `folder` when it sits inside it, and `path` as it is otherwise.
fn relative_to(path: &Path, folder: &Path) -> PathBuf {
    path.strip_prefix(folder).map_or_else(|_| path.to_path_buf(), Path::to_path_buf)
}

/// One row of `mod_sources`, read from a query that names its columns.
fn mod_source_of(row: SqliteRow) -> ModSource {
    ModSource {
        instance: row.get("instance_id"),
        platform: row.get("platform"),
        project_id: row.get("project_id"),
        version_id: row.get("version_id"),
        name: row.get("name"),
        file_name: row.get("file_name"),
        sha1: row.get("sha1"),
    }
}

/// Inserts or replaces each record, within `transaction`.
async fn write_mod_sources(transaction: &mut Transaction<'_, Sqlite>, records: &[ModSource]) -> Result<(), String> {
    let added_at = Utc::now().timestamp();
    for record in records {
        sqlx::query(
            "INSERT OR REPLACE INTO mod_sources (instance_id, file_name, platform, project_id, version_id, name, sha1, added_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&record.instance)
        .bind(&record.file_name)
        .bind(&record.platform)
        .bind(&record.project_id)
        .bind(&record.version_id)
        .bind(&record.name)
        .bind(&record.sha1)
        .bind(added_at)
        .execute(&mut **transaction)
        .await
        .map_err(|error| error.to_string())?;
    }
    Ok(())
}

/// Removes the record and the cached hash of the jar `file_name`, within `transaction`.
async fn delete_jar(transaction: &mut Transaction<'_, Sqlite>, instance: &str, file_name: &str) -> Result<(), String> {
    sqlx::query("DELETE FROM mod_sources WHERE instance_id = ? AND file_name = ?")
        .bind(instance)
        .bind(file_name)
        .execute(&mut **transaction)
        .await
        .map_err(|error| error.to_string())?;
    sqlx::query("DELETE FROM jar_hashes WHERE instance_id = ? AND file_name = ?")
        .bind(instance)
        .bind(file_name)
        .execute(&mut **transaction)
        .await
        .map_err(|error| error.to_string())?;
    Ok(())
}

/// Reads one row of `imports`.
fn import_from_row(row: &SqliteRow) -> Result<ImportRecord, String> {
    let format: String = row.get("format");
    let origin: String = row.get("origin");
    let imported_at = DateTime::<Utc>::from_timestamp(row.get::<i64, _>("imported_at"), 0).ok_or("an import has a time out of range")?;
    Ok(ImportRecord {
        name: row.get("name"),
        format: ImportFormat::from_db(&format).ok_or_else(|| format!("an import has an unknown format, {format}"))?,
        origin: ImportOrigin::from_db(&origin).ok_or_else(|| format!("an import has an unknown origin, {origin}"))?,
        source: row.get("source"),
        bytes: row.get::<i64, _>("bytes") as u64,
        imported_at: imported_at.with_timezone(&Local),
    })
}

#[cfg(test)]
#[path = "../../tests/app/database.rs"]
mod tests;
