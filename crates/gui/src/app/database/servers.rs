//! The servers the player saved, kept in the `servers` table. An address is kept once, and the favorites come first.

use chrono::{DateTime, Local, Utc};
use sqlx::Row;
use sqlx::sqlite::SqliteRow;

use super::Database;

/// A saved server, as the Servers page and the Join Server dialog list it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedServer {
    pub id: i64,
    pub name: String,
    /// The address as it was typed and saved, such as `mc.hypixel.net` or `192.168.1.42:25565`.
    pub address: String,
    pub favorite: bool,
    pub added_at: DateTime<Local>,
}

impl Database {
    /// Every saved server: the favorites first, then by name.
    pub async fn saved_servers(&self) -> Result<Vec<SavedServer>, String> {
        let rows = sqlx::query(
            "SELECT id, name, address, favorite, added_at FROM servers \
             ORDER BY favorite DESC, name COLLATE NOCASE, id",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|error| error.to_string())?;
        rows.iter().map(server_from_row).collect()
    }

    /// Saves a server. An address that is saved already is an error.
    pub async fn add_server(&self, name: &str, address: &str, favorite: bool, at: DateTime<Local>) -> Result<(), String> {
        sqlx::query("INSERT INTO servers (name, address, favorite, added_at) VALUES (?, ?, ?, ?)")
            .bind(name)
            .bind(address)
            .bind(favorite)
            .bind(at.timestamp())
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    /// Changes a saved server's name, address and favorite mark.
    pub async fn update_server(&self, id: i64, name: &str, address: &str, favorite: bool) -> Result<(), String> {
        sqlx::query("UPDATE servers SET name = ?, address = ?, favorite = ? WHERE id = ?")
            .bind(name)
            .bind(address)
            .bind(favorite)
            .bind(id)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    /// Removes a saved server.
    pub async fn remove_server(&self, id: i64) -> Result<(), String> {
        sqlx::query("DELETE FROM servers WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(|error| error.to_string())
    }
}

/// A saved server from its row.
fn server_from_row(row: &SqliteRow) -> Result<SavedServer, String> {
    let added_at = DateTime::<Utc>::from_timestamp(row.get::<i64, _>("added_at"), 0).ok_or("a server has a time out of range")?;
    Ok(SavedServer {
        id: row.get("id"),
        name: row.get("name"),
        address: row.get("address"),
        favorite: row.get::<bool, _>("favorite"),
        added_at: added_at.with_timezone(&Local),
    })
}

#[cfg(test)]
#[path = "../../../tests/app/database/servers.rs"]
mod tests;
