//! The resource packs and shader packs in an instance's folders, each read for the name and size it shows. A pack is a zip
//! file or a folder; any other file in the folder is not a pack and is left out.

use std::path::Path;

/// One resource pack or shader pack in an instance's folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackFile {
    /// The file or folder name on disk, with its extension.
    pub file_name: String,
    /// The name the pack is shown by: its file name without `.zip`, or the folder's name.
    pub name: String,
    /// The size of the zip in bytes. `None` for a folder.
    pub size: Option<u64>,
    /// Whether the pack is a folder.
    pub is_dir: bool,
}

/// The packs in `dir`, sorted by name. A folder that cannot be read holds none.
pub fn scan(dir: &Path) -> Vec<PackFile> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut packs: Vec<PackFile> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let file_name = entry.file_name().to_string_lossy().into_owned();
            let kind = entry.file_type().ok()?;
            if kind.is_dir() {
                return Some(PackFile {
                    name: file_name.clone(),
                    file_name,
                    size: None,
                    is_dir: true,
                });
            }
            if !kind.is_file() || !is_zip(&file_name) {
                return None;
            }
            let name = Path::new(&file_name).file_stem()?.to_string_lossy().into_owned();
            Some(PackFile {
                size: entry.metadata().ok().map(|metadata| metadata.len()),
                file_name,
                name,
                is_dir: false,
            })
        })
        .collect();
    packs.sort_by_key(|pack| pack.name.to_lowercase());
    packs
}

/// The name a pack is shown by, given its file name: the name without `.zip`, or the name of a folder as it is. The lists
/// show a shared pack and an installed one by this name, so both read alike.
pub fn name_of(file_name: &str) -> String {
    if is_zip(file_name)
        && let Some(stem) = Path::new(file_name).file_stem()
    {
        return stem.to_string_lossy().into_owned();
    }
    file_name.to_string()
}

/// Whether `file_name` names a zip file, whatever the case of its extension.
fn is_zip(file_name: &str) -> bool {
    Path::new(file_name)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"))
}

#[cfg(test)]
#[path = "../../../../../tests/app/pages/instance/files/packs.rs"]
mod tests;
