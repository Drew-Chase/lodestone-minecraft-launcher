//! The playtime the launcher keeps: when an instance was last launched, and how long its games have run. A run's time is
//! added as it goes, in checkpoints, so a launcher that stops early keeps the time it has already seen.

use chrono::{DateTime, Local};

use super::Database;

impl Database {
    /// Records that a run of the instance `id` started at `started`. The instance was last played then, unless a later run
    /// already set a later time.
    pub async fn record_launch(&self, id: &str, started: DateTime<Local>) -> Result<(), String> {
        sqlx::query("UPDATE instances SET last_played_at = MAX(COALESCE(last_played_at, 0), ?) WHERE id = ?")
            .bind(started.timestamp())
            .bind(id)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    /// Adds `seconds` of play to the playtime of the instance `id`. The state is left alone, because the instance may have
    /// other runs still going.
    pub async fn credit_playtime(&self, id: &str, seconds: u64) -> Result<(), String> {
        sqlx::query("UPDATE instances SET playtime_seconds = playtime_seconds + ? WHERE id = ?")
            .bind(i64::try_from(seconds).unwrap_or(i64::MAX))
            .bind(id)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(|error| error.to_string())
    }
}

#[cfg(test)]
#[path = "../../../tests/app/database/playtime.rs"]
mod tests;
