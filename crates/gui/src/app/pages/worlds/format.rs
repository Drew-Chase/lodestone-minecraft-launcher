//! The relative times the table shows for when a world was last played: "2h ago", "Yesterday", "3d ago", "1w ago",
//! and a date for anything older.

use chrono::{DateTime, Local};

use crate::app::lang;

/// How long ago `saved` was, as seen from `now`, in the words the table uses.
pub fn format_ago(saved: DateTime<Local>, now: DateTime<Local>) -> String {
    let elapsed = now.signed_duration_since(saved);
    let minutes = elapsed.num_minutes().max(0);
    if minutes < 1 {
        return lang::t("worlds.ago.just_now").to_owned();
    }
    if minutes < 60 {
        return lang::t_with("worlds.ago.minutes", &[("minutes", minutes.to_string().as_str())]);
    }
    let hours = elapsed.num_hours();
    if hours < 24 {
        return lang::t_with("worlds.ago.hours", &[("hours", hours.to_string().as_str())]);
    }
    // Past a day, the calendar days between the two dates decide: "Yesterday" is the day before, even when
    // the save is less than 48 hours old.
    let days = now.date_naive().signed_duration_since(saved.date_naive()).num_days();
    match days {
        1 => lang::t("worlds.ago.yesterday").to_owned(),
        2..=6 => lang::t_with("worlds.ago.days", &[("days", days.to_string().as_str())]),
        7..=29 => lang::t_with("worlds.ago.weeks", &[("weeks", (days / 7).to_string().as_str())]),
        30.. => lang::t_with("worlds.ago.on", &[("date", saved.format("%b %-d, %Y").to_string().as_str())]),
        _ => lang::t_with("worlds.ago.hours", &[("hours", hours.to_string().as_str())]),
    }
}

#[cfg(test)]
#[path = "../../../../tests/app/pages/worlds/format.rs"]
mod tests;
