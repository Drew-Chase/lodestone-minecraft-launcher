//! Copying, moving and removing a whole entry: a file, a link, or a folder with everything in it.
//!
//! A link is always handled as a link. It is copied as a link and removed as a link, and never followed, so a link to a
//! folder above it cannot copy the same files forever, and removing a link never removes what it points at.

use std::fs;
use std::io;
use std::path::Path;

/// Copies a file, or a folder and everything in it, to `target`, which must not exist. Links stay links.
pub fn copy_tree(source: &Path, target: &Path) -> io::Result<()> {
    let kind = fs::symlink_metadata(source)?.file_type();
    if kind.is_symlink() {
        return copy_link(source, target);
    }
    if kind.is_dir() {
        fs::create_dir(target)?;
        for entry in fs::read_dir(source)? {
            let entry = entry?;
            copy_tree(&entry.path(), &target.join(entry.file_name()))?;
        }
        return Ok(());
    }
    fs::copy(source, target).map(|_| ())
}

#[cfg(unix)]
fn copy_link(source: &Path, target: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(fs::read_link(source)?, target)
}

#[cfg(not(unix))]
fn copy_link(source: &Path, target: &Path) -> io::Result<()> {
    fs::copy(source, target).map(|_| ())
}

/// Removes a file, a link, or a folder with everything in it. A link is removed as a link.
pub fn remove_tree(path: &Path) -> io::Result<()> {
    if fs::symlink_metadata(path)?.file_type().is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

/// Moves a file, a link or a folder to `target`, which must not exist. A rename does it when both are on one drive.
/// Across drives the entry is copied and then removed, and a copy that fails is removed again, so the source is left whole.
pub fn move_tree(source: &Path, target: &Path) -> io::Result<()> {
    match fs::rename(source, target) {
        Err(error) if error.kind() == io::ErrorKind::CrossesDevices => {
            if let Err(error) = copy_tree(source, target) {
                let _ = remove_tree(target);
                return Err(error);
            }
            remove_tree(source)
        }
        result => result,
    }
}

#[cfg(test)]
#[path = "../tests/tree.rs"]
mod tests;
