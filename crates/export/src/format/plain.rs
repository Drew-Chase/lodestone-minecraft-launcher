//! The plain zip: every selected file at its own path, with a `lodestone.json` at the root so the links
//! survive. Nothing is moved into an overrides folder, so the archive holds the instance's own files.

use std::path::Path;

use super::Package;
use super::lodestone::{self, MANIFEST};
use crate::error::Result;
use crate::format::write_archive;
use crate::link::Entry;
use crate::progress::Control;

/// Writes the plain zip of `package` to `target`.
pub fn write(target: &Path, package: &Package<'_>, control: &mut Control<'_>) -> Result<()> {
    let mut manifest = lodestone::manifest(package, true);
    manifest.overrides.clear();
    let json = serde_json::to_vec_pretty(&manifest)?;
    let files: Vec<&Entry> = package.entries.iter().collect();
    write_archive(target, (MANIFEST, &json), &files, "", control)
}

#[cfg(test)]
#[path = "../../tests/format/plain.rs"]
mod tests;
