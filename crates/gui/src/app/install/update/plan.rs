//! The plan of an in-place update: what the release being installed does to each file, set against what the instance holds.
//! Each file either release ships is classified once, and each classification carries the decision the update takes unless the
//! user chooses otherwise. The rules are pure: the caller supplies what is on disk and a way to hash a file, and the planner
//! hashes a file only when a decision depends on it.

use std::collections::{BTreeMap, BTreeSet};

use crate::app::install::shipped::{user_file, user_folder};

/// What an update does to one path, against what the instance holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    /// The release ships a file the instance does not have.
    Add,
    /// The release still ships a file the user deleted.
    Restore,
    /// The release changed a file the user did not, so the release's copy replaces it.
    Update,
    /// The release no longer ships a file the instance holds.
    Drop,
    /// The release ships a jar of a project the instance has under another file name. The release's jar replaces the one named
    /// here.
    Supersede { removes: String },
    /// The release and the user each changed the file. Text is merged hunk by hunk, and a binary file is taken whole.
    Conflict,
    /// The user changed a file the release did not change, so the user's copy stays.
    Mine,
    /// A jar the user added, which no release ships. It stays.
    Yours,
}

/// What the update does with a change: take the release's copy or the removal, or keep what the instance holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// Keep what the instance holds. For a conflict, the user's side.
    Keep,
    /// Take the release's copy, or the removal. For a conflict, the release's side.
    Take,
}

/// One planned change to one path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    /// The path the change is about, relative to the instance folder.
    pub path: String,
    pub kind: Kind,
}

impl Change {
    /// The decision that applies unless the user chooses otherwise. Mods are taken as the release has them, so a dropped mod is
    /// removed. A dropped config file is kept, since the user may still rely on it.
    pub(crate) fn default_decision(&self) -> Decision {
        match &self.kind {
            Kind::Add | Kind::Restore | Kind::Update | Kind::Supersede { .. } => Decision::Take,
            Kind::Drop if is_mod(&self.path) => Decision::Take,
            Kind::Drop | Kind::Conflict | Kind::Mine | Kind::Yours => Decision::Keep,
        }
    }
}

/// What the planner reads about the instance and the two releases.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct Input {
    /// The files the installed release shipped, each with the hash it recorded (`None` when it recorded none). Empty when the
    /// installed release is not known.
    pub installed: BTreeMap<String, Option<String>>,
    /// The files the release being installed ships, each with the hash it gives (`None` when it gives none).
    pub release: BTreeMap<String, Option<String>>,
    /// The paths the instance holds among those the planner may touch: the files of either release, and every file in `mods/`.
    pub held: BTreeSet<String>,
    /// The platform project each jar in `mods/` came from, as `platform/project_id`, by path.
    pub jar_projects: BTreeMap<String, String>,
    /// The platform project each jar the release ships in `mods/` comes from, as `platform/project_id`, by path.
    pub release_projects: BTreeMap<String, String>,
}

/// Whether `path` is a mod: a file in the instance's `mods` folder, whether enabled or disabled.
pub(crate) fn is_mod(path: &str) -> bool {
    path.starts_with("mods/")
}

/// Plans the update: one change per path that needs one, in path order. `hash` gives the SHA-1 of the instance's file at a
/// path, or `None` when it cannot be read; the planner calls it only for a file whose decision depends on it.
pub(crate) fn plan(input: &Input, hash: &mut dyn FnMut(&str) -> Option<String>) -> Vec<Change> {
    let pairs = supersedes(input);
    let removed: BTreeSet<&str> = pairs.iter().map(|(old, _)| old.as_str()).collect();
    let replacing: BTreeSet<&str> = pairs.iter().map(|(_, new)| new.as_str()).collect();
    let mut changes: Vec<Change> = pairs
        .iter()
        .map(|(old, new)| Change {
            path: new.clone(),
            kind: Kind::Supersede { removes: old.clone() },
        })
        .collect();

    let paths: BTreeSet<&str> = input.installed.keys().chain(input.release.keys()).map(String::as_str).collect();
    for path in paths {
        if removed.contains(path) || replacing.contains(path) {
            continue;
        }
        changes.extend(classify(input, path, hash).map(|kind| Change {
            path: path.to_string(),
            kind,
        }));
    }

    for path in input.held.iter().filter(|path| is_mod(path)) {
        let known = input.installed.contains_key(path) || input.release.contains_key(path);
        if !known && !removed.contains(path.as_str()) {
            changes.push(Change {
                path: path.clone(),
                kind: Kind::Yours,
            });
        }
    }
    changes.sort_by(|a, b| a.path.cmp(&b.path));
    changes
}

/// The jars the release replaces: each held jar of a project the release ships under another file name, paired with the
/// release's jar. A jar the release itself ships is never replaced.
fn supersedes(input: &Input) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    let mut replaced: BTreeSet<&str> = BTreeSet::new();
    for (new, project) in &input.release_projects {
        if !is_mod(new) || !input.release.contains_key(new) {
            continue;
        }
        for (old, held_project) in &input.jar_projects {
            let is_other = old != new && !input.release.contains_key(old) && input.held.contains(old);
            if is_other && held_project == project && replaced.insert(old.as_str()) {
                pairs.push((old.clone(), new.clone()));
            }
        }
    }
    pairs.sort();
    pairs
}

/// What the update does to one path the two releases ship, or `None` when nothing is to be done.
fn classify(input: &Input, path: &str, hash: &mut dyn FnMut(&str) -> Option<String>) -> Option<Kind> {
    let held = input.held.contains(path);
    if user_folder(path) || user_file(path) {
        // The user's own folders and files: a shipped file is only added where it is missing.
        return (!held && input.release.contains_key(path)).then_some(Kind::Add);
    }
    if is_mod(path) && !held && input.held.contains(&format!("{path}.disabled")) {
        // A jar the user disabled stays disabled, rather than coming back enabled.
        return None;
    }
    let installed = input.installed.get(path);
    let release = input.release.get(path);
    match (installed, release) {
        (Some(old), Some(new)) => {
            if !held {
                return Some(Kind::Restore);
            }
            if old.is_some() && old == new {
                // The release does not change the file. A config the user changed stays theirs; a jar is left as it is.
                if is_mod(path) {
                    return None;
                }
                let current = hash(path);
                return (current.is_some() && current != *old).then_some(Kind::Mine);
            }
            if new.is_none() && is_mod(path) {
                // A jar the release gives no hash for is taken as the same jar it was, when it is there.
                return None;
            }
            let current = hash(path);
            if old.is_some() && current == *old {
                Some(Kind::Update)
            } else if new.is_some() && current == *new {
                None
            } else {
                Some(Kind::Conflict)
            }
        }
        (None, Some(new)) => {
            if !held {
                return Some(Kind::Add);
            }
            if is_mod(path) && new.is_none() {
                return None;
            }
            let current = hash(path);
            (!(new.is_some() && current == *new)).then_some(Kind::Conflict)
        }
        (Some(_), None) => held.then_some(Kind::Drop),
        (None, None) => None,
    }
}

#[cfg(test)]
#[path = "../../../../tests/app/install/update/plan.rs"]
mod tests;
