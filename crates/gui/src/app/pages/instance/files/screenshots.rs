//! The captures in `screenshots/`, listed from their files. The pixels are
//! decoded separately, on the page, as thumbnails.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::app::pages::instance::format::format_bytes;

/// One capture, read from its file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScreenshotFile {
    pub file_name: String,
    /// The file size as displayed, e.g. "2.1 MB".
    pub size_label: String,
    pub path: PathBuf,
}

/// The PNG captures in `dir`, most recent first.
pub fn scan(dir: &Path) -> Vec<ScreenshotFile> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut shots: Vec<(SystemTime, ScreenshotFile)> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .filter(|entry| entry.path().extension().is_some_and(|extension| extension.eq_ignore_ascii_case("png")))
        .filter_map(|entry| {
            let metadata = entry.metadata().ok()?;
            let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
            let shot = ScreenshotFile {
                file_name: entry.file_name().to_string_lossy().into_owned(),
                size_label: format_bytes(metadata.len()),
                path: entry.path(),
            };
            Some((modified, shot))
        })
        .collect();
    shots.sort_by_key(|(modified, _)| std::cmp::Reverse(*modified));
    shots.into_iter().map(|(_, shot)| shot).collect()
}

#[cfg(test)]
#[path = "../../../../../tests/app/pages/instance/files/screenshots.rs"]
mod tests;
