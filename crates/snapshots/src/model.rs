//! What a snapshot holds, as its JSON writes it, and the limit on how many are kept.

use serde::{Deserialize, Serialize};

/// The fewest snapshots an instance keeps. Zero turns snapshots off.
pub const LIMIT_MIN: u32 = 0;
/// The most snapshots an instance keeps.
pub const LIMIT_MAX: u32 = 20;
/// How many snapshots an instance keeps when nothing else says.
pub const LIMIT_DEFAULT: u32 = 5;

/// `value`, held within [`LIMIT_MIN`] and [`LIMIT_MAX`].
pub fn clamp_limit(value: u32) -> u32 {
    value.clamp(LIMIT_MIN, LIMIT_MAX)
}

/// The platforms a snapshot records mods from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Platform {
    Modrinth,
    CurseForge,
}

impl Platform {
    /// Both platforms, in the order the JSON lists them.
    pub const ALL: [Platform; 2] = [Platform::Modrinth, Platform::CurseForge];

    /// The platform's key in the JSON and in the mod records: `modrinth` or `curseforge`.
    pub fn key(self) -> &'static str {
        match self {
            Platform::Modrinth => "modrinth",
            Platform::CurseForge => "curseforge",
        }
    }

    /// The platform a key names, if it names one.
    pub fn from_key(key: &str) -> Option<Self> {
        Platform::ALL.into_iter().find(|platform| platform.key() == key)
    }
}

/// One platform mod a snapshot holds: the project, and the release the instance had it at.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    /// The project's id on its platform: a Modrinth project id, or a CurseForge project number.
    pub modid: String,
    /// The release's version number, or `None` when no number was recorded for it.
    pub version: Option<String>,
    /// The release's id on its platform: a Modrinth version id, or a CurseForge file number.
    pub fileid: String,
    /// The project's name, so a mod that is no longer installed can still be named.
    pub name: String,
}

/// A jar with no platform record. Its bytes are kept in the snapshot's zip.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalJar {
    /// The jar's file name in `mods/`, `.disabled` included.
    pub file: String,
    /// The SHA-1 of the jar, as lowercase hex.
    pub sha1: String,
}

/// A snapshot as its JSON holds it. A section that is missing reads as empty, and keys this reader does not know are
/// ignored, so a later version can add to the file.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Snapshot {
    /// The Minecraft version the instance ran when the snapshot was taken.
    #[serde(default)]
    pub minecraft: String,
    /// The loader the instance ran, by its database key, such as `fabric`.
    #[serde(default)]
    pub loader: String,
    /// The Modrinth mods the snapshot holds.
    #[serde(default)]
    pub modrinth: Vec<Entry>,
    /// The CurseForge mods the snapshot holds.
    #[serde(default)]
    pub curseforge: Vec<Entry>,
    /// The jars with no platform record, whose bytes are in the snapshot's zip.
    #[serde(default)]
    pub local: Vec<LocalJar>,
}

impl Snapshot {
    /// The entries of `platform`.
    pub fn entries(&self, platform: Platform) -> &[Entry] {
        match platform {
            Platform::Modrinth => &self.modrinth,
            Platform::CurseForge => &self.curseforge,
        }
    }

    /// Every platform entry, Modrinth first, each with the platform it came from.
    pub fn linked(&self) -> impl Iterator<Item = (Platform, &Entry)> + '_ {
        Platform::ALL
            .into_iter()
            .flat_map(move |platform| self.entries(platform).iter().map(move |entry| (platform, entry)))
    }

    /// How many platform mods the snapshot holds.
    pub fn linked_count(&self) -> usize {
        self.modrinth.len() + self.curseforge.len()
    }
}

/// Why a snapshot was taken. Its file name carries it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    /// Taken by Update all, before the mods it was about to replace were replaced.
    UpdateAll,
    /// Taken by hand from the Mods tab, and holding every mod.
    Manual,
    /// Taken before a rollback changed the mods, so that rollback can be undone.
    Rollback,
}

impl Kind {
    /// Every kind.
    pub const ALL: [Kind; 3] = [Kind::UpdateAll, Kind::Manual, Kind::Rollback];

    /// The kind's name in a file name: `update-all`, `manual` or `rollback`.
    pub fn slug(self) -> &'static str {
        match self {
            Kind::UpdateAll => "update-all",
            Kind::Manual => "manual",
            Kind::Rollback => "rollback",
        }
    }
}

#[cfg(test)]
#[path = "../tests/model.rs"]
mod tests;
