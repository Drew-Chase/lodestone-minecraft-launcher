//! The content the Discover page added to an instance, or to a world, or imported as a world, kept in the `added_content`
//! table. It is what the page shows as added after a restart.

use chrono::Utc;
use sqlx::Row;
use sqlx::sqlite::SqliteRow;

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
        Ok(rows.into_iter().map(added_of).collect())
    }

    /// The content one instance added, under any scope, for the Content tab to show where its packs were taken from.
    pub async fn added_content_of(&self, instance: &str) -> Result<Vec<AddedContent>, String> {
        let rows = sqlx::query(
            "SELECT instance_id, scope, kind, platform, project_id, version_id, name, file_name FROM added_content \
             WHERE instance_id = ? ORDER BY added_at",
        )
        .bind(instance)
        .fetch_all(&self.pool)
        .await
        .map_err(|error| error.to_string())?;
        Ok(rows.into_iter().map(added_of).collect())
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

    /// Records that `file_name`, copied into `instance`'s folder for `kind`, was taken from the shared download `download`.
    /// The download's record names the platform and project it came from, and that is what the Content tab shows. A download
    /// with no record names no platform, so nothing is recorded for it.
    pub async fn record_shared_origin(&self, instance: &str, kind: &str, download: &str, file_name: &str) -> Result<(), String> {
        let shared = sqlx::query("SELECT platform, project_id, version_id, name FROM shared_content WHERE kind = ? AND path = ?")
            .bind(kind)
            .bind(download)
            .fetch_optional(&self.pool)
            .await
            .map_err(|error| error.to_string())?;
        let Some(shared) = shared else {
            return Ok(());
        };
        self.record_added(&AddedContent {
            instance: instance.to_string(),
            scope: String::new(),
            kind: kind.to_string(),
            platform: shared.get("platform"),
            project_id: shared.get("project_id"),
            version_id: shared.get("version_id"),
            name: shared.get("name"),
            file_name: file_name.to_string(),
        })
        .await
    }
}

/// One row of `added_content`, as the record it holds.
fn added_of(row: SqliteRow) -> AddedContent {
    AddedContent {
        instance: row.get("instance_id"),
        scope: row.get("scope"),
        kind: row.get("kind"),
        platform: row.get("platform"),
        project_id: row.get("project_id"),
        version_id: row.get("version_id"),
        name: row.get("name"),
        file_name: row.get("file_name"),
    }
}

#[cfg(test)]
#[path = "../../../tests/app/database/added_content.rs"]
mod tests;
