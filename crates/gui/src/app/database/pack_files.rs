//! The files an installed modpack release shipped, kept per instance. The install writes them with the instance's row, and an
//! update replaces them when the release changes.

use sqlx::{Row, Sqlite, Transaction};

use super::Database;
use crate::app::shared_library::PackFile;

impl Database {
    /// The files the instance's installed release shipped, in path order. An instance with none has no release recorded: it
    /// was made before these were kept, or it was not installed from a platform.
    pub async fn pack_files_of(&self, instance: &str) -> Result<Vec<PackFile>, String> {
        let rows = sqlx::query("SELECT instance_id, path, sha1 FROM pack_files WHERE instance_id = ? ORDER BY path")
            .bind(instance)
            .fetch_all(&self.pool)
            .await
            .map_err(|error| error.to_string())?;
        Ok(rows
            .into_iter()
            .map(|row| PackFile {
                instance: row.get("instance_id"),
                path: row.get("path"),
                sha1: row.get("sha1"),
            })
            .collect())
    }
}

/// Inserts each of `files` for `instance` within `transaction`. A row already held for the same path is replaced.
pub(super) async fn write_pack_files(transaction: &mut Transaction<'_, Sqlite>, instance: &str, files: &[PackFile]) -> Result<(), String> {
    for file in files {
        sqlx::query("INSERT OR REPLACE INTO pack_files (instance_id, path, sha1) VALUES (?, ?, ?)")
            .bind(instance)
            .bind(&file.path)
            .bind(&file.sha1)
            .execute(&mut **transaction)
            .await
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "../../../tests/app/database/pack_files.rs"]
mod tests;
