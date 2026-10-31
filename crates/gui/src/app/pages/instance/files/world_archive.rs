//! Backing a save up to a zip, and importing a zipped save into `saves/`. Both
//! run on the blocking pool: a save can be gigabytes.

use std::ffi::OsStr;
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Local};
use zip::write::SimpleFileOptions;

use crate::app::perf;

/// The lock a running game holds on its save. It is not part of the save, so a backup leaves it out.
const SESSION_LOCK: &str = "session.lock";
/// The folder an import falls back to when the archive's name gives nothing usable.
const FALLBACK_NAME: &str = "World";

/// Zips the save folder `world` into `backups` as `<folder>-<timestamp>.zip`, and returns the zip's
/// path. A zip that fails part way is removed.
pub fn back_up(world: &Path, backups: &Path, now: DateTime<Local>) -> Result<PathBuf, String> {
    let _scope = perf::scope("io.backup");
    let folder = world.file_name().ok_or_else(|| "the save has no folder name".to_string())?;
    let folder = folder.to_string_lossy().into_owned();
    std::fs::create_dir_all(backups).map_err(|error| error.to_string())?;
    let destination = backups.join(format!("{folder}-{}.zip", now.format("%Y-%m-%d_%H.%M.%S")));

    let file = File::create(&destination).map_err(|error| error.to_string())?;
    let mut archive = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let written = write_dir(&mut archive, world, &folder, options).and_then(|()| archive.finish().map(drop).map_err(|error| error.to_string()));
    if let Err(error) = written {
        let _ = std::fs::remove_file(&destination);
        return Err(error);
    }
    Ok(destination)
}

/// Writes every file under `dir` into `archive` beneath `prefix/`, leaving out the session lock.
/// Symlinks are not followed, as the size walk does not follow them.
fn write_dir(archive: &mut zip::ZipWriter<File>, dir: &Path, prefix: &str, options: SimpleFileOptions) -> Result<(), String> {
    let entries = std::fs::read_dir(dir).map_err(|error| error.to_string())?;
    for entry in entries {
        let entry = entry.map_err(|error| error.to_string())?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == SESSION_LOCK {
            continue;
        }
        let archived = format!("{prefix}/{name}");
        let kind = entry.file_type().map_err(|error| error.to_string())?;
        if kind.is_dir() {
            write_dir(archive, &entry.path(), &archived, options)?;
        } else if kind.is_file() {
            archive.start_file(archived, options).map_err(|error| error.to_string())?;
            let mut source = File::open(entry.path()).map_err(|error| error.to_string())?;
            io::copy(&mut source, archive).map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

/// Extracts the save in the zip at `archive_path` into `saves`, under a folder name no save uses yet,
/// and returns that name. The save is the archive's root or its one top folder, and must hold a
/// `level.dat`. An entry that would land outside the save refuses the whole archive. A failed
/// extraction leaves no folder behind.
pub fn import(archive_path: &Path, saves: &Path) -> Result<String, String> {
    let _scope = perf::scope("io.import");
    let file = File::open(archive_path).map_err(|error| error.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|error| error.to_string())?;

    let names: Vec<PathBuf> = (0..archive.len())
        .map(|index| {
            let entry = archive.by_index(index).map_err(|error| error.to_string())?;
            entry.enclosed_name().ok_or_else(|| {
                let name = entry.name().map(|name| name.into_owned()).unwrap_or_default();
                format!("the archive entry “{name}” leaves the save folder")
            })
        })
        .collect::<Result<_, _>>()?;
    let prefix = save_prefix(&names).ok_or_else(|| "the archive holds no level.dat".to_string())?;

    let stem = archive_path
        .file_stem()
        .map(|stem| sanitize(&stem.to_string_lossy()))
        .filter(|stem| !stem.is_empty())
        .unwrap_or_else(|| FALLBACK_NAME.to_string());
    let folder = unique_name(saves, &stem);
    let target = saves.join(&folder);
    std::fs::create_dir_all(&target).map_err(|error| error.to_string())?;

    let extracted = extract(&mut archive, &names, &prefix, &target);
    if let Err(error) = extracted {
        let _ = std::fs::remove_dir_all(&target);
        return Err(error);
    }
    Ok(folder)
}

/// The folder the save sits in, relative to the archive root: the empty path for a save at the root,
/// or its one top folder. `None` when no `level.dat` is that shallow.
fn save_prefix(names: &[PathBuf]) -> Option<PathBuf> {
    names
        .iter()
        .filter(|name| name.file_name() == Some(OsStr::new("level.dat")))
        .filter_map(|name| name.parent().map(Path::to_path_buf))
        .filter(|parent| parent.components().count() <= 1)
        .min_by_key(|parent| parent.components().count())
}

/// Writes the entries under `prefix` into `target`, with the prefix stripped from each name.
fn extract(archive: &mut zip::ZipArchive<File>, names: &[PathBuf], prefix: &Path, target: &Path) -> Result<(), String> {
    for (index, name) in names.iter().enumerate() {
        let Ok(relative) = name.strip_prefix(prefix) else {
            continue;
        };
        if relative.as_os_str().is_empty() {
            continue;
        }
        let destination = target.join(relative);
        let mut entry = archive.by_index(index).map_err(|error| error.to_string())?;
        if entry.is_dir() {
            std::fs::create_dir_all(&destination).map_err(|error| error.to_string())?;
            continue;
        }
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let mut output = File::create(&destination).map_err(|error| error.to_string())?;
        io::copy(&mut entry, &mut output).map_err(|error| error.to_string())?;
    }
    Ok(())
}

/// `stem` if no folder in `saves` has that name, otherwise `stem (2)`, `stem (3)`, and so on.
fn unique_name(saves: &Path, stem: &str) -> String {
    let mut name = stem.to_string();
    let mut taken = 2;
    while saves.join(&name).exists() {
        name = format!("{stem} ({taken})");
        taken += 1;
    }
    name
}

/// `name` with the characters a file name cannot hold replaced by underscores, and its ends trimmed.
fn sanitize(name: &str) -> String {
    name.chars()
        .map(|character| {
            if "/\\:*?\"<>|".contains(character) || character.is_control() {
                '_'
            } else {
                character
            }
        })
        .collect::<String>()
        .trim()
        .to_string()
}

#[cfg(test)]
#[path = "../../../../../tests/app/pages/instance/files/world_archive.rs"]
mod tests;
