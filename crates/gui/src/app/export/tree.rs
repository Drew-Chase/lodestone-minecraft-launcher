//! The folder listing the export dialog shows: the entries of one folder of the instance, sorted, with the
//! listings read so far kept by folder. Listings are read on the blocking pool, one folder at a time, so a
//! folder opened again shows at once.

use std::collections::HashMap;
use std::path::PathBuf;

use crate::app::folder_listing::{self, Entries};
pub use crate::app::folder_listing::{Column, Row, Sort, sorted};

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
        self.sort = self.sort.by(column);
    }
}

/// Reads the entries of `folder` inside the instance at `root`, on the blocking pool. Leftovers and symbolic
/// links are left out, the way the export leaves them out.
pub async fn read_folder(root: PathBuf, folder: String) -> Result<Vec<Row>, String> {
    folder_listing::read_folder(root, folder, Entries::Exportable).await
}

#[cfg(test)]
#[path = "../../../tests/app/export/tree.rs"]
mod tests;
