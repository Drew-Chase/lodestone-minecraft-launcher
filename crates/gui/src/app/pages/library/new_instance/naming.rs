//! Instance folder names: a display name cleaned for every filesystem the
//! launcher runs on, then made unique inside the instances directory.

use std::path::Path;

/// Device names Windows reserves. A folder named with one of them (with or
/// without an extension) cannot be created there.
const RESERVED: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5",
    "LPT6", "LPT7", "LPT8", "LPT9",
];

/// The folder name fallback when nothing printable is left of the name.
const FALLBACK: &str = "Instance";

/// The longest folder name kept, in characters. Long names are cut here.
const MAX_CHARS: usize = 96;

/// The folder name for `name`. Characters that filesystems reject are dropped,
/// leading and trailing dots and spaces are trimmed (Windows strips trailing
/// ones silently), reserved device names get an underscore, and the result is
/// capped at [`MAX_CHARS`]. Spaces stay, so "Dragon Survival v2" is unchanged.
pub fn clean_directory_name(name: &str) -> String {
    let kept: String = name
        .chars()
        .filter(|c| !c.is_control() && !matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'))
        .take(MAX_CHARS)
        .collect();
    let trimmed = kept.trim_matches(|c: char| c == '.' || c.is_whitespace());
    if trimmed.is_empty() {
        return FALLBACK.to_string();
    }
    let stem = trimmed.split('.').next().unwrap_or_default().to_ascii_uppercase();
    if RESERVED.contains(&stem.as_str()) {
        return format!("_{trimmed}");
    }
    trimmed.to_string()
}

/// A folder name that no folder in `instances_dir` uses yet: `base`, then
/// `base (2)`, `base (3)`, and so on. Case-insensitive filesystems count
/// `Base` and `base` as the same folder, and `Path::exists` already agrees.
pub fn unique_directory_name(instances_dir: &Path, base: &str) -> String {
    let mut candidate = base.to_string();
    let mut suffix = 2;
    while instances_dir.join(&candidate).exists() {
        candidate = format!("{base} ({suffix})");
        suffix += 1;
    }
    candidate
}

#[cfg(test)]
#[path = "../../../../../tests/app/pages/library/new_instance/naming.rs"]
mod tests;
