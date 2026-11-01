//! The folder listing the export dialog shows: the entries of one folder of the instance, sorted, with the
//! listings read so far kept by folder. Listings are read on the blocking pool, one folder at a time, so a
//! folder opened again shows at once.

use std::collections::HashMap;
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
    pub is_folder: bool,
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

/// The folder on screen, and the listings read so far.
#[derive(Debug, Default)]
pub struct Tree {
    /// The folder on screen, relative to the instance. Empty is the instance folder itself.
    pub folder: String,
    pub sort: Sort,
    listings: HashMap<String, Vec<Row>>,
    errors: HashMap<String, String>,
}

impl Tree {
    /// Makes `folder` the folder on screen. Returns whether its listing still has to be read: a folder that has
    /// not been read, or whose last read failed, is read again when it is opened.
    pub fn open(&mut self, folder: String) -> bool {
        let needed = !self.listings.contains_key(&folder);
        self.folder = folder;
        needed
    }

    /// Stores the outcome of reading `folder`.
    pub fn store(&mut self, folder: String, result: Result<Vec<Row>, String>) {
        match result {
            Ok(rows) => {
                self.errors.remove(&folder);
                self.listings.insert(folder, rows);
            }
            Err(error) => {
                self.errors.insert(folder, error);
            }
        }
    }

    /// The rows of `folder`, as read, when its listing has landed.
    pub fn rows_of(&self, folder: &str) -> Option<&[Row]> {
        self.listings.get(folder).map(Vec::as_slice)
    }

    /// The reason reading `folder` failed, when it did.
    pub fn error_of(&self, folder: &str) -> Option<&str> {
        self.errors.get(folder).map(String::as_str)
    }

    /// The paths of the immediate children of `folder`, as read. A folder that has not been read has none.
    pub fn children(&self, folder: &str) -> Vec<String> {
        self.rows_of(folder).unwrap_or_default().iter().map(|row| row.path.clone()).collect()
    }

    /// The rows on screen, sorted by the sort the tree holds. `None` until the folder's listing lands.
    pub fn sorted_rows(&self) -> Option<Vec<&Row>> {
        self.rows_of(&self.folder).map(|rows| sorted(rows, self.sort))
    }

    /// The paths of the rows on screen, in the order they are drawn.
    pub fn screen_paths(&self) -> Vec<String> {
        self.sorted_rows()
            .map(|rows| rows.iter().map(|row| row.path.clone()).collect())
            .unwrap_or_default()
    }

    /// Sorts by `column`. Choosing the column that is already the sort flips its direction.
    pub fn sort_by(&mut self, column: Column) {
        self.sort = if self.sort.column == column {
            Sort {
                column,
                descending: !self.sort.descending,
            }
        } else {
            Sort { column, descending: false }
        };
    }
}

/// The rows sorted by `sort`, with folders first. Names break ties, so the order is stable.
pub fn sorted(rows: &[Row], sort: Sort) -> Vec<&Row> {
    let mut sorted: Vec<&Row> = rows.iter().collect();
    sorted.sort_by(|a, b| {
        let column = match sort.column {
            Column::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            Column::Size => a.size.cmp(&b.size),
            Column::Modified => a.modified.cmp(&b.modified),
        };
        let column = if sort.descending { column.reverse() } else { column };
        b.is_folder.cmp(&a.is_folder).then(column).then_with(|| a.name.cmp(&b.name))
    });
    sorted
}

/// Reads the entries of `folder` inside the instance at `root`, on the blocking pool. Leftovers and symbolic
/// links are left out, the way the export leaves them out.
pub async fn read_folder(root: PathBuf, folder: String) -> Result<Vec<Row>, String> {
    tokio::task::spawn_blocking(move || read_rows(&root, &folder))
        .await
        .map_err(|error| error.to_string())?
}

fn read_rows(root: &Path, folder: &str) -> Result<Vec<Row>, String> {
    let directory = if folder.is_empty() { root.to_path_buf() } else { root.join(folder) };
    let entries = std::fs::read_dir(&directory).map_err(|error| error.to_string())?;
    let mut rows = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| error.to_string())?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if is_leftover(&name) {
            continue;
        }
        let file_type = entry.file_type().map_err(|error| error.to_string())?;
        if file_type.is_symlink() || !(file_type.is_dir() || file_type.is_file()) {
            continue;
        }
        let metadata = entry.metadata().map_err(|error| error.to_string())?;
        let path = if folder.is_empty() { name.clone() } else { format!("{folder}/{name}") };
        rows.push(Row {
            name,
            path,
            is_folder: file_type.is_dir(),
            size: if file_type.is_dir() { 0 } else { metadata.len() },
            modified: metadata.modified().ok().map(DateTime::<Local>::from),
        });
    }
    Ok(rows)
}

#[cfg(test)]
#[path = "../../../tests/app/export/tree.rs"]
mod tests;
