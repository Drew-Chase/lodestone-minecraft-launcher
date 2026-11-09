//! The shared folders: the content the Discover page downloads. Each kind of
//! content has a folder under `<root>/shared`, outside every instance, so any
//! instance can import from it. The mods the page adds to an instance are kept
//! in that instance's own `mods/` folder, and recorded as [`ModSource`].

use std::path::{Path, PathBuf};

use lodestone_platforms::ContentType;

use crate::app::paths::LauncherPaths;

/// The name of the folder a kind of content is kept in, under `shared/`.
pub fn folder_name(content: ContentType) -> &'static str {
    match content {
        ContentType::Modpack => "modpacks",
        ContentType::ResourcePack => "resourcepacks",
        ContentType::Shader => "shaderpacks",
        ContentType::DataPack => "datapacks",
        ContentType::World => "worlds",
        ContentType::Mod => "mods",
    }
}

/// The shared folder for `content`.
pub fn folder(paths: &LauncherPaths, content: ContentType) -> PathBuf {
    paths.shared().join(folder_name(content))
}

/// The folder one download is written into, named for its platform, project and version so that two
/// downloads never share a name. The ids keep only file-name characters.
pub fn download_folder(platform: &str, project_id: &str, version_id: &str) -> String {
    format!("{platform}-{}-{}", file_safe(project_id), file_safe(version_id))
}

/// `id` with every character a file name may not hold replaced by `_`.
fn file_safe(id: &str) -> String {
    id.chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect()
}

/// One download the Discover page made into a shared folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedRecord {
    /// The shared folder's name, such as `resourcepacks`.
    pub kind: String,
    /// The platform's key, such as `modrinth`.
    pub platform: String,
    /// The project's id on that platform.
    pub project_id: String,
    /// The version that was downloaded.
    pub version_id: String,
    /// The project's name, as it was shown when downloaded.
    pub name: String,
    /// Where the download is, relative to its kind's folder.
    pub path: String,
    /// The size in bytes, when the platform gave one.
    pub size: Option<u64>,
}

/// A mod in an instance's `mods/` folder that came from a platform: the project and release it was installed as.
/// The Discover page, a modpack install and the Mods tab all keep these.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModSource {
    /// The instance's id.
    pub instance: String,
    /// The platform's key, such as `modrinth`.
    pub platform: String,
    /// The project's id on that platform.
    pub project_id: String,
    /// The release that was installed.
    pub version_id: String,
    /// The project's name, or the jar's name when the platform's is not known.
    pub name: String,
    /// The jar's file name in the instance's `mods/` folder, `.disabled` included.
    pub file_name: String,
    /// The SHA-1 of the jar as it was installed, when the platform gave one.
    pub sha1: Option<String>,
}

/// A file an installed modpack release shipped outside the user's own folders: a mod, a config file or a script. The
/// instance keeps these so an update can tell what the pack placed from what the user added.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackFile {
    /// The instance's id.
    pub instance: String,
    /// The file's path relative to the instance folder, with `/` separators, such as `mods/sodium.jar`.
    pub path: String,
    /// The SHA-1 the release gives the file, or the SHA-1 read from disk when it gives none, as lowercase hex.
    pub sha1: Option<String>,
}

/// The SHA-1 of a jar in an instance's `mods/` folder, with the size and modification time it had when hashed. The
/// hash stands while the jar keeps both.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JarHash {
    /// The jar's file name in the instance's `mods/` folder.
    pub file_name: String,
    /// The jar's size in bytes when it was hashed.
    pub size: u64,
    /// The jar's modification time, in seconds since the Unix epoch, when it was hashed.
    pub modified_at: i64,
    /// The SHA-1 of the jar, as lowercase hex.
    pub sha1: String,
}

/// Content the Discover page added to an instance, or to one of its worlds, or imported as a world. It is kept so the
/// page still shows the content as added after a restart.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddedContent {
    /// The instance's id.
    pub instance: String,
    /// Empty for the instance's own folders. For a world's datapacks, the world's folder name.
    pub scope: String,
    /// The shared folder's name for the kind of content, such as `resourcepacks`.
    pub kind: String,
    /// The platform's key, such as `modrinth`.
    pub platform: String,
    /// The project's id on that platform.
    pub project_id: String,
    /// The release that was added.
    pub version_id: String,
    /// The project's name, as it was shown when added.
    pub name: String,
    /// The file or folder written into the destination, such as `FreshAnimations.zip`.
    pub file_name: String,
}

/// One thing in a shared folder that can be added to an instance: a file or folder kept loose in the shared folder, or a
/// file a download wrote into its own folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedEntry {
    /// The file or folder's name, which is also its name where it is added.
    pub name: String,
    /// Where the entry is on disk.
    pub path: PathBuf,
    /// Whether the entry is a folder.
    pub is_dir: bool,
    /// The download folder the entry is in, when it is in one. A loose entry has none.
    pub download: Option<String>,
}

/// What a shared folder holds that can be added to an instance, sorted by name. A loose file or folder is an entry of
/// its own. Each file in a folder is an entry, since a folder in a shared folder is one download's. A folder that cannot
/// be read lists nothing.
pub fn entries(folder: &Path) -> Vec<SharedEntry> {
    let Ok(children) = std::fs::read_dir(folder) else {
        return Vec::new();
    };
    let mut entries = Vec::new();
    for child in children.filter_map(Result::ok) {
        let path = child.path();
        let name = child.file_name().to_string_lossy().into_owned();
        if !path.is_dir() {
            entries.push(SharedEntry {
                name,
                path,
                is_dir: false,
                download: None,
            });
            continue;
        }
        let Ok(files) = std::fs::read_dir(&path) else {
            continue;
        };
        for file in files.filter_map(Result::ok) {
            let file_path = file.path();
            entries.push(SharedEntry {
                name: file.file_name().to_string_lossy().into_owned(),
                is_dir: file_path.is_dir(),
                path: file_path,
                download: Some(name.clone()),
            });
        }
    }
    entries.sort_by_key(|entry| entry.name.to_lowercase());
    entries
}

/// The path that deleting an entry takes away. A file in a download goes with the rest of its download, so the download's
/// folder is the path. A loose file is its own path.
pub fn delete_target(entry: &SharedEntry) -> PathBuf {
    match (&entry.download, entry.path.parent()) {
        (Some(_), Some(download)) => download.to_path_buf(),
        _ => entry.path.clone(),
    }
}

/// The name a shared world goes by: its file's name without the extension, as the Worlds tab lists it.
pub fn world_name(entry: &SharedEntry) -> String {
    Path::new(&entry.name)
        .file_stem()
        .map_or_else(|| entry.name.clone(), |stem| stem.to_string_lossy().into_owned())
}

/// Copies `from`, a file or a folder, into the folder `into` under its own name, and returns that name. A name that is
/// already in `into` is left as it is, so a pack already there is never overwritten.
pub fn copy_into(from: &Path, into: &Path) -> Result<String, String> {
    let name = from
        .file_name()
        .ok_or_else(|| "the shared entry has no name".to_string())?
        .to_string_lossy()
        .into_owned();
    std::fs::create_dir_all(into).map_err(|error| error.to_string())?;
    let target = into.join(&name);
    if target.exists() {
        return Ok(name);
    }
    if from.is_dir() {
        copy_dir(from, &target)?;
    } else {
        std::fs::copy(from, &target).map_err(|error| error.to_string())?;
    }
    Ok(name)
}

/// Copies the folder `from` into the new folder `to`, file by file.
fn copy_dir(from: &Path, to: &Path) -> Result<(), String> {
    std::fs::create_dir_all(to).map_err(|error| error.to_string())?;
    for child in std::fs::read_dir(from).map_err(|error| error.to_string())? {
        let child = child.map_err(|error| error.to_string())?;
        let source = child.path();
        let target = to.join(child.file_name());
        if source.is_dir() {
            copy_dir(&source, &target)?;
        } else {
            std::fs::copy(&source, &target).map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "../../tests/app/shared_library.rs"]
mod tests;
