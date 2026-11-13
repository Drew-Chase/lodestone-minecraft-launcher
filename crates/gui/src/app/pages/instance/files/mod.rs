//! What an instance folder holds on disk, read for the instance tabs.
//!
//! An instance's game directory keeps `mods/`, `saves/`, `screenshots/` and
//! `logs/`, the same layout `.minecraft` uses. [`scan`] reads all of them in one
//! pass on the blocking pool, and the page applies the result, so nothing here
//! runs on the UI thread.

mod backups;
mod log;
pub mod mod_index;
mod mods;
mod packs;
mod screenshots;
mod servers;
pub mod world_archive;
mod worlds;

use std::path::Path;

use flate2::read::GzDecoder;
use iced::widget::image::Handle;
use serde::de::DeserializeOwned;

use crate::app::perf;

use super::logs::{LogLine, MAX_LINES};

pub use backups::BackupFile;
pub use mods::{ModFile, switched_name};
pub use packs::{PackFile, name_of as pack_name};
pub use screenshots::ScreenshotFile;
pub use servers::ServerFile;
pub use worlds::{Difficulty, GameMode, WorldFile};

/// Everything the instance tabs show, read from one instance folder.
#[derive(Debug, Clone, Default)]
pub struct InstanceFiles {
    /// The folder's total size on disk.
    pub folder_bytes: u64,
    /// The mod jars, enabled and disabled, sorted by name.
    pub mods: Vec<ModFile>,
    /// The resource packs in `resourcepacks/`, sorted by name.
    pub resource_packs: Vec<PackFile>,
    /// The shader packs in `shaderpacks/`, sorted by name.
    pub shader_packs: Vec<PackFile>,
    /// The saved worlds, most recently saved first.
    pub worlds: Vec<WorldFile>,
    /// The servers the game's `servers.dat` lists, in the list's order.
    pub servers: Vec<ServerFile>,
    /// The captures, most recent first.
    pub screenshots: Vec<ScreenshotFile>,
    /// The end of `logs/latest.log`, oldest line first.
    pub log: Vec<LogLine>,
}

/// The saved worlds in the instance folder's `saves/`, most recently saved first.
pub fn scan_worlds(folder: &Path) -> Vec<WorldFile> {
    let _scope = perf::scope("io.scan_worlds");
    worlds::scan(&folder.join("saves"))
}

/// The backups in the instance folder's `backups/`, newest first.
pub fn scan_backups(folder: &Path) -> Vec<BackupFile> {
    let _scope = perf::scope("io.scan_backups");
    backups::scan(&folder.join("backups"))
}

/// The captures in the instance folder's `screenshots/`, most recent first.
pub fn scan_screenshots(folder: &Path) -> Vec<ScreenshotFile> {
    screenshots::scan(&folder.join("screenshots"))
}

/// The servers the game's `servers.dat` in the instance folder lists, in the list's order. A file that is missing or does
/// not decode lists none.
pub fn scan_servers(folder: &Path) -> Vec<ServerFile> {
    let _scope = perf::scope("io.scan_servers");
    servers::scan(folder)
}

/// The save in `path`, read again. `None` when its folder has no `level.dat`.
pub fn read_world(path: &Path) -> Option<WorldFile> {
    worlds::read_world(path)
}

/// The capture in `path`, read again. `None` when it is no longer a PNG file.
pub fn read_shot(path: &Path) -> Option<ScreenshotFile> {
    screenshots::read(path)
}

/// Reads everything the instance tabs show from the instance folder at `folder`. Each part reads its own folder on its own
/// thread, so the page waits for the slowest part rather than for all of them in turn.
pub fn scan(folder: &Path) -> InstanceFiles {
    let _scope = perf::scope("io.scan");
    std::thread::scope(|scope| {
        let folder_bytes = scope.spawn(|| dir_size(folder));
        let mods = scope.spawn(|| mods::scan(&folder.join("mods")));
        let resource_packs = scope.spawn(|| packs::scan(&folder.join("resourcepacks")));
        let shader_packs = scope.spawn(|| packs::scan(&folder.join("shaderpacks")));
        let worlds = scope.spawn(|| worlds::scan(&folder.join("saves")));
        let servers = scope.spawn(|| scan_servers(folder));
        let screenshots = scope.spawn(|| screenshots::scan(&folder.join("screenshots")));
        let log = read_log(folder);
        InstanceFiles {
            folder_bytes: folder_bytes.join().unwrap_or_default(),
            mods: mods.join().unwrap_or_default(),
            resource_packs: resource_packs.join().unwrap_or_default(),
            shader_packs: shader_packs.join().unwrap_or_default(),
            worlds: worlds.join().unwrap_or_default(),
            servers: servers.join().unwrap_or_default(),
            screenshots: screenshots.join().unwrap_or_default(),
            log,
        }
    })
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

/// The mod jars the instance folder holds, read for their metadata and sorted by name, without the rest of the
/// folder. The Mods tab rescans with this after it changes a jar.
pub fn scan_mods(folder: &Path) -> Vec<ModFile> {
    let _scope = perf::scope("io.scan_mods");
    mods::scan(&folder.join("mods"))
}

/// The datapacks a world holds in its `datapacks/` folder, sorted by name. The datapack dialog reads them again after it adds
/// or removes one.
pub fn scan_datapacks(world: &Path) -> Vec<PackFile> {
    let _scope = perf::scope("io.scan_datapacks");
    packs::scan(&world.join("datapacks"))
}

/// The resource packs and the shader packs the instance folder holds, each sorted by name. The Content tab reads them
/// again after it adds or removes one.
pub fn scan_packs(folder: &Path) -> (Vec<PackFile>, Vec<PackFile>) {
    let _scope = perf::scope("io.scan_packs");
    (packs::scan(&folder.join("resourcepacks")), packs::scan(&folder.join("shaderpacks")))
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

/// How far the backdrop blur spreads, in pixels of the icon. An icon is 64px, so this turns it into a soft colour
/// field rather than a second copy of the picture.
const BACKDROP_BLUR: f32 = 4.0;

/// Decodes a world's `icon.png` at full size, and a blurred copy of it for the card's backdrop. `None` when the
/// file is missing or is not an image the decoder reads.
pub fn decode_world_art(path: &Path) -> Option<(Handle, Handle)> {
    let _scope = perf::scope("io.decode");
    let icon = image::open(path).ok()?.to_rgba8();
    let backdrop = image::imageops::blur(&icon, BACKDROP_BLUR);
    Some((rgba_handle(icon), rgba_handle(backdrop)))
}

/// A handle over the RGBA pixels of `pixels`.
fn rgba_handle(pixels: image::RgbaImage) -> Handle {
    Handle::from_rgba(pixels.width(), pixels.height(), pixels.into_raw())
}

/// Decodes the image at `path` at full size, as RGBA pixels. The error says why it would not decode.
pub fn decode_rgba(path: &Path) -> Result<image::RgbaImage, String> {
    let _scope = perf::scope("io.decode");
    image::open(path).map(|decoded| decoded.to_rgba8()).map_err(|error| error.to_string())
}

/// Moves `path` to the OS trash, a folder with everything in it included. The trash keeps it until it is
/// emptied, so it can be restored.
pub fn move_to_trash(path: &Path) -> Result<(), String> {
    let _scope = perf::scope("io.trash");
    trash::delete(path).map_err(|error| error.to_string())
}

/// The first two bytes of a gzip stream.
const GZIP_MAGIC: [u8; 2] = [0x1f, 0x8b];

/// Decodes the NBT file at `path`. The game gzip-compresses its files, but a file without the gzip header is read
/// as it is. `None` when the file is missing or does not decode.
fn read_nbt<T: DeserializeOwned>(path: &Path) -> Option<T> {
    let bytes = std::fs::read(path).ok()?;
    if bytes.starts_with(&GZIP_MAGIC) {
        fastnbt::from_reader(GzDecoder::new(&bytes[..])).ok()
    } else {
        fastnbt::from_bytes(&bytes).ok()
    }
}

/// The size of every file under `dir`, folders included. Symlinks are not
/// followed, so a link cannot make the walk loop or leave the instance. The folders directly inside `dir` are walked on
/// their own threads, since a large instance (worlds, logs, mods) takes many file reads.
fn dir_size(dir: &Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    let mut files = 0;
    let mut folders = Vec::new();
    for entry in entries.filter_map(Result::ok) {
        match entry.file_type() {
            Ok(kind) if kind.is_dir() => folders.push(entry.path()),
            Ok(kind) if kind.is_file() => files += entry.metadata().map_or(0, |metadata| metadata.len()),
            _ => {}
        }
    }
    std::thread::scope(|scope| {
        let walks: Vec<_> = folders.iter().map(|folder| scope.spawn(move || walk_size(folder))).collect();
        files + walks.into_iter().filter_map(|walk| walk.join().ok()).sum::<u64>()
    })
}

/// The size of every file under `dir`, on this thread. Symlinks are not followed.
fn walk_size(dir: &Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    entries
        .filter_map(Result::ok)
        .map(|entry| match entry.file_type() {
            Ok(kind) if kind.is_dir() => walk_size(&entry.path()),
            Ok(kind) if kind.is_file() => entry.metadata().map_or(0, |metadata| metadata.len()),
            _ => 0,
        })
        .sum()
}

#[cfg(test)]
#[path = "../../../../../tests/app/pages/instance/files/mod.rs"]
mod tests;
