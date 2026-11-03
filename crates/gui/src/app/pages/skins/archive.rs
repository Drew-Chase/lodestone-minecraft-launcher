//! The archive: the skins the player wore before Lodestone replaced them. Mojang's profile keeps only the skin in use, so
//! the archive is the only history of skins the launcher can build. Each skin is one PNG named by its digest and its
//! model, so a skin kept twice in the same model is kept once.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::library::{self, SavedSkin};

/// The folder the archive lives in, beside the saved skins.
pub fn dir(skins: &Path) -> PathBuf {
    skins.join("archive")
}

/// The name a skin is archived under: its digest, then the model it was worn in.
fn file_name(digest: &str, slim: bool) -> String {
    format!("{digest}-{}.png", if slim { "slim" } else { "classic" })
}

/// Keeps `png`, worn in the given model, in the archive. A skin that is already archived in that model, or is one of
/// `known` (the digests of the saved and bundled skins), is not kept again. The file is written under a temporary name and
/// renamed, so a half-written skin never shows in the archive. Returns whether a file was written.
pub fn keep(dir: &Path, png: &[u8], slim: bool, known: &[String]) -> std::io::Result<bool> {
    let digest = library::digest(png);
    if known.contains(&digest) {
        return Ok(false);
    }
    let target = dir.join(file_name(&digest, slim));
    if target.exists() {
        return Ok(false);
    }
    std::fs::create_dir_all(dir)?;
    let partial = target.with_extension("png.partial");
    std::fs::write(&partial, png)?;
    std::fs::rename(&partial, &target)?;
    Ok(true)
}

/// Reads the archive, newest first. A skin is named by the day it was kept, and its model comes from its file name.
pub fn scan(dir: &Path) -> Vec<SavedSkin> {
    let mut kept: Vec<(SystemTime, SavedSkin)> = library::png_files(dir)
        .into_iter()
        .filter_map(|path| {
            let png = std::fs::read(&path).ok()?;
            let kept_at = std::fs::metadata(&path).ok()?.modified().ok()?;
            let slim = path.file_stem()?.to_string_lossy().ends_with("-slim");
            let name = day(kept_at);
            Some((kept_at, SavedSkin::new(name, path, png, Some(slim))))
        })
        .collect();
    kept.sort_by_key(|(kept_at, _)| std::cmp::Reverse(*kept_at));
    kept.into_iter().map(|(_, skin)| skin).collect()
}

/// The day a skin was kept, as the archive shows it, such as `2 Nov 2026`.
fn day(kept_at: SystemTime) -> String {
    chrono::DateTime::<chrono::Local>::from(kept_at).format("%-d %b %Y").to_string()
}

#[cfg(test)]
#[path = "../../../../tests/app/pages/skins/archive.rs"]
mod tests;
