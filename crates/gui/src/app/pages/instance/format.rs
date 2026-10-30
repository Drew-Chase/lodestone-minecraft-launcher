//! Display formatting for the instance tabs: byte sizes and play times.

use crate::app::lang;

/// A byte count in the launcher's compact style: `2.1 MB`, `412 MB`, `1.4 GB`.
/// Values under 10 keep one decimal place; larger values round to whole units.
pub fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    let value = bytes as f64;
    let (scaled, unit) = if value >= GB {
        (value / GB, "GB")
    } else if value >= MB {
        (value / MB, "MB")
    } else {
        (value / KB, "KB")
    };

    if scaled < 10.0 {
        format!("{scaled:.1} {unit}")
    } else {
        format!("{scaled:.0} {unit}")
    }
}

/// Playtime, humanized — "24h 12m", "12 min", "—" (the Overview's form).
pub fn format_playtime(seconds: u32) -> String {
    let minutes = seconds / 60;
    let hours = minutes / 60;
    if hours > 0 {
        let rest = minutes % 60;
        let hours = hours.to_string();
        if rest == 0 {
            lang::t_with("instance.playtime.hours", &[("hours", hours.as_str())])
        } else {
            let rest = rest.to_string();
            lang::t_with(
                "instance.playtime.hours_minutes",
                &[("hours", hours.as_str()), ("minutes", rest.as_str())],
            )
        }
    } else if minutes > 0 {
        let minutes = minutes.to_string();
        lang::t_with("instance.playtime.minutes", &[("minutes", minutes.as_str())])
    } else {
        "—".to_string()
    }
}

#[cfg(test)]
#[path = "../../../../tests/app/pages/instance/format.rs"]
mod tests;
