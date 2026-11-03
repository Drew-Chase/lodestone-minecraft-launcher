//! The news feed's copy on disk, so the page shows the last feed at once, while the next one is fetched, and works offline.
//! Both documents are kept as the server sent them. The time the cache was written is the feed's time.

use std::path::PathBuf;
use std::time::SystemTime;

/// The file the news feed is kept in, inside the cache folder.
const NEWS_FILE: &str = "news.json";
/// The file the Java patch notes are kept in, inside the cache folder.
const PATCHES_FILE: &str = "javaPatchNotes.json";

/// The folder the feed is cached in: `<cache dir>/lodestone/news`.
fn folder() -> Option<PathBuf> {
    dirs::cache_dir().map(|dir| dir.join("lodestone").join("news"))
}

/// The cached documents, and the time they were written.
pub struct Cached {
    pub news: Vec<u8>,
    pub patches: Vec<u8>,
    pub written: SystemTime,
}

/// Reads the cached feed, when both of its documents are there.
pub fn read() -> Option<Cached> {
    let folder = folder()?;
    let news = std::fs::read(folder.join(NEWS_FILE)).ok()?;
    let patches = std::fs::read(folder.join(PATCHES_FILE)).ok()?;
    let written = std::fs::metadata(folder.join(NEWS_FILE)).ok()?.modified().ok()?;
    Some(Cached { news, patches, written })
}

/// Replaces the cached feed. A cache that cannot be written only costs another download, so a failure is not reported.
pub fn write(news: &[u8], patches: &[u8]) {
    let Some(folder) = folder() else {
        return;
    };
    if std::fs::create_dir_all(&folder).is_err() {
        return;
    }
    let _ = std::fs::write(folder.join(NEWS_FILE), news);
    let _ = std::fs::write(folder.join(PATCHES_FILE), patches);
}
