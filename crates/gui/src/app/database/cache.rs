//! The cache section of the database: tables named `cache_` hold what the launcher can work out again, and Clear cache
//! empties them. Today that is the mod index, what each mod jar declares (see `pages/instance/files/mod_index.rs`).

use lodestone_loaders::ModDependency;
use sqlx::Row;
use tracing::warn;

use super::Database;

/// One jar's row in the mod index. The jar is known by its folder, `stem`, `size` and `modified`.
#[derive(Debug, Clone)]
pub struct ModIndexRow {
    /// The jar's name without `.jar` or `.jar.disabled`.
    pub stem: String,
    pub size: u64,
    /// The jar's modification time, in nanoseconds since the Unix epoch.
    pub modified: u64,
    pub name: Option<String>,
    pub author: Option<String>,
    pub version: Option<String>,
    pub id: Option<String>,
    pub dependencies: Vec<ModDependency>,
    /// The jar's icon as a PNG, scaled to the Mods tab's tile.
    pub icon: Option<Vec<u8>>,
}

/// How a jar is named in the mod index, for removing its row.
#[derive(Debug, Clone)]
pub struct ModIndexKey {
    pub stem: String,
    pub size: u64,
    pub modified: u64,
}

impl Database {
    /// The mod index's rows for the mods folder `folder`. A row whose dependencies no longer read is left out, so its jar
    /// is read again.
    pub async fn mod_index(&self, folder: &str) -> Result<Vec<ModIndexRow>, String> {
        let rows =
            sqlx::query("SELECT stem, size, modified, name, author, version, mod_id, dependencies, icon FROM cache_mod_index WHERE folder = ?")
                .bind(folder)
                .fetch_all(&self.pool)
                .await
                .map_err(|error| error.to_string())?;
        Ok(rows
            .into_iter()
            .filter_map(|row| {
                let stem: String = row.get("stem");
                let dependencies: String = row.get("dependencies");
                let dependencies = match serde_json::from_str(&dependencies) {
                    Ok(dependencies) => dependencies,
                    Err(error) => {
                        warn!("The mod index row of {stem} does not read, so the jar is read again: {error}");
                        return None;
                    }
                };
                Some(ModIndexRow {
                    stem,
                    size: row.get::<i64, _>("size") as u64,
                    modified: row.get::<i64, _>("modified") as u64,
                    name: row.get("name"),
                    author: row.get("author"),
                    version: row.get("version"),
                    id: row.get("mod_id"),
                    dependencies,
                    icon: row.get("icon"),
                })
            })
            .collect())
    }

    /// Adds the rows of the jars just read to `folder`'s mod index, and removes the rows of jars that are gone, in one
    /// transaction. A row already there is replaced.
    pub async fn update_mod_index(&self, folder: &str, added: &[ModIndexRow], removed: &[ModIndexKey]) -> Result<(), String> {
        if added.is_empty() && removed.is_empty() {
            return Ok(());
        }
        let mut transaction = self.pool.begin().await.map_err(|error| error.to_string())?;
        for key in removed {
            sqlx::query("DELETE FROM cache_mod_index WHERE folder = ? AND stem = ? AND size = ? AND modified = ?")
                .bind(folder)
                .bind(&key.stem)
                .bind(key.size as i64)
                .bind(key.modified as i64)
                .execute(&mut *transaction)
                .await
                .map_err(|error| error.to_string())?;
        }
        for row in added {
            let dependencies = serde_json::to_string(&row.dependencies).map_err(|error| error.to_string())?;
            sqlx::query(
                "INSERT OR REPLACE INTO cache_mod_index \
                 (folder, stem, size, modified, name, author, version, mod_id, dependencies, icon) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(folder)
            .bind(&row.stem)
            .bind(row.size as i64)
            .bind(row.modified as i64)
            .bind(&row.name)
            .bind(&row.author)
            .bind(&row.version)
            .bind(&row.id)
            .bind(dependencies)
            .bind(row.icon.as_deref())
            .execute(&mut *transaction)
            .await
            .map_err(|error| error.to_string())?;
        }
        transaction.commit().await.map_err(|error| error.to_string())
    }

    /// Removes every row of the mods folder `folder`, as when its instance is deleted.
    pub async fn forget_mod_index(&self, folder: &str) -> Result<(), String> {
        sqlx::query("DELETE FROM cache_mod_index WHERE folder = ?")
            .bind(folder)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(|error| error.to_string())
    }
}
