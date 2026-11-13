//! What each mod jar declares, kept so the Mods tab does not read every jar again. Reading a jar opens its archive several
//! times (its metadata, its dependencies, its manifest and its icon), which made a large mods folder slow to list.
//!
//! A jar is known by its folder, its name without `.disabled`, its size and its modification time. Switching a mod on or
//! off renames it and keeps both, so it keeps its entry, while any change to its bytes reads it again. Entries live in
//! memory for the session and in the `cache_mod_index` table of the launcher database across starts, with each icon as a
//! PNG blob scaled to twice the tile it fills, so a list never decodes a full-size picture while it draws.
//!
//! Every function here may wait on the database, so it runs on tokio's blocking pool, where the scans run. Without a
//! database (before it opens, or outside the launcher's runtime) the index works from memory alone.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex, OnceLock, PoisonError};

use iced::widget::image;
use lodestone_loaders::ModDependency;
use tracing::warn;

use crate::app::database::{Database, ModIndexKey, ModIndexRow};

/// The side, in pixels, an icon is kept at: twice the Mods tab's 44px tile, so it stays sharp on a 2x display.
const ICON_SIDE: u32 = 88;

/// How a jar is known: its name without `.disabled`, its size and its modification time in nanoseconds.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
}

/// A jar just read: what it declares, and its scaled icon as PNG bytes for the database.
pub(super) struct Read {
    pub known: Known,
    pub icon_png: Option<Vec<u8>>,
}

/// One mods folder's entries, and whether its rows have been read this session.
#[derive(Default)]
struct Folder {
    loaded: bool,
    entries: HashMap<JarKey, Arc<Known>>,
}

static INDEX: LazyLock<Mutex<HashMap<PathBuf, Folder>>> = LazyLock::new(Default::default);

/// The launcher database, once it has opened.
static DATABASE: OnceLock<Database> = OnceLock::new();

/// Gives the index the launcher database, so entries are kept across starts. The root calls it once the database opens.
pub fn attach(db: Database) {
    let _ = DATABASE.set(db);
}

/// Runs a database call from the blocking pool and waits for it. `None` without a database or a runtime to run it on.
fn with_database<T>(call: impl AsyncFnOnce(&Database) -> Result<T, String>) -> Option<T> {
    let db = DATABASE.get()?;
    let Ok(runtime) = tokio::runtime::Handle::try_current() else {
        // The database is open but this thread is outside the runtime, so nothing is kept. Said once, so a caller that
        // forgets to enter the runtime shows in the log rather than as an empty table.
        static TOLD: std::sync::Once = std::sync::Once::new();
        TOLD.call_once(|| warn!("The mod index ran outside the tokio runtime, so it keeps its entries in memory only"));
        return None;
    };
    match runtime.block_on(call(db)) {
        Ok(value) => Some(value),
        Err(error) => {
            warn!("The mod index could not reach the database: {error}");
            None
        }
    }
}

/// The entries known for `dir`, read from the database the first time a session asks.
pub(super) fn entries(dir: &Path) -> HashMap<JarKey, Arc<Known>> {
    if let Some(folder) = INDEX
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(dir)
        .filter(|folder| folder.loaded)
    {
        return folder.entries.clone();
    }
    let key = folder_key(dir);
    let Some(rows) = with_database(async |db| db.mod_index(&key).await) else {
        // Without the database the folder is read as new, and its rows are read when it is open next time.
        return INDEX
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(dir)
            .map(|folder| folder.entries.clone())
            .unwrap_or_default();
    };
    let loaded: HashMap<JarKey, Arc<Known>> = rows.into_iter().map(entry_of).collect();
    let mut index = INDEX.lock().unwrap_or_else(PoisonError::into_inner);
    let folder = index.entry(dir.to_path_buf()).or_default();
    if !folder.loaded {
        folder.loaded = true;
        // A scan that finished while the rows were read keeps what it found.
        for (key, known) in loaded {
            folder.entries.entry(key).or_insert(known);
        }
    }
    folder.entries.clone()
}

/// Makes `entries` what `dir` holds. The jars just read are written to the database and the rows of jars that are gone
/// removed, so a scan writes only what changed.
pub(super) fn store(dir: &Path, entries: HashMap<JarKey, Arc<Known>>, added: Vec<(JarKey, Option<Vec<u8>>)>, removed: Vec<JarKey>) {
    if !added.is_empty() || !removed.is_empty() {
        let key = folder_key(dir);
        let rows: Vec<ModIndexRow> = added
            .into_iter()
            .filter_map(|(jar, icon)| {
                let known = entries.get(&jar)?;
                Some(ModIndexRow {
                    stem: jar.stem,
                    size: jar.size,
                    modified: jar.modified,
                    name: known.name.clone(),
                    author: known.author.clone(),
                    version: known.version.clone(),
                    id: known.id.clone(),
                    dependencies: known.dependencies.clone(),
                    icon,
                })
            })
            .collect();
        let gone: Vec<ModIndexKey> = removed
            .into_iter()
            .map(|jar| ModIndexKey {
                stem: jar.stem,
                size: jar.size,
                modified: jar.modified,
            })
            .collect();
        with_database(async |db| db.update_mod_index(&key, &rows, &gone).await);
    }
    let mut index = INDEX.lock().unwrap_or_else(PoisonError::into_inner);
    let loaded = index.get(dir).is_some_and(|folder| folder.loaded);
    index.insert(dir.to_path_buf(), Folder { loaded, entries });
}

/// Forgets the entries of the mods folder `dir` in memory, as when its instance is deleted. The root removes its rows.
pub fn forget(dir: &Path) {
    INDEX.lock().unwrap_or_else(PoisonError::into_inner).remove(dir);
}

/// Forgets every entry in memory, so every mods folder is read again. Clear cache empties the table beside it.
pub fn clear() {
    INDEX.lock().unwrap_or_else(PoisonError::into_inner).clear();
}

/// The text a mods folder is stored under.
pub fn folder_key(dir: &Path) -> String {
    dir.to_string_lossy().into_owned()
}

/// Reads what the jar at `path` declares. Its icon is decoded and scaled here, off the UI thread.
pub(super) fn read(loader: &lodestone_loaders::Loader<lodestone_loaders::Fabric>, path: &Path) -> Read {
    let info = loader.extract_mod_info(path).ok();
    let icon_png = lodestone_loaders::mod_icon(path).and_then(|bytes| scaled_png(&bytes));
    let icon = icon_png.as_deref().and_then(handle_from_png);
    let known = match info {
        Some(info) => Known {
            name: info.name.filter(|name| !name.trim().is_empty()),
            author: info.authors.into_iter().next(),
            id: Some(info.id).filter(|id| !id.trim().is_empty()),
            version: info.version,
            dependencies: info.dependencies,
            icon,
        },
        None => Known {
            name: None,
            author: None,
            version: None,
            id: None,
            dependencies: Vec::new(),
            icon,
        },
    };
    Read { known, icon_png }
}

/// The entry a database row holds. Its icon is decoded here, so drawing never decodes it.
fn entry_of(row: ModIndexRow) -> (JarKey, Arc<Known>) {
    let known = Known {
        name: row.name,
        author: row.author,
        version: row.version,
        id: row.id,
        dependencies: row.dependencies,
        icon: row.icon.as_deref().and_then(handle_from_png),
    };
    let key = JarKey {
        stem: row.stem,
        size: row.size,
        modified: row.modified,
    };
    (key, Arc::new(known))
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
