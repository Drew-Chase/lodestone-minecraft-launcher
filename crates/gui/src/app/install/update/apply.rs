//! Planning an in-place update against what the instance holds, and applying the plan. Planning reads the instance and the staged
//! release, and changes nothing. Applying writes the journal before its first move, so the rollback knows every path to undo, then
//! moves each original it replaces or removes into the workspace, and writes the release's copies in their place.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use super::journal::{Journal, Workspace, move_file};
use super::merge::{self, Side};
use super::plan::{self, Change, Decision, Input, Kind, is_mod};
use super::{Decisions, UpdateRequest};
use crate::app::database::Database;
use crate::app::install::shipped::is_text;
use crate::app::mod_sources::sha1_of_file;
use crate::app::shared_library::PackFile;

/// The plan of an update: each change with the decision it takes, and every file the release ships.
pub(super) struct Planned {
    /// The changes, in path order, each with its decision.
    pub changes: Vec<(Change, Decision)>,
    /// Every file the release ships, with the SHA-1 it gives or the one read from its staged copy. `None` where neither is known.
    pub release: BTreeMap<String, Option<String>>,
}

/// What applying the plan left in the instance.
pub(super) struct Applied {
    /// The files the release ships, as the instance records them once the update commits.
    pub files: Vec<PackFile>,
    /// The file names of the jars the update removed from `mods`.
    pub removed_jars: Vec<String>,
    /// How many jars `mods` holds once the update is applied.
    pub mod_count: u32,
}

/// Works out what the update changes against the instance's files, and what each change takes by the user's decisions.
pub(super) async fn plan_release(request: &UpdateRequest, db: &Database, ws: &Workspace) -> Result<Planned, String> {
    let id = request.instance.id.as_str();
    let installed: BTreeMap<String, Option<String>> = db.pack_files_of(id).await?.into_iter().map(|file| (file.path, file.sha1)).collect();
    let jar_projects: BTreeMap<String, String> = db
        .mod_sources_of(id)
        .await?
        .into_iter()
        .map(|record| (format!("mods/{}", record.file_name), format!("{}/{}", record.platform, record.project_id)))
        .collect();
    let release_projects: BTreeMap<String, String> = request
        .payload
        .sources
        .iter()
        .map(|source| (format!("mods/{}", source.file_name), format!("{}/{}", source.platform, source.project_id)))
        .collect();
    let shipped: BTreeMap<String, Option<String>> = request
        .payload
        .files
        .iter()
        .map(|file| (file.path.clone(), file.sha1.as_deref().map(str::to_ascii_lowercase)))
        .collect();
    let instance = ws.instance().to_path_buf();
    let staging = ws.staging();
    let decisions = request.decisions.clone();
    tokio::task::spawn_blocking(move || plan_blocking(installed, jar_projects, release_projects, shipped, &instance, &staging, &decisions))
        .await
        .map_err(|error| error.to_string())?
}

/// The planning itself, on the blocking pool: it reads the staged release and the files the instance holds, and hashes a file only
/// where a decision depends on it.
fn plan_blocking(
    installed: BTreeMap<String, Option<String>>,
    jar_projects: BTreeMap<String, String>,
    release_projects: BTreeMap<String, String>,
    shipped: BTreeMap<String, Option<String>>,
    instance: &Path,
    staging: &Path,
    decisions: &Decisions,
) -> Result<Planned, String> {
    let staged = staged_hashes(staging);
    let release = release_of(&shipped, &staged);

    let mut held: BTreeSet<String> = BTreeSet::new();
    for path in installed.keys().chain(release.keys()) {
        if instance.join(path).is_file() {
            held.insert(path.clone());
        }
    }
    for name in file_names(&instance.join("mods")) {
        held.insert(format!("mods/{name}"));
    }

    let input = Input {
        installed,
        release: release.clone(),
        held,
        jar_projects,
        release_projects,
    };
    let mut cache: BTreeMap<String, Option<String>> = BTreeMap::new();
    let mut hash = |path: &str| -> Option<String> {
        if let Some(known) = cache.get(path) {
            return known.clone();
        }
        let sha1 = sha1_of_file(&instance.join(path)).ok();
        cache.insert(path.to_string(), sha1.clone());
        sha1
    };
    let changes = plan::plan(&input, &mut hash)
        .into_iter()
        .map(|change| {
            let decision = decisions.decide(&change);
            (change, decision)
        })
        .collect();
    Ok(Planned { changes, release })
}

/// What applying the plan does to the instance, worked out before anything is moved.
struct Operations {
    /// The paths, relative to the instance, whose originals move into the workspace.
    moved: Vec<String>,
    /// The paths the update creates, which the instance does not hold yet.
    created: Vec<String>,
    /// The paths the update writes, and what it writes there.
    writes: Vec<(String, Write)>,
}

/// What a write puts in place of a file.
enum Write {
    /// The copy the release staged at the same path.
    Staged,
    /// Text computed from the user's file, the pristine copy and the release's.
    Merged(Vec<u8>),
}

/// Applies the plan under a journal, and returns what the instance holds afterwards. The journal names every path before the first
/// move, so a failure or a crash after that point is undone by the rollback.
pub(super) async fn apply(request: &UpdateRequest, ws: &Workspace, planned: Planned, token: &str) -> Result<Applied, String> {
    let instance = ws.instance().to_path_buf();
    let staging = ws.staging();
    let moved_into = ws.moved();
    let old_base = request.paths.pack_base(&request.instance.id);
    let id = request.instance.id.clone();
    let token = token.to_string();
    let workspace = ws.clone();
    let decisions = request.decisions.clone();
    tokio::task::spawn_blocking(move || {
        let ops = operations(&planned.changes, &instance, &staging, &old_base, &decisions)?;
        let journal = Journal {
            token,
            moved: ops.moved.clone(),
            created: ops.created.clone(),
        };
        workspace.write_journal(&journal)?;
        for path in &ops.moved {
            move_file(&instance.join(path), &moved_into.join(path))?;
        }
        for (path, write) in &ops.writes {
            match write {
                Write::Staged => move_file(&staging.join(path), &instance.join(path))?,
                Write::Merged(bytes) => lodestone_fs::write_atomic(&instance.join(path), bytes).map_err(|error| error.to_string())?,
            }
        }
        Ok(record(&id, &planned.release, &ops, &instance))
    })
    .await
    .map_err(|error| error.to_string())?
}

/// Works out the moves and writes that carry out the plan. A decision that keeps the instance's files changes nothing.
fn operations(changes: &[(Change, Decision)], instance: &Path, staging: &Path, old_base: &Path, decisions: &Decisions) -> Result<Operations, String> {
    let mut moved: BTreeSet<String> = BTreeSet::new();
    let mut writes: Vec<(String, Write)> = Vec::new();
    for (change, decision) in changes {
        let path = change.path.clone();
        match (&change.kind, decision) {
            (Kind::Add | Kind::Restore | Kind::Update | Kind::Mine | Kind::Conflict, Decision::Take) => writes.push((path, Write::Staged)),
            (Kind::Supersede { removes }, Decision::Take) => {
                if instance.join(removes).is_file() {
                    moved.insert(removes.clone());
                }
                writes.push((path, Write::Staged));
            }
            (Kind::Drop, Decision::Take) => {
                if instance.join(&path).is_file() {
                    moved.insert(path);
                }
            }
            (Kind::Conflict, Decision::Keep) => {
                let choose = |index: usize| decisions.side(&path, index);
                if let Some(bytes) = merged(&instance.join(&path), &old_base.join(&path), &staging.join(&path), choose)? {
                    writes.push((path, Write::Merged(bytes)));
                }
            }
            _ => {}
        }
    }
    // A write over a file the instance holds moves that file aside first, so the rollback can put it back.
    for (path, _) in &writes {
        if instance.join(path).is_file() {
            moved.insert(path.clone());
        }
    }
    let created: Vec<String> = writes.iter().map(|(path, _)| path.clone()).filter(|path| !moved.contains(path)).collect();
    Ok(Operations {
        moved: moved.into_iter().collect(),
        created,
        writes,
    })
}

/// The text to write where a text file changes on both sides: the release's changes that do not touch the user's lines are merged in,
/// and each conflict takes the side `choose` gives it. The pristine copy is the base of the merge, and without one the two files are
/// compared, so every difference is a conflict. `None` when nothing is to be written: a binary file, or a result that is the user's file
/// as it stands.
fn merged(yours: &Path, base: &Path, theirs: &Path, choose: impl Fn(usize) -> Side) -> Result<Option<Vec<u8>>, String> {
    let (Ok(yours), Ok(theirs)) = (std::fs::read(yours), std::fs::read(theirs)) else {
        return Ok(None);
    };
    let base = std::fs::read(base).ok().filter(|bytes| is_text(bytes));
    if !is_text(&yours) || !is_text(&theirs) {
        return Ok(None);
    }
    let (Ok(yours), Ok(theirs)) = (std::str::from_utf8(&yours), std::str::from_utf8(&theirs)) else {
        return Ok(None);
    };
    let pieces = match base.as_deref() {
        Some(base) => match std::str::from_utf8(base) {
            Ok(base) => merge::merge(base, yours, theirs),
            Err(_) => return Ok(None),
        },
        None => merge::compare(yours, theirs),
    };
    let text = merge::render(&pieces, choose);
    Ok((text != yours).then(|| text.into_bytes()))
}

/// What the instance holds once the operations are done: the files the release ships, the jars removed, and the jar count.
fn record(id: &str, release: &BTreeMap<String, Option<String>>, operations: &Operations, instance: &Path) -> Applied {
    let written: BTreeSet<&str> = operations.writes.iter().map(|(path, _)| path.as_str()).collect();
    let removed_jars = operations
        .moved
        .iter()
        .filter(|path| is_mod(path) && !written.contains(path.as_str()))
        .filter_map(|path| path.rsplit('/').next().map(str::to_string))
        .collect();
    Applied {
        files: files_of(id, release, &written, instance),
        removed_jars,
        mod_count: mod_count_of(instance),
    }
}

/// The files the release ships, as the instance records them. A jar the update wrote has the hash of what it wrote, when the release
/// gave none.
pub(super) fn files_of(id: &str, release: &BTreeMap<String, Option<String>>, written: &BTreeSet<&str>, instance: &Path) -> Vec<PackFile> {
    release
        .iter()
        .map(|(path, sha1)| {
            let sha1 = sha1.clone().or_else(|| {
                (is_mod(path) && written.contains(path.as_str()))
                    .then(|| sha1_of_file(&instance.join(path)).ok())
                    .flatten()
            });
            PackFile {
                instance: id.to_string(),
                path: path.clone(),
                sha1,
            }
        })
        .collect()
}

/// How many jars the instance's `mods` folder holds.
pub(super) fn mod_count_of(instance: &Path) -> u32 {
    file_names(&instance.join("mods")).iter().filter(|name| name.ends_with(".jar")).count() as u32
}

/// Every file the release ships: the files its manifest lists, with the SHA-1 the manifest gives, and the files its archives unpack,
/// which take the SHA-1 of their staged copies. A file the manifest lists takes the hash of its staged copy, unless it is a mod: a
/// mod's copy is not staged, so it keeps the hash its manifest gives.
pub(super) fn release_of(manifest: &BTreeMap<String, Option<String>>, staged: &BTreeMap<String, String>) -> BTreeMap<String, Option<String>> {
    let mut release = BTreeMap::new();
    for (path, listed) in manifest {
        let hash = match staged.get(path) {
            Some(sha1) if !is_mod(path) => Some(sha1.clone()),
            _ => listed.clone(),
        };
        release.insert(path.clone(), hash);
    }
    for (path, sha1) in staged {
        release.entry(path.clone()).or_insert_with(|| Some(sha1.clone()));
    }
    release
}

/// The SHA-1 of each file staged under `staging`, by its path relative to it.
fn staged_hashes(staging: &Path) -> BTreeMap<String, String> {
    walk(staging)
        .into_iter()
        .filter_map(|path| sha1_of_file(&staging.join(&path)).ok().map(|sha1| (path, sha1)))
        .collect()
}

/// What a record-only update leaves: the release's files as the staged copies show them, and the instance's jars as they stand. No
/// file in the instance changes, so the plan has no changes.
pub(super) async fn record_release(request: &UpdateRequest, ws: &Workspace) -> Result<(Planned, Applied), String> {
    let shipped: BTreeMap<String, Option<String>> = request
        .payload
        .files
        .iter()
        .map(|file| (file.path.clone(), file.sha1.as_deref().map(str::to_ascii_lowercase)))
        .collect();
    let instance = ws.instance().to_path_buf();
    let staging = ws.staging();
    let id = request.instance.id.clone();
    tokio::task::spawn_blocking(move || {
        let release = release_of(&shipped, &staged_hashes(&staging));
        let applied = Applied {
            files: files_of(&id, &release, &BTreeSet::new(), &instance),
            removed_jars: Vec::new(),
            mod_count: mod_count_of(&instance),
        };
        (
            Planned {
                changes: Vec::new(),
                release,
            },
            applied,
        )
    })
    .await
    .map_err(|error| error.to_string())
}

/// The names of the files in `dir`, not its folders. A folder that is not there has none.
pub(super) fn file_names(dir: &Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .flatten()
                .filter(|entry| entry.path().is_file())
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default()
}

/// Every file under `root`, as its path relative to `root` with `/` separators. The folder the archives are fetched into is left out.
pub(super) fn walk(root: &Path) -> Vec<String> {
    fn visit(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                visit(&path, out);
            } else if path.is_file() {
                out.push(path);
            }
        }
    }
    let mut found = Vec::new();
    visit(root, &mut found);
    found
        .iter()
        .filter_map(|path| path.strip_prefix(root).ok())
        .map(|relative| relative.to_string_lossy().replace('\\', "/"))
        .filter(|relative| !relative.starts_with(".modpack/"))
        .collect()
}

#[cfg(test)]
#[path = "../../../../tests/app/install/update/apply.rs"]
mod tests;
