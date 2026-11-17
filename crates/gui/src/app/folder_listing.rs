//! One folder of an instance, read and sorted: the listing the export dialog and the Files tab both show.
//!
//! A folder is read on the blocking pool. The export leaves out what it never packs (leftovers and links), and the
//! Files tab lists everything the folder holds.

use std::cmp::Ordering;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Local};
use lodestone_export::plan::is_leftover;

/// One entry of a folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// The entry's name.
    pub name: String,
    /// The entry's path inside the instance, with `/` separators.
    pub path: String,
    /// Whether the entry is a folder, or a link to one.
    pub is_folder: bool,
    /// Whether the entry is a symbolic link.
    pub is_link: bool,
    /// The file's size in bytes. Folders show none.
    pub size: u64,
    /// When the entry was last changed, when the file system says.
    pub modified: Option<DateTime<Local>>,
}

/// The columns a listing sorts by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Column {
    Name,
    Size,
    Modified,
}

/// How a listing is sorted. Folders always come first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sort {
    pub column: Column,
    pub descending: bool,
}

impl Default for Sort {
    fn default() -> Self {
        Self {
            column: Column::Name,
            descending: false,
        }
    }
}

impl Sort {
    /// The sort after `column` is chosen. Choosing the column that is already the sort flips its direction.
    pub fn by(self, column: Column) -> Self {
        Self {
            column,
            descending: self.column == column && !self.descending,
        }
    }
}

/// Which entries of a folder a listing holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Entries {
    /// The files and folders an export can pack: no leftovers and no links.
    Exportable,
    /// Everything the folder holds, links included.
    All,
}

/// How `a` and `b` order under `sort`, with folders first. Names break ties, so the order is stable.
fn compare(a: &Row, b: &Row, sort: Sort) -> Ordering {
    let column = match sort.column {
        Column::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        Column::Size => a.size.cmp(&b.size),
        Column::Modified => a.modified.cmp(&b.modified),
    };
    let column = if sort.descending { column.reverse() } else { column };
    b.is_folder.cmp(&a.is_folder).then(column).then_with(|| a.name.cmp(&b.name))
}

/// The rows sorted by `sort`, with folders first.
pub fn sorted(rows: &[Row], sort: Sort) -> Vec<&Row> {
    let mut sorted: Vec<&Row> = rows.iter().collect();
    sorted.sort_by(|a, b| compare(a, b, sort));
    sorted
}

/// The indices of `rows` in the order `sort` puts them, with folders first.
pub fn order(rows: &[Row], sort: Sort) -> Vec<usize> {
    let mut order: Vec<usize> = (0..rows.len()).collect();
    order.sort_by(|&a, &b| compare(&rows[a], &rows[b], sort));
    order
}

/// Reads the entries of `folder` inside the instance at `root`, on the blocking pool.
pub async fn read_folder(root: PathBuf, folder: String, entries: Entries) -> Result<Vec<Row>, String> {
    tokio::task::spawn_blocking(move || read_rows(&root, &folder, entries))
        .await
        .map_err(|error| error.to_string())?
}

/// Reads the entries of `folder` inside the instance at `root`. Blocks, so it runs on the blocking pool.
pub fn read_rows(root: &Path, folder: &str, entries: Entries) -> Result<Vec<Row>, String> {
    let directory = if folder.is_empty() { root.to_path_buf() } else { root.join(folder) };
    let listing = std::fs::read_dir(&directory).map_err(|error| error.to_string())?;
    let mut rows = Vec::new();
    for entry in listing {
        let entry = entry.map_err(|error| error.to_string())?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let file_type = entry.file_type().map_err(|error| error.to_string())?;
        let is_link = file_type.is_symlink();
        if entries == Entries::Exportable && (is_leftover(&name) || is_link || !(file_type.is_dir() || file_type.is_file())) {
            continue;
        }
        // A link is listed as what it points at, so a link to a folder opens as one. A link that points nowhere is listed
        // as an empty file.
        let metadata = if is_link {
            std::fs::metadata(entry.path()).ok()
        } else {
            Some(entry.metadata().map_err(|error| error.to_string())?)
        };
        let is_folder = metadata.as_ref().is_some_and(std::fs::Metadata::is_dir);
        let path = if folder.is_empty() { name.clone() } else { format!("{folder}/{name}") };
        rows.push(Row {
            name,
            path,
            is_folder,
            is_link,
            size: metadata.as_ref().filter(|_| !is_folder).map_or(0, std::fs::Metadata::len),
            modified: metadata.and_then(|metadata| metadata.modified().ok()).map(DateTime::<Local>::from),
        });
    }
    Ok(rows)
}

#[cfg(test)]
#[path = "../../tests/app/folder_listing.rs"]
mod tests;
