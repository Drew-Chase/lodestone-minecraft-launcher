//! The skin files in a folder. The saved skins the player keeps there, and the archive beside them, are read off the UI
//! thread, and what the panels show of each one (its size, its colour and its digest) is worked out once, here.

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use iced::widget::image::Handle;
use sha1::{Digest, Sha1};

use super::viewer::skin_texels;

/// One skin file: its picture, and what the panels show of it.
#[derive(Clone)]
pub struct SavedSkin {
    /// The file's name without its extension, which the page shows.
    pub name: String,
    /// The file the skin is kept in.
    pub path: PathBuf,
    /// The PNG's bytes, which an upload sends.
    pub png: Vec<u8>,
    /// The skin's texels for the 3D previews, decoded once here. `None` when the bytes are not a skin the viewer can draw.
    pub texels: Option<Arc<Vec<u8>>>,
    /// The picture the grid shows.
    pub handle: Handle,
    /// The PNG's size in pixels, when the bytes are a PNG.
    pub size: Option<(u32, u32)>,
    /// The most common colour of the skin, as `#rrggbb`.
    pub accent: Option<String>,
    /// The SHA-1 of the PNG's bytes, in hex. Two files with the same bytes have the same digest.
    pub digest: String,
    /// The model an archived skin was worn in, read from its file name. `None` for a saved skin, whose model the page's
    /// switch picks.
    pub slim: Option<bool>,
}

impl SavedSkin {
    /// A skin from its file's bytes, with its size, colour and digest worked out.
    pub fn new(name: String, path: PathBuf, png: Vec<u8>, slim: Option<bool>) -> Self {
        Self {
            name,
            path,
            size: dimensions(&png),
            accent: accent_hex(&png),
            digest: digest(&png),
            handle: Handle::from_bytes(png.clone()),
            texels: skin_texels(&png).map(Arc::new),
            png,
            slim,
        }
    }
}

impl fmt::Debug for SavedSkin {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The bytes are left out: a skin is a few kilobytes of pixels that nobody reads in a log.
        formatter
            .debug_struct("SavedSkin")
            .field("name", &self.name)
            .field("path", &self.path)
            .finish()
    }
}

/// Reads the PNG skins in `dir`, sorted by name. A folder that cannot be read has no skins.
pub fn scan(dir: &Path) -> Vec<SavedSkin> {
    let mut skins: Vec<SavedSkin> = png_files(dir)
        .into_iter()
        .filter_map(|path| {
            let png = std::fs::read(&path).ok()?;
            let name = path.file_stem()?.to_string_lossy().into_owned();
            Some(SavedSkin::new(name, path, png, None))
        })
        .collect();
    skins.sort_by(|a, b| a.name.cmp(&b.name));
    skins
}

/// The PNG files directly in `dir`. A folder that cannot be read has none.
pub fn png_files(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("png")))
        .collect()
}

/// The size of a PNG, read from its header. `None` when the bytes are not a PNG.
pub fn dimensions(png: &[u8]) -> Option<(u32, u32)> {
    image::ImageReader::new(std::io::Cursor::new(png))
        .with_guessed_format()
        .ok()?
        .into_dimensions()
        .ok()
}

/// The most common colour among the skin's visible pixels, as `#rrggbb`, with each channel rounded to four bits. Near-black
/// pixels are skipped, so a dark outline does not stand for the skin.
pub fn accent_hex(png: &[u8]) -> Option<String> {
    let image = image::load_from_memory(png).ok()?.to_rgba8();
    let mut counts = [0u32; 4096];
    for pixel in image.pixels() {
        let [red, green, blue, alpha] = pixel.0;
        if alpha < 128 || u32::from(red) + u32::from(green) + u32::from(blue) < 90 {
            continue;
        }
        let bucket = usize::from(red >> 4) << 8 | usize::from(green >> 4) << 4 | usize::from(blue >> 4);
        counts[bucket] += 1;
    }
    let (bucket, count) = counts.iter().enumerate().max_by_key(|(_, count)| **count)?;
    if *count == 0 {
        return None;
    }
    let channel = |shift: usize| (((bucket >> shift) & 0xF) as u8) * 17;
    Some(format!("#{:02x}{:02x}{:02x}", channel(8), channel(4), channel(0)))
}

/// The SHA-1 of `bytes`, in hex.
pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha1::digest(bytes))
}

#[cfg(test)]
#[path = "../../../../tests/app/pages/skins/library.rs"]
mod tests;
