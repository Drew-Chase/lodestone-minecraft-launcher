//! The notifications the bell lists, kept in the `notifications` table. A key names the event that raised a
//! notification, so an event recorded twice is kept once. The table keeps only its newest rows.

use chrono::{DateTime, Local, Utc};
use sqlx::Row;
use sqlx::sqlite::SqliteRow;

use super::Database;
use crate::app::notifications::{Draft, Kind, Notification};

/// How many notifications are kept. The oldest go first.
const KEPT: i64 = 200;

impl Database {
    /// Records a notification under its key, unless one with that key is kept already, then keeps only the newest rows.
    pub async fn record_notification(&self, draft: &Draft) -> Result<(), String> {
        let mut transaction = self.pool.begin().await.map_err(|error| error.to_string())?;
        sqlx::query(
            "INSERT OR IGNORE INTO notifications (key, kind, subject, detail, instance_id, target, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&draft.key)
        .bind(draft.kind.to_db())
        .bind(&draft.subject)
        .bind(&draft.detail)
        .bind(&draft.instance_id)
        .bind(&draft.target)
        .bind(draft.created_at.timestamp())
        .execute(&mut *transaction)
        .await
        .map_err(|error| error.to_string())?;
        sqlx::query("DELETE FROM notifications WHERE id NOT IN (SELECT id FROM notifications ORDER BY id DESC LIMIT ?)")
            .bind(KEPT)
            .execute(&mut *transaction)
            .await
            .map_err(|error| error.to_string())?;
        transaction.commit().await.map_err(|error| error.to_string())
    }

    /// The kept notifications, newest first.
    pub async fn notifications(&self) -> Result<Vec<Notification>, String> {
        let rows = sqlx::query(
            "SELECT id, kind, subject, detail, instance_id, target, created_at, read_at FROM notifications \
             ORDER BY created_at DESC, id DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|error| error.to_string())?;
        Ok(rows
            .iter()
            .filter_map(|row| match notification_from_row(row) {
                Ok(notification) => Some(notification),
                Err(error) => {
                    tracing::warn!("skipping a notification: {error}");
                    None
                }
            })
            .collect())
    }

    /// Marks every unread notification as read at `at`.
    pub async fn mark_notifications_read(&self, at: DateTime<Local>) -> Result<(), String> {
        sqlx::query("UPDATE notifications SET read_at = ? WHERE read_at IS NULL")
            .bind(at.timestamp())
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    /// Removes one notification.
    pub async fn dismiss_notification(&self, id: i64) -> Result<(), String> {
        sqlx::query("DELETE FROM notifications WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    /// Removes every notification.
    pub async fn clear_notifications(&self) -> Result<(), String> {
        sqlx::query("DELETE FROM notifications")
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(|error| error.to_string())
    }
}

/// A notification from its row. A row with a kind this build does not know is an error, so it is skipped.
fn notification_from_row(row: &SqliteRow) -> Result<Notification, String> {
    let kind: String = row.get("kind");
    let created_at = DateTime::<Utc>::from_timestamp(row.get::<i64, _>("created_at"), 0).ok_or("a notification has a time out of range")?;
    Ok(Notification {
        id: row.get("id"),
        kind: Kind::from_db(&kind).ok_or_else(|| format!("a notification has an unknown kind, {kind}"))?,
        subject: row.get("subject"),
        detail: row.get("detail"),
        instance_id: row.get("instance_id"),
        target: row.get("target"),
        created_at: created_at.with_timezone(&Local),
        read: row.get::<Option<i64>, _>("read_at").is_some(),
    })
}

#[cfg(test)]
#[path = "../../../tests/app/database/notifications.rs"]
mod tests;
