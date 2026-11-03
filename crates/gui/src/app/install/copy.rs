//! Copies a launcher's game folder into a new instance. The walk takes the files under the folder, leaves out the names a
//! launcher keeps for itself, and follows a symbolic link that points at a file, since a launcher may share a file that way.
//! A file that cannot be copied is logged and counted, so one locked file does not end the import.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use lodestone_export::plan::is_leftover;
use tracing::warn;

/// The top-level names no copy takes: the logs, and the downloads a launcher shares between its instances.
const ALWAYS_SKIPPED: [&str; 11] = [
    "logs",
    "crash-reports",
    "versions",
    "libraries",
    "assets",
    "runtime",
    "natives",
    ".fabric",
    ".quilt",
    ".mixin.out",
    "usercache.json",
];

/// The prefix of the files a launcher keeps accounts and sign-in tokens in. None of them are copied.
const CREDENTIAL_PREFIX: &str = "launcher_";

/// One file the copy takes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Planned {
    /// The path inside the instance, with `/` separators.
    pub path: String,
    /// Where the bytes are read from.
    pub source: PathBuf,
    /// The file's size in bytes.
    pub size: u64,
}

/// What a copy did.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Report {
    /// The files that were copied.
    pub copied: usize,
    /// The files that could not be, and were left out.
    pub failed: usize,
}

/// Lists the files under `root` that a copy takes, sorted by path. `skip` names top-level entries to leave out, on top of
/// the names every copy skips and the launcher's credential files.
pub fn plan(root: &Path, skip: &[String]) -> io::Result<Vec<Planned>> {
    let mut planned = Vec::new();
    let mut folders = vec![(root.to_path_buf(), String::new())];
    while let Some((folder, prefix)) = folders.pop() {
        let entries = match fs::read_dir(&folder) {
            Ok(entries) => entries,
            // The root must be readable. A folder inside it that cannot be read is left out.
            Err(error) if !prefix.is_empty() => {
                warn!("could not read {}: {error}", folder.display());
                continue;
            }
            Err(error) => return Err(error),
        };
        for entry in entries {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if is_leftover(&name) || (prefix.is_empty() && is_skipped(&name, skip)) {
                continue;
            }
            let path = if prefix.is_empty() { name } else { format!("{prefix}/{name}") };
            let source = entry.path();
            let file_type = entry.file_type()?;
            if file_type.is_symlink() {
                // A link to a folder is left out, since it may lead anywhere. A link to a file is copied as the file.
                if let Ok(meta) = fs::metadata(&source)
                    && meta.is_file()
                {
                    planned.push(Planned {
                        path,
                        source,
                        size: meta.len(),
                    });
                }
            } else if file_type.is_dir() {
                folders.push((source, path));
            } else if file_type.is_file() {
                let size = entry.metadata()?.len();
                planned.push(Planned { path, source, size });
            }
        }
    }
    planned.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(planned)
}

/// The bytes the planned files come to.
pub fn total_bytes(planned: &[Planned]) -> u64 {
    planned.iter().map(|file| file.size).sum()
}

/// Copies the planned files into `destination`, checking `abandoned` before each one. A file that cannot be copied is logged
/// and counted, and the copy goes on. An abandoned copy stops with an error.
pub fn copy_into(planned: &[Planned], destination: &Path, abandoned: &AtomicBool) -> io::Result<Report> {
    let mut report = Report::default();
    for file in planned {
        if abandoned.load(Ordering::Relaxed) {
            return Err(io::Error::new(io::ErrorKind::Interrupted, "the copy was abandoned"));
        }
        let target = destination.join(&file.path);
        let copied = match target.parent() {
            Some(parent) => fs::create_dir_all(parent).and_then(|()| fs::copy(&file.source, &target)),
            None => fs::copy(&file.source, &target),
        };
        match copied {
            Ok(_) => report.copied += 1,
            Err(error) => {
                warn!("could not copy {}: {error}", file.source.display());
                report.failed += 1;
            }
        }
    }
    Ok(report)
}

/// Whether the top-level entry `name` is left out of a copy.
fn is_skipped(name: &str, skip: &[String]) -> bool {
    ALWAYS_SKIPPED.contains(&name) || name.starts_with(CREDENTIAL_PREFIX) || skip.iter().any(|skipped| skipped == name)
}

#[cfg(test)]
#[path = "../../../tests/app/install/copy.rs"]
mod tests;
