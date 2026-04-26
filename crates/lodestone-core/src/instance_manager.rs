use std::path::{Path, PathBuf};

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};

use crate::instance::{CreateInstanceParams, InstanceConfig, LoaderType};
use crate::utils::path_util::PathUtil;

/// Manages Minecraft instances backed by a SQLite database.
///
/// Each instance gets its own subdirectory under `instances_dir`.
/// Metadata is stored in `{data_dir}/instances.db`.
pub struct InstanceManager {
    pool: SqlitePool,
    instances_dir: PathBuf,
}

impl InstanceManager {
    /// Open (or create) the instance database at `{data_dir}/instances.db`
    /// and use `instances_dir` as the root for instance directories.
    pub async fn new(data_dir: &Path, instances_dir: PathBuf) -> anyhow::Result<Self> {
        std::fs::create_dir_all(data_dir)?;
        std::fs::create_dir_all(&instances_dir)?;

        let db_path = data_dir.join("instances.db");
        let options = SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .connect_with(options)
            .await?;

        let mgr = Self {
            pool,
            instances_dir,
        };
        mgr.migrate().await?;
        Ok(mgr)
    }

    /// Run schema migrations.
    async fn migrate(&self) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS instances (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                name            TEXT    NOT NULL,
                minecraft_version TEXT  NOT NULL,
                loader          TEXT    NOT NULL DEFAULT 'vanilla',
                loader_version  TEXT,
                java_version    TEXT,
                created_at      TEXT    NOT NULL,
                last_played     TEXT,
                instance_path   TEXT    NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Create a new instance. Returns the full config with generated ID and path.
    pub async fn create(&self, params: CreateInstanceParams) -> anyhow::Result<InstanceConfig> {
        let mut dir_path = self.instances_dir.join(&params.name);
        dir_path.clean()?;
        dir_path.unique();
        std::fs::create_dir_all(&dir_path)?;

        let created_at = chrono::Utc::now().to_rfc3339();
        let loader_str = params.loader.as_str();
        let path_str = dir_path.to_string_lossy().to_string();

        let id = sqlx::query(
            r#"
            INSERT INTO instances (name, minecraft_version, loader, loader_version, java_version, created_at, instance_path)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&params.name)
        .bind(&params.minecraft_version)
        .bind(loader_str)
        .bind(&params.loader_version)
        .bind(&params.java_version)
        .bind(&created_at)
        .bind(&path_str)
        .execute(&self.pool)
        .await?
        .last_insert_rowid();

        Ok(InstanceConfig {
            id,
            name: params.name,
            minecraft_version: params.minecraft_version,
            loader: params.loader,
            loader_version: params.loader_version,
            java_version: params.java_version,
            created_at,
            last_played: None,
            instance_path: path_str,
        })
    }

    /// List all instances.
    pub async fn list(&self) -> anyhow::Result<Vec<InstanceConfig>> {
        let rows = sqlx::query(
            "SELECT id, name, minecraft_version, loader, loader_version, java_version, created_at, last_played, instance_path FROM instances ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut instances = Vec::with_capacity(rows.len());
        for row in rows {
            instances.push(row_to_config(&row));
        }
        Ok(instances)
    }

    /// Get a single instance by ID.
    pub async fn get(&self, id: i64) -> anyhow::Result<Option<InstanceConfig>> {
        let row = sqlx::query(
            "SELECT id, name, minecraft_version, loader, loader_version, java_version, created_at, last_played, instance_path FROM instances WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| row_to_config(&r)))
    }

    /// Delete an instance by ID. Also removes the instance directory from disk.
    pub async fn delete(&self, id: i64) -> anyhow::Result<()> {
        let instance = self.get(id).await?;
        if let Some(inst) = instance {
            let path = PathBuf::from(&inst.instance_path);
            if path.exists() {
                std::fs::remove_dir_all(&path)?;
            }
        }
        sqlx::query("DELETE FROM instances WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Update the last_played timestamp for an instance.
    pub async fn touch(&self, id: i64) -> anyhow::Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query("UPDATE instances SET last_played = ? WHERE id = ?")
            .bind(&now)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Return the current instances directory path.
    pub fn instances_dir(&self) -> &Path {
        &self.instances_dir
    }
}

fn row_to_config(row: &sqlx::sqlite::SqliteRow) -> InstanceConfig {
    let loader_str: String = row.get("loader");
    InstanceConfig {
        id: row.get("id"),
        name: row.get("name"),
        minecraft_version: row.get("minecraft_version"),
        loader: LoaderType::parse(&loader_str).unwrap_or(LoaderType::Vanilla),
        loader_version: row.get("loader_version"),
        java_version: row.get("java_version"),
        created_at: row.get("created_at"),
        last_played: row.get("last_played"),
        instance_path: row.get("instance_path"),
    }
}
