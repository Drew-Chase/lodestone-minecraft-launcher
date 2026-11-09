//! The database side of an in-place pack update: the one transaction that moves an instance onto the release it now runs, and the
//! record of which update committed, which the launch reads to decide whether an update that stopped can be finished or must be
//! rolled back.

use std::collections::BTreeMap;

use chrono::Utc;
use sqlx::Row;

use super::pack_files::write_pack_files;
use super::{Database, delete_jar, write_mod_sources};
use crate::app::pages::library::instance::Instance;
use crate::app::shared_library::{AddedContent, ModSource, PackFile};

impl Database {
    /// Commits an in-place update in one transaction. The instance's row moves to the release it now runs, its pack's file list
    /// is replaced, the jars the update removed lose their records, and the release's mods and packs are recorded. `token` names
    /// the update, and it is recorded in the same transaction, so a launch can tell that this update committed.
    pub async fn commit_pack_update(
        &self,
        instance: &Instance,
        files: &[PackFile],
        removed_jars: &[String],
        sources: &[ModSource],
        packs: &[AddedContent],
        token: &str,
    ) -> Result<(), String> {
        let mut transaction = self.pool.begin().await.map_err(|error| error.to_string())?;
        sqlx::query(
            "UPDATE instances SET mc_version = ?, loader = ?, loader_version = ?, version_id = ?, java_component = ?, \
             source_version = ?, source_version_id = ?, mod_count = ? WHERE id = ?",
        )
        .bind(&instance.mc_version)
        .bind(instance.loader.to_db())
        .bind(&instance.loader_version)
        .bind(&instance.version_id)
        .bind(&instance.java_component)
        .bind(&instance.source_version)
        .bind(&instance.source_version_id)
        .bind(instance.mod_count as i64)
        .bind(&instance.id)
        .execute(&mut *transaction)
        .await
        .map_err(|error| error.to_string())?;
        sqlx::query("DELETE FROM pack_files WHERE instance_id = ?")
            .bind(&instance.id)
            .execute(&mut *transaction)
            .await
            .map_err(|error| error.to_string())?;
        write_pack_files(&mut transaction, &instance.id, files).await?;
        for file_name in removed_jars {
            delete_jar(&mut transaction, &instance.id, file_name).await?;
        }
        write_mod_sources(&mut transaction, sources).await?;
        for record in packs {
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
            .execute(&mut *transaction)
            .await
            .map_err(|error| error.to_string())?;
        }
        sqlx::query("INSERT OR REPLACE INTO pack_commits (instance_id, token) VALUES (?, ?)")
            .bind(&instance.id)
            .bind(token)
            .execute(&mut *transaction)
            .await
            .map_err(|error| error.to_string())?;
        transaction.commit().await.map_err(|error| error.to_string())
    }

    /// The token of the last in-place update each instance committed, by instance id.
    pub async fn committed_pack_updates(&self) -> Result<BTreeMap<String, String>, String> {
        let rows = sqlx::query("SELECT instance_id, token FROM pack_commits")
            .fetch_all(&self.pool)
            .await
            .map_err(|error| error.to_string())?;
        Ok(rows.into_iter().map(|row| (row.get("instance_id"), row.get("token"))).collect())
    }
}

#[cfg(test)]
#[path = "../../../tests/app/database/pack_update.rs"]
mod tests;
