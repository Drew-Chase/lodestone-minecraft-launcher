//! The last full check of each instance's mods, kept across starts, so the Mods tab shows the updates it found without
//! waiting for the platforms. The newer releases are stored as one JSON object per instance.

use std::collections::HashMap;

use lodestone_platforms::ContentVersion;
use sqlx::Row;
use tracing::warn;

use super::Database;

/// One instance's kept check: when it ran, in seconds since the Unix epoch, and the newer release of each jar, by file name.
#[derive(Debug, Clone)]
pub struct StoredModCheck {
    pub instance: String,
    pub checked_at: i64,
    pub updates: HashMap<String, ContentVersion>,
}

impl Database {
    /// Every kept check. A row whose releases no longer read, such as one written by another version, is left out, so its
    /// instance is checked again.
    pub async fn mod_update_checks(&self) -> Result<Vec<StoredModCheck>, String> {
        let rows = sqlx::query("SELECT instance_id, checked_at, updates FROM mod_update_checks")
            .fetch_all(&self.pool)
            .await
            .map_err(|error| error.to_string())?;
        Ok(rows
            .into_iter()
            .filter_map(|row| {
                let instance: String = row.get("instance_id");
                let updates: String = row.get("updates");
                match serde_json::from_str(&updates) {
                    Ok(updates) => Some(StoredModCheck {
                        instance,
                        checked_at: row.get("checked_at"),
                        updates,
                    }),
                    Err(error) => {
                        warn!("the kept mod check of {instance} does not read, so it is checked again: {error}");
                        None
                    }
                }
            })
            .collect())
    }

    /// Keeps `check` as its instance's check, replacing the one kept before.
    pub async fn save_mod_update_check(&self, check: &StoredModCheck) -> Result<(), String> {
        let updates = serde_json::to_string(&check.updates).map_err(|error| error.to_string())?;
        sqlx::query("INSERT OR REPLACE INTO mod_update_checks (instance_id, checked_at, updates) VALUES (?, ?, ?)")
            .bind(&check.instance)
            .bind(check.checked_at)
            .bind(updates)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(|error| error.to_string())
    }
}

impl Database {
    /// Empties the caches the database keeps for mods: the kept update checks and the jars' hashes. Both are worked out
    /// again on the next check. The records of where each mod came from stay.
    pub async fn clear_mod_caches(&self) -> Result<(), String> {
        let mut transaction = self.pool.begin().await.map_err(|error| error.to_string())?;
        for table in ["DELETE FROM mod_update_checks", "DELETE FROM jar_hashes"] {
            sqlx::query(table).execute(&mut *transaction).await.map_err(|error| error.to_string())?;
        }
        transaction.commit().await.map_err(|error| error.to_string())
    }
}
