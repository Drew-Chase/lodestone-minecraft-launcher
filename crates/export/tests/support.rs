//! Scratch folders, files and fixtures for the tests that read and write disk.

use std::fs;
use std::path::{Path, PathBuf};

use crate::format::Metadata;
use crate::hash::digest_file;
use crate::link::{CurseForgeLink, Entry, Links, ModrinthLink};

/// A fresh, empty folder under the system temp directory, named for the test that asked for it.
pub fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lodestone-export-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("the scratch folder can be made");
    dir
}

/// Writes `contents` to `relative` under `root`, making the folders it needs, and returns the file's path.
pub fn put(root: &Path, relative: &str, contents: &[u8]) -> PathBuf {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("a file has a parent")).expect("the folders can be made");
    fs::write(&path, contents).expect("the file can be written");
    path
}

/// An entry for a file already on disk. `digested` gives it digests, as the mod and resource files get.
pub fn entry(path: &str, source: PathBuf, digested: bool, links: Links) -> Entry {
    let size = fs::metadata(&source).expect("the file exists").len();
    let digests = digested.then(|| digest_file(&source).expect("the file hashes"));
    Entry {
        path: path.to_string(),
        source,
        size,
        digests,
        links,
    }
}

/// The metadata of a Fabric instance on Minecraft 1.21.1.
pub fn metadata() -> Metadata {
    Metadata {
        name: "Enhanced Worlds".to_string(),
        version: "1.0.0".to_string(),
        description: "A test pack".to_string(),
        minecraft: "1.21.1".to_string(),
        loader: "fabric".to_string(),
        loader_version: Some("0.16.10".to_string()),
        locked: false,
    }
}

/// A Modrinth link to `project`'s release `version`, downloaded from `download`.
pub fn modrinth(project: &str, version: &str, download: &str) -> Links {
    Links {
        modrinth: Some(ModrinthLink {
            project: project.to_string(),
            version: version.to_string(),
            page: format!("https://modrinth.com/project/{project}"),
            download_url: download.to_string(),
        }),
        curseforge: None,
    }
}

/// A CurseForge link to a project and a file, by their numeric ids.
pub fn curseforge(project: &str, file: &str) -> Links {
    Links {
        modrinth: None,
        curseforge: Some(CurseForgeLink {
            project: project.to_string(),
            file: file.to_string(),
        }),
    }
}
