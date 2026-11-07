//! The content the Discover page added to an instance, or to a world, or imported as a world, kept in the `added_content`
//! table. It is what the page shows as added after a restart.

use chrono::Utc;
use sqlx::Row;

use super::Database;
use crate::app::shared_library::AddedContent;

impl Database {
    /// Every record of content added, for the Discover page to show as added.
    pub async fn added_content(&self) -> Result<Vec<AddedContent>, String> {
        let rows = sqlx::query(
            "SELECT instance_id, scope, kind, platform, project_id, version_id, name, file_name FROM added_content \
             ORDER BY added_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|error| error.to_string())?;
        Ok(rows
            .iter()
            .map(|row| AddedContent {
                instance: row.get("instance_id"),
                scope: row.get("scope"),
                kind: row.get("kind"),
                platform: row.get("platform"),
                project_id: row.get("project_id"),
                version_id: row.get("version_id"),
                name: row.get("name"),
                file_name: row.get("file_name"),
            })
            .collect())
    }

    /// Records content added, replacing the record the same project had in the same place, such as an older release.
    pub async fn record_added(&self, record: &AddedContent) -> Result<(), String> {
        sqlx::query(
            "INSERT INTO added_content (instance_id, scope, kind, platform, project_id, version_id, name, file_name, added_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?) \
             ON CONFLICT (instance_id, scope, kind, platform, project_id) DO UPDATE SET \
                 version_id = excluded.version_id, name = excluded.name, file_name = excluded.file_name, added_at = excluded.added_at",
        )
        .bind(&record.instance)
        .bind(&record.scope)
        .bind(&record.kind)
        .bind(&record.platform)
        .bind(&record.project_id)
        .bind(&record.version_id)
        .bind(&record.name)
        .bind(&record.file_name)
        .bind(Utc::now().timestamp())
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(|error| error.to_string())
    }

    /// Forgets the content of `kind` that an instance holds under `scope` as `file_name`, once its file is removed. Nothing
    /// changes when no record names that file.
    pub async fn forget_added(&self, instance: &str, scope: &str, kind: &str, file_name: &str) -> Result<(), String> {
        sqlx::query("DELETE FROM added_content WHERE instance_id = ? AND scope = ? AND kind = ? AND file_name = ?")
            .bind(instance)
            .bind(scope)
            .bind(kind)
            .bind(file_name)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(|error| error.to_string())
    }
}

#[cfg(test)]
#[path = "../../../tests/app/database/added_content.rs"]
mod tests;
