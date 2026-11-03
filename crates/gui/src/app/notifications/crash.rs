//! The crash reports an instance's game writes into its `crash-reports` folder, and the newest one a run left behind.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// The newest crash report in `folder`'s `crash-reports` folder that was written at or after `since`. Minecraft writes
/// its reports as `.txt` files, so other files are not reports.
pub fn newest_report_since(folder: &Path, since: SystemTime) -> Option<PathBuf> {
    std::fs::read_dir(folder.join("crash-reports"))
        .ok()?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|extension| extension == "txt"))
        .filter_map(|entry| {
            let modified = entry.metadata().ok()?.modified().ok()?;
            (modified >= since).then(|| (modified, entry.path()))
        })
        .max_by_key(|(modified, _)| *modified)
        .map(|(_, path)| path)
}

#[cfg(test)]
#[path = "../../../tests/app/notifications/crash.rs"]
mod tests;
