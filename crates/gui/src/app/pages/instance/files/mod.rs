//! What an instance folder holds on disk, read for the instance tabs.
//!
//! An instance's game directory keeps `mods/`, `saves/`, `screenshots/` and
//! `logs/`, the same layout `.minecraft` uses. [`scan`] reads all of them in one
//! pass on the blocking pool, and the page applies the result, so nothing here
//! runs on the UI thread.

mod log;
mod mods;
mod screenshots;
mod worlds;

use std::path::Path;

use iced::widget::image::Handle;

use crate::app::perf;

use super::logs::{LogLine, MAX_LINES};

pub use mods::ModFile;
pub use screenshots::ScreenshotFile;
pub use worlds::WorldFile;

/// Everything the instance tabs show, read from one instance folder.
#[derive(Debug, Clone, Default)]
pub struct InstanceFiles {
    /// The folder's total size on disk.
    pub folder_bytes: u64,
    /// The mod jars, enabled and disabled, sorted by name.
    pub mods: Vec<ModFile>,
    /// The saved worlds, most recently saved first.
    pub worlds: Vec<WorldFile>,
    /// The captures, most recent first.
    pub screenshots: Vec<ScreenshotFile>,
    /// The end of `logs/latest.log`, oldest line first.
    pub log: Vec<LogLine>,
}

/// Reads everything the instance tabs show from the instance folder at `folder`.
pub fn scan(folder: &Path) -> InstanceFiles {
    let _scope = perf::scope("io.scan");
    InstanceFiles {
        folder_bytes: dir_size(folder),
        mods: mods::scan(&folder.join("mods")),
        worlds: worlds::scan(&folder.join("saves")),
        screenshots: screenshots::scan(&folder.join("screenshots")),
        log: read_log(folder),
    }
}

/// The last [`MAX_LINES`] lines of the instance's `logs/latest.log`.
pub fn read_log(folder: &Path) -> Vec<LogLine> {
    let _scope = perf::scope("io.log");
    log::read_tail(&folder.join("logs").join("latest.log"), MAX_LINES)
}

/// How many mod jars the instance folder holds, enabled or not.
pub fn count_mods(folder: &Path) -> usize {
    mods::count(&folder.join("mods"))
}

/// Decodes the image at `path`, scaled down to fit `bounds` when given. `None`
/// when the file is missing or is not an image the decoder reads.
pub fn decode_image(path: &Path, bounds: Option<(u32, u32)>) -> Option<Handle> {
    let _scope = perf::scope("io.decode");
    let mut decoded = image::open(path).ok()?;
    if let Some((width, height)) = bounds {
        decoded = decoded.thumbnail(width, height);
    }
    let pixels = decoded.to_rgba8();
    Some(Handle::from_rgba(pixels.width(), pixels.height(), pixels.into_raw()))
}

/// The size of every file under `dir`, folders included. Symlinks are not
/// followed, so a link cannot make the walk loop or leave the instance.
fn dir_size(dir: &Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    entries
        .filter_map(Result::ok)
        .map(|entry| match entry.file_type() {
            Ok(kind) if kind.is_dir() => dir_size(&entry.path()),
            Ok(kind) if kind.is_file() => entry.metadata().map_or(0, |metadata| metadata.len()),
            _ => 0,
        })
        .sum()
}

#[cfg(test)]
#[path = "../../../../../tests/app/pages/instance/files/mod.rs"]
mod tests;
