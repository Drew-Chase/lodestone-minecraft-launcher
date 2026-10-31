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
    /// When the file was last written. A capture whose write time or size moved has changed.
    pub modified: SystemTime,
    /// The file size in bytes.
    pub bytes: u64,
}

/// The PNG captures in `dir`, most recent first.
pub fn scan(dir: &Path) -> Vec<ScreenshotFile> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut shots: Vec<ScreenshotFile> = entries.filter_map(Result::ok).filter_map(|entry| read(&entry.path())).collect();
    shots.sort_by_key(|shot| std::cmp::Reverse(shot.modified));
    shots
}

/// The capture at `path`, read again from its file. `None` when the path is not a PNG file, or is gone.
pub fn read(path: &Path) -> Option<ScreenshotFile> {
    if !path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("png")) {
        return None;
    }
    // A symlink is not followed, so a link cannot list a file from outside the instance.
    let metadata = std::fs::symlink_metadata(path).ok()?;
    if !metadata.is_file() {
        return None;
    }
    Some(ScreenshotFile {
        file_name: path.file_name()?.to_string_lossy().into_owned(),
        size_label: format_bytes(metadata.len()),
        path: path.to_path_buf(),
        modified: metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
        bytes: metadata.len(),
    })
}

#[cfg(test)]
#[path = "../../../../../tests/app/pages/instance/files/screenshots.rs"]
mod tests;
