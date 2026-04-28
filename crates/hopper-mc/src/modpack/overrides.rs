use std::io::{Read, Seek};
use std::path::Path;

use crate::error::ContentError;

/// Prefixes to scan for override files in the archive.
const OVERRIDE_DIRS: &[&str] = &["overrides/"];
const CLIENT_OVERRIDE_DIRS: &[&str] = &["client-overrides/"];

/// Extract override files from a modpack archive into `dest`.
///
/// - Always extracts files under `overrides/`.
/// - When `client_only` is `true`, also extracts `client-overrides/`.
/// - Prefix directories are stripped so `overrides/config/foo.cfg` becomes `config/foo.cfg`.
/// - Returns the number of files extracted.
///
/// # Safety
///
/// Paths are sanitised: entries containing `..` or starting with `/` are rejected
/// to prevent directory-traversal (zip-slip) attacks.
pub fn extract_overrides<R: Read + Seek>(
    reader: R,
    dest: &Path,
    client_only: bool,
) -> crate::Result<u64> {
    let mut archive = zip::ZipArchive::new(reader)
        .map_err(|e| ContentError::InvalidArchive(format!("failed to open zip: {e}")))?;

    let mut prefixes: Vec<&str> = OVERRIDE_DIRS.to_vec();
    if client_only {
        prefixes.extend_from_slice(CLIENT_OVERRIDE_DIRS);
    }

    let mut count = 0u64;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| ContentError::InvalidArchive(format!("bad zip entry: {e}")))?;

        let raw_name = match entry.enclosed_name() {
            Some(name) => name.to_path_buf(),
            None => continue, // path sanitization failed — skip
        };

        let raw_str = raw_name.to_string_lossy();

        // Find which prefix this entry matches.
        let relative = prefixes
            .iter()
            .find_map(|prefix| raw_str.strip_prefix(prefix));

        let relative = match relative {
            Some(r) if !r.is_empty() => r,
            _ => continue,
        };

        // Extra safety: reject traversal attempts.
        if relative.contains("..") || relative.starts_with('/') || relative.starts_with('\\') {
            continue;
        }

        let target = dest.join(relative);

        if entry.is_dir() {
            std::fs::create_dir_all(&target)?;
        } else {
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut out = std::fs::File::create(&target)?;
            std::io::copy(&mut entry, &mut out)?;
            count += 1;
        }
    }

    Ok(count)
}
