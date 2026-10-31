//! The shared folders: the content the Discover page downloads. Each kind of
//! content has a folder under `<root>/shared`, outside every instance, so any
//! instance can import from it.

use std::path::PathBuf;

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

#[cfg(test)]
#[path = "../../tests/app/shared_library.rs"]
mod tests;
