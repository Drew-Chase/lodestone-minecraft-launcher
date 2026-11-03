//! The imports the Import Instance dialog remembers. A row is written when an import is queued, so the dialog can list the
//! latest ones and start any of them again from its source.

use chrono::{DateTime, Local};

/// The format an import read its source as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportFormat {
    /// A Modrinth pack, `.mrpack`.
    Mrpack,
    /// A CurseForge pack, a zip whose manifest is `manifest.json`.
    CurseForge,
    /// A Lodestone package, `.lodestone`.
    Lodestone,
}

impl ImportFormat {
    /// The name the format is stored under.
    pub fn to_db(self) -> &'static str {
        match self {
            Self::Mrpack => "mrpack",
            Self::CurseForge => "curseforge",
            Self::Lodestone => "lodestone",
        }
    }

    /// The format stored under `name`, when this launcher wrote it.
    pub fn from_db(name: &str) -> Option<Self> {
        match name {
            "mrpack" => Some(Self::Mrpack),
            "curseforge" => Some(Self::CurseForge),
            "lodestone" => Some(Self::Lodestone),
            _ => None,
        }
    }
}

/// Where an import's source is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportOrigin {
    /// A file on disk.
    File,
    /// A CurseForge page, which the import fetches by its url.
    Url,
}

impl ImportOrigin {
    /// The name the origin is stored under.
    pub fn to_db(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Url => "url",
        }
    }

    /// The origin stored under `name`, when this launcher wrote it.
    pub fn from_db(name: &str) -> Option<Self> {
        match name {
            "file" => Some(Self::File),
            "url" => Some(Self::Url),
            _ => None,
        }
    }
}

/// One import, as the dialog lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportRecord {
    /// The modpack's name, which the instance takes.
    pub name: String,
    /// The format the source was read as.
    pub format: ImportFormat,
    /// Whether the source is a file on disk or a page.
    pub origin: ImportOrigin,
    /// The file's path, or the page's url, as the import was started from it.
    pub source: String,
    /// The size of the source in bytes: the file, or the archive a page import downloaded.
    pub bytes: u64,
    /// When the import was queued.
    pub imported_at: DateTime<Local>,
}
