use std::path::{Path, PathBuf};

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};

use crate::instance::{CreateInstanceParams, InstanceConfig, LoaderType};
use crate::utils::path_util::PathUtil;

/// Manages Minecraft instances, accounts, and recent imports backed by SQLite.
///
/// Each instance gets its own subdirectory under `instances_dir`.
/// Metadata is stored in `{data_dir}/app.db`.
pub struct InstanceManager {
    pool: SqlitePool,
    instances_dir: PathBuf,
}

impl InstanceManager {
    /// Open (or create) the application database at `{data_dir}/app.db`
    /// and use `instances_dir` as the root for instance directories.
    ///
    /// Automatically migrates from the old `instances.db` filename if present.
    pub async fn new(data_dir: &Path, instances_dir: PathBuf) -> anyhow::Result<Self> {
        std::fs::create_dir_all(data_dir)?;
        std::fs::create_dir_all(&instances_dir)?;

        // Migrate old database filename
        let old_path = data_dir.join("instances.db");
        let db_path = data_dir.join("app.db");
        if old_path.exists() && !db_path.exists() {
            std::fs::rename(&old_path, &db_path)?;
        }

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

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS installed_mods (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                instance_id     INTEGER NOT NULL,
                file_name       TEXT    NOT NULL,
                modrinth_id     TEXT,
                curseforge_id   TEXT,
                version_id      TEXT,
                project_name    TEXT,
                icon_url        TEXT,
                installed_at    TEXT    NOT NULL,
                FOREIGN KEY (instance_id) REFERENCES instances(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Migration: add icon_url column if it doesn't exist (for existing DBs)
        let _ = sqlx::query("ALTER TABLE installed_mods ADD COLUMN icon_url TEXT")
            .execute(&self.pool)
            .await;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS recent_imports (
                id           INTEGER PRIMARY KEY AUTOINCREMENT,
                file_hash    TEXT    NOT NULL UNIQUE,
                name         TEXT    NOT NULL,
                source       TEXT    NOT NULL,
                size_bytes   INTEGER NOT NULL,
                imported_at  TEXT    NOT NULL,
                file_name    TEXT    NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS accounts (
                id             INTEGER PRIMARY KEY AUTOINCREMENT,
                mode           TEXT    NOT NULL,
                uuid           TEXT    NOT NULL UNIQUE,
                username       TEXT    NOT NULL,
                refresh_token  TEXT,
                skin_url       TEXT,
                cape_url       TEXT,
                is_active      INTEGER NOT NULL DEFAULT 0,
                created_at     TEXT    NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Instances
    // -----------------------------------------------------------------------

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

    /// Delete an instance by ID. Also removes the instance directory from disk
    /// and all associated installed mod records.
    pub async fn delete(&self, id: i64) -> anyhow::Result<()> {
        let instance = self.get(id).await?;
        if let Some(inst) = instance {
            let path = PathBuf::from(&inst.instance_path);
            if path.exists() {
                std::fs::remove_dir_all(&path)?;
            }
        }
        // Delete all installed mod records for this instance
        sqlx::query("DELETE FROM installed_mods WHERE instance_id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
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

    /// Update an instance's version fields.
    pub async fn update(
        &self,
        id: i64,
        minecraft_version: &str,
        loader: &LoaderType,
        loader_version: Option<&str>,
        java_version: Option<&str>,
    ) -> anyhow::Result<()> {
        sqlx::query(
            "UPDATE instances SET minecraft_version = ?, loader = ?, loader_version = ?, java_version = ? WHERE id = ?",
        )
        .bind(minecraft_version)
        .bind(loader.as_str())
        .bind(loader_version)
        .bind(java_version)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Installed mod tracking
    // -----------------------------------------------------------------------

    /// Record an installed mod in the database.
    pub async fn add_installed_mod(
        &self,
        instance_id: i64,
        file_name: &str,
        modrinth_id: Option<&str>,
        curseforge_id: Option<&str>,
        version_id: Option<&str>,
        project_name: Option<&str>,
        icon_url: Option<&str>,
    ) -> anyhow::Result<i64> {
        let now = chrono::Utc::now().to_rfc3339();
        let id = sqlx::query(
            r#"INSERT INTO installed_mods (instance_id, file_name, modrinth_id, curseforge_id, version_id, project_name, icon_url, installed_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(instance_id)
        .bind(file_name)
        .bind(modrinth_id)
        .bind(curseforge_id)
        .bind(version_id)
        .bind(project_name)
        .bind(icon_url)
        .bind(&now)
        .execute(&self.pool)
        .await?
        .last_insert_rowid();
        Ok(id)
    }

    /// Remove an installed mod record by file name.
    pub async fn remove_installed_mod(
        &self,
        instance_id: i64,
        file_name: &str,
    ) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM installed_mods WHERE instance_id = ? AND file_name = ?")
            .bind(instance_id)
            .bind(file_name)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Get all installed mods for an instance.
    pub async fn list_installed_mods(
        &self,
        instance_id: i64,
    ) -> anyhow::Result<Vec<InstalledModRecord>> {
        let rows = sqlx::query(
            "SELECT id, instance_id, file_name, modrinth_id, curseforge_id, version_id, project_name, icon_url, installed_at FROM installed_mods WHERE instance_id = ? ORDER BY installed_at DESC",
        )
        .bind(instance_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .iter()
            .map(|r| InstalledModRecord {
                id: r.get("id"),
                instance_id: r.get("instance_id"),
                file_name: r.get("file_name"),
                modrinth_id: r.get("modrinth_id"),
                curseforge_id: r.get("curseforge_id"),
                version_id: r.get("version_id"),
                project_name: r.get("project_name"),
                icon_url: r.get("icon_url"),
                installed_at: r.get("installed_at"),
            })
            .collect())
    }

    /// Find an installed mod record by project ID (modrinth or curseforge).
    pub async fn find_installed_mod_by_project(
        &self,
        instance_id: i64,
        modrinth_id: Option<&str>,
        curseforge_id: Option<&str>,
    ) -> anyhow::Result<Option<InstalledModRecord>> {
        let row = if let Some(mid) = modrinth_id {
            sqlx::query(
                "SELECT id, instance_id, file_name, modrinth_id, curseforge_id, version_id, project_name, icon_url, installed_at FROM installed_mods WHERE instance_id = ? AND modrinth_id = ?",
            )
            .bind(instance_id)
            .bind(mid)
            .fetch_optional(&self.pool)
            .await?
        } else if let Some(cid) = curseforge_id {
            sqlx::query(
                "SELECT id, instance_id, file_name, modrinth_id, curseforge_id, version_id, project_name, icon_url, installed_at FROM installed_mods WHERE instance_id = ? AND curseforge_id = ?",
            )
            .bind(instance_id)
            .bind(cid)
            .fetch_optional(&self.pool)
            .await?
        } else {
            None
        };

        Ok(row.map(|r| InstalledModRecord {
            id: r.get("id"),
            instance_id: r.get("instance_id"),
            file_name: r.get("file_name"),
            modrinth_id: r.get("modrinth_id"),
            curseforge_id: r.get("curseforge_id"),
            version_id: r.get("version_id"),
            project_name: r.get("project_name"),
            icon_url: r.get("icon_url"),
            installed_at: r.get("installed_at"),
        }))
    }

    /// Update an installed mod's file name and version (for replacements).
    pub async fn update_installed_mod(
        &self,
        id: i64,
        new_file_name: &str,
        new_version_id: Option<&str>,
    ) -> anyhow::Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE installed_mods SET file_name = ?, version_id = ?, installed_at = ? WHERE id = ?",
        )
        .bind(new_file_name)
        .bind(new_version_id)
        .bind(&now)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Recent imports
    // -----------------------------------------------------------------------

    /// Add or update a recent import entry. Deduplicates by file hash.
    /// After insert, prunes to keep at most 3 entries within the last 90 days.
    pub async fn add_recent_import(
        &self,
        file_hash: &str,
        name: &str,
        source: &str,
        size_bytes: i64,
        file_name: &str,
    ) -> anyhow::Result<i64> {
        let now = chrono::Utc::now().to_rfc3339();
        // Upsert: if hash exists, update timestamp and name
        sqlx::query(
            r#"INSERT INTO recent_imports (file_hash, name, source, size_bytes, imported_at, file_name)
               VALUES (?, ?, ?, ?, ?, ?)
               ON CONFLICT(file_hash) DO UPDATE SET imported_at = excluded.imported_at, name = excluded.name"#,
        )
        .bind(file_hash)
        .bind(name)
        .bind(source)
        .bind(size_bytes)
        .bind(&now)
        .bind(file_name)
        .execute(&self.pool)
        .await?;

        // Get the row ID
        let row = sqlx::query("SELECT id FROM recent_imports WHERE file_hash = ?")
            .bind(file_hash)
            .fetch_one(&self.pool)
            .await?;
        let id: i64 = row.get("id");

        // Prune old entries (>90 days)
        let cutoff = (chrono::Utc::now() - chrono::Duration::days(90)).to_rfc3339();
        sqlx::query("DELETE FROM recent_imports WHERE imported_at < ?")
            .bind(&cutoff)
            .execute(&self.pool)
            .await?;

        // Keep only the most recent 3
        sqlx::query(
            "DELETE FROM recent_imports WHERE id NOT IN (SELECT id FROM recent_imports ORDER BY imported_at DESC LIMIT 3)",
        )
        .execute(&self.pool)
        .await?;

        Ok(id)
    }

    /// List all recent imports (max 3, most recent first).
    pub async fn list_recent_imports(&self) -> anyhow::Result<Vec<RecentImportRecord>> {
        let rows = sqlx::query(
            "SELECT id, file_hash, name, source, size_bytes, imported_at, file_name FROM recent_imports ORDER BY imported_at DESC LIMIT 3",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .iter()
            .map(|r| RecentImportRecord {
                id: r.get("id"),
                file_hash: r.get("file_hash"),
                name: r.get("name"),
                source: r.get("source"),
                size_bytes: r.get("size_bytes"),
                imported_at: r.get("imported_at"),
                file_name: r.get("file_name"),
            })
            .collect())
    }

    /// Get a single recent import by ID.
    pub async fn get_recent_import(&self, id: i64) -> anyhow::Result<Option<RecentImportRecord>> {
        let row = sqlx::query(
            "SELECT id, file_hash, name, source, size_bytes, imported_at, file_name FROM recent_imports WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| RecentImportRecord {
            id: r.get("id"),
            file_hash: r.get("file_hash"),
            name: r.get("name"),
            source: r.get("source"),
            size_bytes: r.get("size_bytes"),
            imported_at: r.get("imported_at"),
            file_name: r.get("file_name"),
        }))
    }

    // -----------------------------------------------------------------------
    // Accounts
    // -----------------------------------------------------------------------

    /// Add or update an account. Upserts by UUID.
    pub async fn add_account(
        &self,
        mode: &str,
        uuid: &str,
        username: &str,
        refresh_token: Option<&str>,
        skin_url: Option<&str>,
        cape_url: Option<&str>,
    ) -> anyhow::Result<i64> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            r#"INSERT INTO accounts (mode, uuid, username, refresh_token, skin_url, cape_url, is_active, created_at)
               VALUES (?, ?, ?, ?, ?, ?, 0, ?)
               ON CONFLICT(uuid) DO UPDATE SET
                   username = excluded.username,
                   refresh_token = excluded.refresh_token,
                   skin_url = excluded.skin_url,
                   cape_url = excluded.cape_url"#,
        )
        .bind(mode)
        .bind(uuid)
        .bind(username)
        .bind(refresh_token)
        .bind(skin_url)
        .bind(cape_url)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        let row = sqlx::query("SELECT id FROM accounts WHERE uuid = ?")
            .bind(uuid)
            .fetch_one(&self.pool)
            .await?;
        Ok(row.get("id"))
    }

    /// Set one account as active (deactivates all others).
    pub async fn set_active_account(&self, id: i64) -> anyhow::Result<()> {
        sqlx::query("UPDATE accounts SET is_active = 0")
            .execute(&self.pool)
            .await?;
        sqlx::query("UPDATE accounts SET is_active = 1 WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Deactivate all accounts (no active session).
    pub async fn deactivate_all_accounts(&self) -> anyhow::Result<()> {
        sqlx::query("UPDATE accounts SET is_active = 0")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Get the currently active account.
    pub async fn get_active_account(&self) -> anyhow::Result<Option<AccountRecord>> {
        let row = sqlx::query(
            "SELECT id, mode, uuid, username, refresh_token, skin_url, cape_url, is_active, created_at FROM accounts WHERE is_active = 1 LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| row_to_account(&r)))
    }

    /// List all accounts.
    pub async fn list_accounts(&self) -> anyhow::Result<Vec<AccountRecord>> {
        let rows = sqlx::query(
            "SELECT id, mode, uuid, username, refresh_token, skin_url, cape_url, is_active, created_at FROM accounts ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.iter().map(|r| row_to_account(r)).collect())
    }

    /// Remove an account by ID.
    pub async fn remove_account(&self, id: i64) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM accounts WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Update an account's refresh token (for Microsoft token refresh).
    pub async fn update_account_token(
        &self,
        id: i64,
        refresh_token: Option<&str>,
    ) -> anyhow::Result<()> {
        sqlx::query("UPDATE accounts SET refresh_token = ? WHERE id = ?")
            .bind(refresh_token)
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

// ---------------------------------------------------------------------------
// Record types
// ---------------------------------------------------------------------------

/// Record of an installed mod stored in the database.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledModRecord {
    pub id: i64,
    pub instance_id: i64,
    pub file_name: String,
    pub modrinth_id: Option<String>,
    pub curseforge_id: Option<String>,
    pub version_id: Option<String>,
    pub project_name: Option<String>,
    pub icon_url: Option<String>,
    pub installed_at: String,
}

/// Record of a recent modpack import.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentImportRecord {
    pub id: i64,
    pub file_hash: String,
    pub name: String,
    pub source: String,
    pub size_bytes: i64,
    pub imported_at: String,
    pub file_name: String,
}

/// Record of a saved account.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountRecord {
    pub id: i64,
    pub mode: String,
    pub uuid: String,
    pub username: String,
    pub refresh_token: Option<String>,
    pub skin_url: Option<String>,
    pub cape_url: Option<String>,
    pub is_active: bool,
    pub created_at: String,
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

fn row_to_account(row: &sqlx::sqlite::SqliteRow) -> AccountRecord {
    let is_active: i32 = row.get("is_active");
    AccountRecord {
        id: row.get("id"),
        mode: row.get("mode"),
        uuid: row.get("uuid"),
        username: row.get("username"),
        refresh_token: row.get("refresh_token"),
        skin_url: row.get("skin_url"),
        cape_url: row.get("cape_url"),
        is_active: is_active != 0,
        created_at: row.get("created_at"),
    }
}
