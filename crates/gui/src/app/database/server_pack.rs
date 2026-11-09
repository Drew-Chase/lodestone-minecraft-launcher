//! The mod choices the server pack wizard remembers, kept in the `server_pack_sides` table. A choice is keyed by the
//! mod's manifest id, so it holds for the same mod in every instance and across restarts.

use chrono::Utc;
use sqlx::Row;

use super::Database;

impl Database {
    /// Every remembered choice, as (mod id, `include` or `exclude`).
    pub async fn server_pack_choices(&self) -> Result<Vec<(String, String)>, String> {
        let rows = sqlx::query("SELECT mod_id, choice FROM server_pack_sides")
            .fetch_all(&self.pool)
            .await
            .map_err(|error| error.to_string())?;
        Ok(rows.into_iter().map(|row| (row.get("mod_id"), row.get("choice"))).collect())
    }

    /// Remembers `choice` (`include` or `exclude`) for the mod `mod_id`, or forgets its choice when `choice` is `None`.
    pub async fn set_server_pack_choice(&self, mod_id: &str, choice: Option<&str>) -> Result<(), String> {
        let result = match choice {
            Some(choice) => {
                sqlx::query(
                    "INSERT INTO server_pack_sides (mod_id, choice, updated_at) VALUES (?, ?, ?) \
                     ON CONFLICT(mod_id) DO UPDATE SET choice = excluded.choice, updated_at = excluded.updated_at",
                )
                .bind(mod_id)
                .bind(choice)
                .bind(Utc::now().timestamp())
                .execute(&self.pool)
                .await
            }
            None => {
                sqlx::query("DELETE FROM server_pack_sides WHERE mod_id = ?")
                    .bind(mod_id)
                    .execute(&self.pool)
                    .await
            }
        };
        result.map(|_| ()).map_err(|error| error.to_string())
    }
}

#[cfg(test)]
#[path = "../../../tests/app/database/server_pack.rs"]
mod tests;
