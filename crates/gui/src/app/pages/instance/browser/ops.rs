//! The file work the Files tab asks for: copying and moving entries into a folder, renaming one, and making a new file
//! or folder. Everything here blocks, so it runs on the blocking pool.
//!
//! Every path the tab changes is the instance folder joined with a path [`inside`] has checked, so nothing outside the
//! instance is written. A link is copied, moved and replaced as a link, never followed.

use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use lodestone_fs::{copy_tree, move_tree};

/// The characters no file name may hold on Windows. They are refused everywhere, so an instance moves between systems.
const FORBIDDEN: [char; 9] = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
/// The names Windows keeps for devices, with or without an extension.
const RESERVED: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5",
    "LPT6", "LPT7", "LPT8", "LPT9",
];

/// The absolute path of `relative` inside the instance at `root`. `None` when the path would leave the instance: one
/// with a `..`, a root or a drive in it. The empty path is the instance folder itself.
pub fn inside(root: &Path, relative: &str) -> Option<PathBuf> {
    let relative = Path::new(relative);
    relative
        .components()
        .all(|component| matches!(component, Component::Normal(_)))
        .then(|| root.join(relative))
}

/// Why a name cannot be a file's name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameError {
    /// The name is empty.
    Empty,
    /// The name holds a character no file name may hold.
    Character(char),
    /// The name ends in a dot or a space, which Windows drops.
    Ending,
    /// The name is one the system keeps for itself, such as `.`, `..` or `CON`.
    Reserved,
}

/// Whether `name` can be the name of a file or a folder on every system the launcher runs on.
pub fn valid_name(name: &str) -> Result<(), NameError> {
    if name.is_empty() {
        return Err(NameError::Empty);
    }
    if let Some(found) = name.chars().find(|character| FORBIDDEN.contains(character) || character.is_control()) {
        return Err(NameError::Character(found));
    }
    if name.ends_with('.') || name.ends_with(' ') {
        // `.` and `..` end in a dot too, and they are reserved rather than badly ended.
        return Err(if name == "." || name == ".." {
            NameError::Reserved
        } else {
            NameError::Ending
        });
    }
    let stem = name.split('.').next().unwrap_or(name);
    if RESERVED.iter().any(|reserved| reserved.eq_ignore_ascii_case(stem)) {
        return Err(NameError::Reserved);
    }
    Ok(())
}

/// Whether anything is at `path`. A link counts even when it points nowhere.
fn exists(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok()
}

/// `name` with `suffix` before its extension: `notes.txt` and ` 2` make `notes 2.txt`. A leading dot is part of the
/// name, so `.gitignore` takes the suffix at its end.
fn suffixed(name: &str, suffix: &str) -> String {
    match name.rfind('.') {
        Some(dot) if dot > 0 => format!("{}{suffix}{}", &name[..dot], &name[dot..]),
        _ => format!("{name}{suffix}"),
    }
}

/// A name in `folder` that nothing has yet: `name`, or `name` with a number when it is taken (`New folder 2`).
pub fn free_name(folder: &Path, name: &str) -> String {
    (1..)
        .map(|number| {
            if number == 1 {
                name.to_string()
            } else {
                suffixed(name, &format!(" {number}"))
            }
        })
        .find(|candidate| !exists(&folder.join(candidate)))
        .unwrap_or_else(|| name.to_string())
}

/// The name a copy of `name` takes beside the original: `notes (copy).txt`, then `notes (copy 2).txt`. `word` is the
/// word for "copy" in the active language.
pub fn copy_name(folder: &Path, name: &str, word: &str) -> String {
    (1..)
        .map(|number| {
            let suffix = if number == 1 {
                format!(" ({word})")
            } else {
                format!(" ({word} {number})")
            };
            suffixed(name, &suffix)
        })
        .find(|candidate| !exists(&folder.join(candidate)))
        .unwrap_or_else(|| name.to_string())
}

/// Whether a transfer copies its sources or moves them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Copy,
    Move,
}

/// What a transfer does about a name the target folder already holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Existing {
    /// Leaves both entries as they are and reports the source, so the user can be asked.
    Report,
    /// Sends the entry in the target folder to the trash, then puts the source in its place.
    Replace,
}

/// Entries to put into a folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transfer {
    /// The entries to copy or move, as absolute paths. They may lie outside the instance, as dropped files do.
    pub sources: Vec<PathBuf>,
    /// The folder that takes them, as an absolute path inside the instance.
    pub into: PathBuf,
    pub mode: Mode,
}

/// What a transfer did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    /// The entries written, as they are now named in the target folder.
    pub written: Vec<PathBuf>,
    /// The sources left alone because the target folder already holds their name.
    pub conflicts: Vec<PathBuf>,
    /// The sources that failed, each with its reason.
    pub errors: Vec<Failure>,
}

/// A source a transfer could not put in place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    pub name: String,
    pub reason: Reason,
}

/// Why a source could not be put in place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reason {
    /// The source is a folder, and the target is that folder or lies inside it.
    IntoItself,
    /// The file system refused, with its own words.
    Io(String),
}

/// Runs `job`. A copy into the folder the source already sits in makes a copy beside it, named with `copy_word`, and a
/// move into that folder does nothing. `trash` sends a replaced entry to the trash.
pub fn transfer(job: &Transfer, existing: Existing, copy_word: &str, trash: impl Fn(&Path) -> Result<(), String>) -> Report {
    let mut report = Report::default();
    for source in &job.sources {
        let Some(name) = source.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let fail = |reason| Failure {
            name: name.to_string(),
            reason,
        };
        let beside = source.parent() == Some(job.into.as_path());
        if beside && job.mode == Mode::Move {
            continue;
        }
        let is_folder = fs::symlink_metadata(source).is_ok_and(|metadata| metadata.is_dir());
        if is_folder && job.into.starts_with(source) {
            report.errors.push(fail(Reason::IntoItself));
            continue;
        }
        let target = if beside {
            job.into.join(copy_name(&job.into, name, copy_word))
        } else {
            job.into.join(name)
        };
        if exists(&target) {
            match existing {
                Existing::Report => {
                    report.conflicts.push(source.clone());
                    continue;
                }
                Existing::Replace => {
                    if let Err(error) = trash(&target) {
                        report.errors.push(fail(Reason::Io(error)));
                        continue;
                    }
                }
            }
        }
        let outcome = match job.mode {
            Mode::Copy => copy_tree(source, &target),
            Mode::Move => move_tree(source, &target),
        };
        match outcome {
            Ok(()) => report.written.push(target),
            Err(error) => report.errors.push(fail(Reason::Io(error.to_string()))),
        }
    }
    report
}

/// Why an entry could not be renamed or made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MakeError {
    /// The name cannot be a file's name.
    Name(NameError),
    /// The folder already holds an entry with that name.
    Taken,
    /// The file system refused, with its own words.
    Io(String),
}

impl From<io::Error> for MakeError {
    fn from(error: io::Error) -> Self {
        Self::Io(error.to_string())
    }
}

/// Renames the entry at `path` to `name`, in the folder it sits in. Returns its new path. A name that differs only by
/// case is a rename of the entry itself, so it is not refused as taken.
pub fn rename(path: &Path, name: &str) -> Result<PathBuf, MakeError> {
    valid_name(name).map_err(MakeError::Name)?;
    let target = path.with_file_name(name);
    if target == path {
        return Ok(target);
    }
    let same_entry = path
        .file_name()
        .and_then(|old| old.to_str())
        .is_some_and(|old| old.eq_ignore_ascii_case(name));
    if exists(&target) && !same_entry {
        return Err(MakeError::Taken);
    }
    fs::rename(path, &target)?;
    Ok(target)
}

/// What [`create`] makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    File,
    Folder,
}

/// Makes an empty file or a folder in `folder`, named `name` or the next free name after it. Returns its path.
pub fn create(folder: &Path, name: &str, kind: Kind) -> Result<PathBuf, MakeError> {
    valid_name(name).map_err(MakeError::Name)?;
    let path = folder.join(free_name(folder, name));
    match kind {
        Kind::Folder => fs::create_dir(&path)?,
        // `create_new` refuses a name another process took since the free name was picked.
        Kind::File => drop(fs::OpenOptions::new().write(true).create_new(true).open(&path)?),
    }
    Ok(path)
}

#[cfg(test)]
#[path = "../../../../../tests/app/pages/instance/browser/ops.rs"]
mod tests;
