//! Keeping the paths that packs and platforms name inside the folder they belong to.
//!
//! A modpack's file list, a platform's file name and an archive entry are all written by someone else. Joined to a base
//! folder unchecked, `../` walks out of it and an absolute path replaces it. Every such path goes through [`contained`]
//! before it is written to.

use std::path::{Component, Path, PathBuf};

/// Whether `relative` is a plain relative path: `/`-separated names, none of them empty, `.` or `..`, and nothing that
/// makes a path absolute or names a drive on any system. Backslashes and colons are refused, because Windows reads them
/// as a separator and a drive or stream marker.
pub fn is_contained(relative: &str) -> bool {
    if relative.is_empty() || relative.contains(['\\', ':', '\0']) {
        return false;
    }
    let names_ok = relative.split('/').all(|name| !name.is_empty() && name != "." && name != "..");
    names_ok && Path::new(relative).components().all(|part| matches!(part, Component::Normal(_)))
}

/// `base` joined with `relative`, when `relative` stays inside `base` (see [`is_contained`]). `None` otherwise.
pub fn contained(base: &Path, relative: &str) -> Option<PathBuf> {
    is_contained(relative).then(|| base.join(relative))
}

#[cfg(test)]
#[path = "../tests/paths.rs"]
mod tests;
