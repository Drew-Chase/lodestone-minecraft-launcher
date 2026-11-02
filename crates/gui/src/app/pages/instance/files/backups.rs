//! The zips in an instance's `backups/` folder. `world_archive::back_up` writes each one as `<folder>-<stamp>.zip`:
//! the save's folder name, then the time the backup was made, in the shape `SHAPE` describes.

use std::path::{Component, Path, PathBuf};

use chrono::{DateTime, Local, NaiveDateTime};

/// The time `back_up` writes into a backup's name, in the local time zone.
const STAMP_FORMAT: &str = "%Y-%m-%d_%H.%M.%S";
/// The shape of that stamp: `d` is a digit, and anything else is a separator the stamp has as it is.
const SHAPE: &[u8] = b"dddd-dd-dd_dd.dd.dd";

/// One zip in the `backups/` folder.
#[derive(Debug, Clone)]
pub struct BackupFile {
    /// The zip's path.
    pub path: PathBuf,
    /// The folder name of the save the zip holds.
    pub folder: String,
    /// When the backup was made, from the stamp in its name.
    pub taken: DateTime<Local>,
    /// The zip's size on disk.
    pub size_bytes: u64,
}

/// Every backup in `dir`, newest first. A file that is not named as a backup is left out.
pub fn scan(dir: &Path) -> Vec<BackupFile> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut backups: Vec<BackupFile> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .filter_map(|entry| {
            let name = entry.file_name();
            let (folder, taken) = parse(name.to_str()?)?;
            Some(BackupFile {
                path: entry.path(),
                folder: folder.to_owned(),
                taken,
                size_bytes: entry.metadata().map_or(0, |metadata| metadata.len()),
            })
        })
        .collect();
    backups.sort_by_key(|backup| std::cmp::Reverse(backup.taken));
    backups
}

/// Whether `name` is a zip that `back_up` wrote for the save `folder`.
pub fn is_backup_of(name: &str, folder: &str) -> bool {
    parse(name).is_some_and(|(backed_up, _)| backed_up == folder)
}

/// The save folder and the time that `name` gives, when it is `<folder>-<stamp>.zip` with the stamp in `SHAPE`. The
/// stamp is read from the end of the name, so the folder may keep dashes of its own. The shape keeps a world named
/// "Hub" from matching a backup of "Hub-2020".
pub fn parse(name: &str) -> Option<(&str, DateTime<Local>)> {
    let stem = name.strip_suffix(".zip")?;
    let split = stem.len().checked_sub(SHAPE.len() + 1)?;
    let folder = stem.get(..split)?;
    let stamp = stem.get(split..)?.strip_prefix('-')?;
    if !is_folder_name(folder) || !looks_like_stamp(stamp) {
        return None;
    }
    let taken = NaiveDateTime::parse_from_str(stamp, STAMP_FORMAT)
        .ok()?
        .and_local_timezone(Local)
        .earliest()?;
    Some((folder, taken))
}

/// Whether `name` can name a save's folder: one ordinary path component, so it cannot reach outside `saves/`.
pub fn is_folder_name(name: &str) -> bool {
    let mut components = Path::new(name).components();
    !name.contains(std::path::is_separator) && matches!((components.next(), components.next()), (Some(Component::Normal(_)), None))
}

/// Whether `stamp` has the shape `SHAPE` describes.
fn looks_like_stamp(stamp: &str) -> bool {
    stamp.len() == SHAPE.len()
        && stamp.bytes().zip(SHAPE).all(|(byte, shape)| match shape {
            b'd' => byte.is_ascii_digit(),
            separator => byte == *separator,
        })
}

#[cfg(test)]
#[path = "../../../../../tests/app/pages/instance/files/backups.rs"]
mod tests;
