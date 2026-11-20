//! What a rollback would change: each platform mod a snapshot holds, set against the mods the instance holds now.

use std::collections::HashMap;

use crate::model::{Platform, Snapshot};

/// The instance a snapshot would be put back on: its Minecraft version and its loader's key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub minecraft: String,
    pub loader: String,
}

/// A platform mod the instance holds, read from its record and the jar it is installed as.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Installed {
    pub platform: Platform,
    pub project_id: String,
    pub version_id: String,
    /// The release's version number, when its record has one.
    pub version: Option<String>,
    /// The jar's file name in `mods/`, `.disabled` included.
    pub file_name: String,
}

/// Why a snapshot cannot be put back on the instance at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// The snapshot was taken for another Minecraft version.
    Minecraft { snapshot: String, instance: String },
    /// The snapshot was taken for another loader.
    Loader { snapshot: String, instance: String },
}

/// How a platform mod of a snapshot stands against the instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// The instance holds the snapshot's release already.
    Same,
    /// The instance holds another release of the mod, which a rollback replaces.
    Changed,
    /// The instance holds no record of the mod, which a rollback installs.
    NotInstalled,
}

/// A platform mod of a snapshot, against the instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub platform: Platform,
    pub project_id: String,
    pub name: String,
    /// The release's version number in the snapshot, when it recorded one.
    pub wanted_version: Option<String>,
    /// The release's id in the snapshot, which a rollback installs.
    pub wanted_id: String,
    /// The installed record the mod matches, when the instance holds one.
    pub installed: Option<Installed>,
    pub state: State,
}

/// How a local jar of a snapshot stands against `mods/`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalState {
    /// A jar of that name is in `mods/` with the bytes the snapshot recorded.
    Same,
    /// No jar of that name is in `mods/`, so a rollback copies it back from the snapshot's zip.
    Missing,
    /// A jar of that name is in `mods/` with other bytes, which a rollback leaves alone.
    Differs,
}

/// A local jar of a snapshot, against `mods/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalItem {
    pub file: String,
    /// The SHA-1 the snapshot recorded for the jar.
    pub sha1: String,
    pub state: LocalState,
}

/// What a rollback would do with a snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// The platform mods, Modrinth first, in the order the snapshot lists them.
    pub items: Vec<Item>,
    /// The local jars, in the order the snapshot lists them.
    pub local: Vec<LocalItem>,
}

impl Plan {
    /// How many platform mods a rollback would change: the ones that are not already at the snapshot's release.
    pub fn changes(&self) -> usize {
        self.items.iter().filter(|item| item.state != State::Same).count()
    }
}

/// Plans the rollback of `snapshot` onto `target`. `installed` holds the instance's platform mods, and `on_disk` maps each
/// jar in `mods/` to its SHA-1, for the snapshot's local jars.
pub fn plan(snapshot: &Snapshot, target: &Target, installed: &[Installed], on_disk: &HashMap<String, String>) -> Result<Plan, Refusal> {
    if snapshot.minecraft != target.minecraft {
        return Err(Refusal::Minecraft {
            snapshot: snapshot.minecraft.clone(),
            instance: target.minecraft.clone(),
        });
    }
    if snapshot.loader != target.loader {
        return Err(Refusal::Loader {
            snapshot: snapshot.loader.clone(),
            instance: target.loader.clone(),
        });
    }
    let items = snapshot
        .linked()
        .map(|(platform, entry)| {
            let found = installed
                .iter()
                .find(|record| record.platform == platform && record.project_id == entry.modid)
                .cloned();
            let state = match &found {
                Some(record) if record.version_id == entry.fileid => State::Same,
                Some(_) => State::Changed,
                None => State::NotInstalled,
            };
            Item {
                platform,
                project_id: entry.modid.clone(),
                name: entry.name.clone(),
                wanted_version: entry.version.clone(),
                wanted_id: entry.fileid.clone(),
                installed: found,
                state,
            }
        })
        .collect();
    let local = snapshot
        .local
        .iter()
        .map(|jar| {
            let state = match on_disk.get(&jar.file) {
                None => LocalState::Missing,
                Some(sha1) if sha1.eq_ignore_ascii_case(&jar.sha1) => LocalState::Same,
                Some(_) => LocalState::Differs,
            };
            LocalItem {
                file: jar.file.clone(),
                sha1: jar.sha1.clone(),
                state,
            }
        })
        .collect();
    Ok(Plan { items, local })
}

#[cfg(test)]
#[path = "../tests/plan.rs"]
mod tests;
