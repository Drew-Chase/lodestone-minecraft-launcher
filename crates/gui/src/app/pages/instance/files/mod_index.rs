//! What each mod jar declares, kept so the Mods tab does not read every jar again. Reading a jar opens its archive several
//! times (its metadata, its dependencies, its manifest and its icon), which made a large mods folder slow to list.
//!
//! A jar is known by its folder, its name without `.disabled`, its size and its modification time. Switching a mod on or
//! off renames it and keeps both, so it keeps its entry, while any change to its bytes reads it again. Entries live in
//! memory for the session and in `mod-index/` in the cache folder across starts, one file per mods folder. Icons are kept
//! scaled to twice the tile they fill, so a list never decodes a full-size picture while it draws.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex, PoisonError};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use iced::widget::image;
use lodestone_loaders::ModDependency;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use tracing::warn;

/// The side, in pixels, an icon is kept at: twice the Mods tab's 44px tile, so it stays sharp on a 2x display.
const ICON_SIDE: u32 = 88;
/// The version of the file format. A file of another version is read as empty, so its folder is read again.
const FORMAT: u32 = 1;

/// How a jar is known: its name without `.disabled`, its size and its modification time in nanoseconds.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(super) struct JarKey {
    pub stem: String,
    pub size: u64,
    pub modified: u64,
}

/// What a jar declares.
#[derive(Debug, Clone)]
pub(super) struct Known {
    /// The name the mod declares, when it declares one that is not blank.
    pub name: Option<String>,
    pub author: Option<String>,
    pub version: Option<String>,
    pub id: Option<String>,
    pub dependencies: Vec<ModDependency>,
    /// The icon, scaled, ready to draw. The same handle is shared by every scan, so the picture is uploaded once.
    pub icon: Option<image::Handle>,
    /// The scaled icon as PNG bytes, for the file on disk.
    icon_png: Option<Vec<u8>>,
}

/// One mods folder's entries, and whether its file has been read this session.
#[derive(Default)]
struct Folder {
    loaded: bool,
    entries: HashMap<JarKey, Arc<Known>>,
}

static INDEX: LazyLock<Mutex<HashMap<PathBuf, Folder>>> = LazyLock::new(Default::default);

/// The entries known for `dir`, read from its file the first time a session asks.
pub(super) fn entries(dir: &Path) -> HashMap<JarKey, Arc<Known>> {
    if let Some(folder) = INDEX
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(dir)
        .filter(|folder| folder.loaded)
    {
        return folder.entries.clone();
    }
    let entries = load(dir);
    let mut index = INDEX.lock().unwrap_or_else(PoisonError::into_inner);
    let folder = index.entry(dir.to_path_buf()).or_default();
    if !folder.loaded {
        folder.loaded = true;
        // A scan that finished while the file was read keeps what it found.
        for (key, known) in entries {
            folder.entries.entry(key).or_insert(known);
        }
    }
    folder.entries.clone()
}

/// Makes `entries` what `dir` holds, in memory and on disk. The file is written only when `changed`.
pub(super) fn store(dir: &Path, entries: HashMap<JarKey, Arc<Known>>, changed: bool) {
    if changed && let Err(error) = save(dir, &entries) {
        warn!("Could not keep the mod index of {}: {error}", dir.display());
    }
    let mut index = INDEX.lock().unwrap_or_else(PoisonError::into_inner);
    index.insert(dir.to_path_buf(), Folder { loaded: true, entries });
}

/// Forgets every entry, in memory and on disk, so every mods folder is read again.
pub fn clear() -> std::io::Result<()> {
    INDEX.lock().unwrap_or_else(PoisonError::into_inner).clear();
    match root() {
        Some(root) => match std::fs::remove_dir_all(root) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(error),
            _ => Ok(()),
        },
        None => Ok(()),
    }
}

/// Reads what the jar at `path` declares. Its icon is decoded and scaled here, off the UI thread.
pub(super) fn read(loader: &lodestone_loaders::Loader<lodestone_loaders::Fabric>, path: &Path) -> Known {
    let info = loader.extract_mod_info(path).ok();
    let icon_png = lodestone_loaders::mod_icon(path).and_then(|bytes| scaled_png(&bytes));
    let icon = icon_png.as_deref().and_then(handle_from_png);
    match info {
        Some(info) => Known {
            name: info.name.filter(|name| !name.trim().is_empty()),
            author: info.authors.into_iter().next(),
            id: Some(info.id).filter(|id| !id.trim().is_empty()),
            version: info.version,
            dependencies: info.dependencies,
            icon,
            icon_png,
        },
        None => Known {
            name: None,
            author: None,
            version: None,
            id: None,
            dependencies: Vec::new(),
            icon,
            icon_png,
        },
    }
}

/// The folder the index files live in.
fn root() -> Option<PathBuf> {
    Some(crate::app::paths::cache_dir()?.join("mod-index"))
}

/// The index file of `dir`, named by the SHA-1 of its path, so any folder name fits.
fn file_of(dir: &Path) -> Option<PathBuf> {
    let digest = Sha1::digest(dir.to_string_lossy().as_bytes());
    Some(root()?.join(format!("{digest:x}.json")))
}

/// The file on disk.
#[derive(Serialize, Deserialize)]
struct IndexFile {
    format: u32,
    entries: Vec<StoredEntry>,
}

#[derive(Serialize, Deserialize)]
struct StoredEntry {
    key: JarKey,
    name: Option<String>,
    author: Option<String>,
    version: Option<String>,
    id: Option<String>,
    dependencies: Vec<ModDependency>,
    /// The scaled icon as base64 PNG.
    icon: Option<String>,
}

/// The entries in `dir`'s file. A missing, unreadable or older file gives none.
fn load(dir: &Path) -> HashMap<JarKey, Arc<Known>> {
    let Some(text) = file_of(dir).and_then(|path| std::fs::read(path).ok()) else {
        return HashMap::new();
    };
    let file: IndexFile = match serde_json::from_slice(&text) {
        Ok(file) => file,
        Err(error) => {
            warn!("The mod index of {} does not read, so the folder is read again: {error}", dir.display());
            return HashMap::new();
        }
    };
    if file.format != FORMAT {
        return HashMap::new();
    }
    file.entries
        .into_iter()
        .map(|entry| {
            let icon_png = entry.icon.and_then(|icon| STANDARD.decode(icon).ok());
            let icon = icon_png.as_deref().and_then(handle_from_png);
            let known = Known {
                name: entry.name,
                author: entry.author,
                version: entry.version,
                id: entry.id,
                dependencies: entry.dependencies,
                icon,
                icon_png,
            };
            (entry.key, Arc::new(known))
        })
        .collect()
}

fn save(dir: &Path, entries: &HashMap<JarKey, Arc<Known>>) -> Result<(), String> {
    let Some(path) = file_of(dir) else {
        return Ok(());
    };
    let file = IndexFile {
        format: FORMAT,
        entries: entries
            .iter()
            .map(|(key, known)| StoredEntry {
                key: key.clone(),
                name: known.name.clone(),
                author: known.author.clone(),
                version: known.version.clone(),
                id: known.id.clone(),
                dependencies: known.dependencies.clone(),
                icon: known.icon_png.as_deref().map(|png| STANDARD.encode(png)),
            })
            .collect(),
    };
    let json = serde_json::to_vec(&file).map_err(|error| error.to_string())?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    lodestone_fs::write_atomic(&path, &json).map_err(|error| error.to_string())
}

/// The icon `bytes` decode to, scaled to fit [`ICON_SIDE`], as PNG. A picture already that small is kept as it is.
fn scaled_png(bytes: &[u8]) -> Option<Vec<u8>> {
    let picture = ::image::load_from_memory(bytes).ok()?;
    let picture = if picture.width() > ICON_SIDE || picture.height() > ICON_SIDE {
        picture.thumbnail(ICON_SIDE, ICON_SIDE)
    } else {
        picture
    };
    let mut png = Vec::new();
    picture
        .to_rgba8()
        .write_to(&mut std::io::Cursor::new(&mut png), ::image::ImageFormat::Png)
        .ok()?;
    Some(png)
}

/// A drawable handle of a small PNG, decoded here so drawing never decodes it.
fn handle_from_png(png: &[u8]) -> Option<image::Handle> {
    let picture = ::image::load_from_memory_with_format(png, ::image::ImageFormat::Png).ok()?.to_rgba8();
    let (width, height) = picture.dimensions();
    Some(image::Handle::from_rgba(width, height, picture.into_raw()))
}
