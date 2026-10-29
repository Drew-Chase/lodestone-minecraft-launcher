//! Display formatting for the instance tabs: byte sizes and play times.

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
        if rest == 0 { format!("{hours}h") } else { format!("{hours}h {rest}m") }
    } else if minutes > 0 {
        format!("{minutes} min")
    } else {
        "—".to_string()
    }
}

#[cfg(test)]
#[path = "../../../../tests/app/pages/instance/format.rs"]
mod tests;
