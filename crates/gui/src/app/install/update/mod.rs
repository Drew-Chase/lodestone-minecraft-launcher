//! An in-place update of an instance to a release of its modpack. The release's files are staged and planned against the instance's
//! files, the changes are applied under a journal that can undo them, and the instance's records move to the new release in one
//! transaction. Nothing here creates an instance folder: the instance keeps its worlds, screenshots, saves and its own mods.
//!
//! The stages are ordered so that a stop before the journal is written leaves the instance as it was. The apply stage does no network
//! work, and the commit is one transaction. A stop after the journal is written is undone by the rollback, or finished by the next
//! launch when the commit had already happened.

pub(crate) mod apply;
pub(crate) mod journal;
pub(crate) mod merge;
pub(crate) mod plan;
pub(crate) mod review;

use std::collections::BTreeMap;
use std::path::PathBuf;

use chrono::Local;
use sipper::Sender as ProgressSender;
use sipper::{Sipper, sipper};
use tokio::sync::mpsc;
use tracing::warn;

use self::apply::{Planned, apply, plan_release};
use self::journal::{Workspace, settle};
use self::plan::{Change, Decision, Kind, is_mod};
use super::download::{self, Part, PartReport};
use super::{InstallProgress, Stage, Totals, download_stage, drive, ensure, fraction, loader_stage, modpack, new_id, report};
use crate::app::database::Database;
use crate::app::install::modpack::Payload;
use crate::app::install::pack::Release;
use crate::app::install::shipped;
use crate::app::lang;
use crate::app::launch::libraries::Item;
use crate::app::pages::instance::files;
use crate::app::pages::instance::files::world_archive;
use crate::app::pages::library::instance::{Instance, Loader};
use crate::app::paths::LauncherPaths;
use crate::app::shared_library::{AddedContent, ModSource, PackFile};

/// The decisions the user made about the changes an update brings. A change with no decision takes its default, and a conflict
/// hunk with no choice keeps the user's side.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Decisions {
    by_path: BTreeMap<String, Decision>,
    /// The side each hunk of a text conflict takes, by the path and the hunk's place in the file's conflicts.
    hunks: BTreeMap<String, Vec<merge::Side>>,
}

impl Decisions {
    /// The decision for `change`: the one the user made, or else the change's default.
    pub(crate) fn decide(&self, change: &Change) -> Decision {
        self.by_path.get(&change.path).copied().unwrap_or_else(|| change.default_decision())
    }

    /// Records the user's decision for the change to `path`.
    pub(crate) fn set(&mut self, path: impl Into<String>, decision: Decision) {
        self.by_path.insert(path.into(), decision);
    }

    /// The side the conflict `index` of the text file `path` takes. A conflict nobody chose keeps the user's side.
    pub(crate) fn side(&self, path: &str, index: usize) -> merge::Side {
        self.hunks
            .get(path)
            .and_then(|sides| sides.get(index))
            .copied()
            .unwrap_or(merge::Side::Yours)
    }

    /// Records which side the conflict `index` of the text file `path` takes.
    pub(crate) fn choose(&mut self, path: impl Into<String>, index: usize, side: merge::Side) {
        let sides = self.hunks.entry(path.into()).or_default();
        if sides.len() <= index {
            sides.resize(index + 1, merge::Side::Yours);
        }
        sides[index] = side;
    }
}

/// What an in-place update takes: the instance as the library holds it, the release it moves onto, and the decisions the user made
/// about the changes that release brings.
#[derive(Debug, Clone)]
pub struct UpdateRequest {
    /// The launcher's paths, which hold the instance folder and the update's workspace.
    pub paths: LauncherPaths,
    /// The instance as it stands before the update.
    pub instance: Instance,
    /// The release the update moves the instance onto.
    pub release: Release,
    /// The Minecraft version of that release, and its loader and loader version.
    pub mc_version: String,
    pub loader: Loader,
    pub loader_version: Option<String>,
    /// The release's files, archives and platform records.
    pub payload: Payload,
    /// Whether the worlds are backed up before a change of Minecraft version.
    pub backup_worlds: bool,
    /// The decisions the user made. A change with no decision takes its default.
    pub decisions: Decisions,
    /// Whether the update only records the release the instance is on. Such an update changes no file: it stages the release and keeps
    /// its pristine copies, then records the release as the one the instance runs, and it leaves the game and the mods as they are.
    pub record_only: bool,
    /// What the review showed the user, when the update was reviewed. The release is then already staged, and the update runs only if
    /// the plan it makes now lists the same changes, with the same hunks in each text conflict.
    pub(crate) reviewed: Option<Reviewed>,
}

/// The plan a review showed: its changes, and how many hunks each text conflict has, by path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reviewed {
    pub changes: Vec<Change>,
    pub hunks: Vec<(String, usize)>,
}

/// Runs an in-place update of `request.instance` onto `request.release`. The sipper reports progress and resolves to the instance as
/// it now stands, or to the reason the update stopped. A stopped update leaves the instance as it was.
pub(crate) fn update(request: UpdateRequest, db: Database) -> impl Sipper<Result<Instance, String>, InstallProgress> {
    sipper(move |mut progress: ProgressSender<InstallProgress>| async move {
        let ws = workspace_of(&request.paths, &request.instance);
        let mut unwritten = Unwritten { workspace: Some(ws.clone()) };
        ws.create()?;
        let outcome = run(&request, &db, &ws, &mut progress).await;
        unwritten.disarm();
        if outcome.is_err() {
            roll_back(ws).await;
        }
        outcome
    })
}

/// Settles the updates an earlier launch left unfinished. A workspace whose update committed is finished, and any other is rolled
/// back. Returns the instances whose update was rolled back, so the launch can say so.
pub(crate) async fn recover(paths: LauncherPaths, db: Database) -> Vec<String> {
    let Ok(instances) = db.instances().await else {
        return Vec::new();
    };
    let Ok(committed) = db.committed_pack_updates().await else {
        return Vec::new();
    };
    let folders: BTreeMap<String, PathBuf> = instances
        .iter()
        .map(|instance| (instance.id.clone(), paths.instance(&instance.directory_name)))
        .collect();
    tokio::task::spawn_blocking(move || settle(&paths, &folders, &committed))
        .await
        .unwrap_or_default()
}

/// The workspace of an update of `instance`.
fn workspace_of(paths: &LauncherPaths, instance: &Instance) -> Workspace {
    Workspace::new(
        paths.pack_journal(&instance.id),
        paths.instance(&instance.directory_name),
        paths.pack_base(&instance.id),
    )
}

/// Removes the workspace of an update dropped before it writes its journal, since then nothing in the instance has changed. Once the
/// journal exists, the rollback or the next launch settles the workspace, so the guard leaves it alone.
struct Unwritten {
    workspace: Option<Workspace>,
}

impl Unwritten {
    /// Keeps the workspace: the update ended by itself, and settles it.
    fn disarm(&mut self) {
        self.workspace = None;
    }
}

impl Drop for Unwritten {
    fn drop(&mut self) {
        if let Some(workspace) = self.workspace.take() {
            workspace.discard_unwritten();
        }
    }
}

/// Undoes an update that stopped with an error, from its journal. Only a failure reaches here: the journal, if there is one, names
/// everything the update changed.
async fn roll_back(ws: Workspace) {
    match tokio::task::spawn_blocking(move || ws.roll_back()).await {
        Ok(Ok(())) => {}
        Ok(Err(error)) => warn!("could not roll back an update that stopped: {error}"),
        Err(error) => warn!("could not roll back an update that stopped: {error}"),
    }
}

/// The game files of the release's Minecraft version and loader, made ready for the instance to launch on. Its version id and Java
/// runtime are what the instance records.
struct Launch {
    version_id: String,
    java_name: String,
}

/// The stages of an update up to its commit, in order. A record-only update skips the game files, the plan, and the apply.
async fn run(request: &UpdateRequest, db: &Database, ws: &Workspace, progress: &mut ProgressSender<InstallProgress>) -> Result<Instance, String> {
    let client = lodestone_net::client();
    let launch = if request.record_only {
        None
    } else {
        Some(launch_game(request, progress, &client).await?)
    };
    if !request.record_only && request.backup_worlds && request.mc_version != request.instance.mc_version {
        report(progress, Stage::Plan, None, Stage::Plan.label().to_string()).await;
        back_up_worlds(request).await?;
    }
    // A reviewed update staged the release when it was reviewed, and the staged files are still there.
    if request.reviewed.is_none() {
        stage_release(progress, request, ws, &client).await?;
    }
    let token = new_id();
    let applied = if request.record_only {
        let (planned, applied) = apply::record_release(request, ws).await?;
        write_copies(request, ws, &planned.release).await?;
        applied
    } else {
        report(progress, Stage::Plan, None, Stage::Plan.label().to_string()).await;
        let planned = plan_release(request, db, ws).await?;
        if let Some(reviewed) = &request.reviewed {
            let hunks: Vec<(String, usize)> = review::conflicts_of(ws, &planned.changes)
                .await?
                .into_iter()
                .map(|conflict| (conflict.path, conflict.hunks.len()))
                .collect();
            if !same_changes(&reviewed.changes, &planned.changes) || reviewed.hunks != hunks {
                return Err(lang::t("pack_update.changed").to_string());
            }
        }
        fetch_mods(progress, request, ws, &planned, &client).await?;
        write_copies(request, ws, &planned.release).await?;
        report(progress, Stage::Apply, None, Stage::Apply.label().to_string()).await;
        apply(request, ws, planned, &token).await?
    };

    let instance = updated_instance(request, launch.as_ref(), applied.mod_count);
    let (packs, sources): (Vec<AddedContent>, Vec<ModSource>) = if request.record_only {
        (Vec::new(), Vec::new())
    } else {
        (
            request
                .payload
                .packs
                .iter()
                .map(|pack| AddedContent {
                    instance: request.instance.id.clone(),
                    ..pack.clone()
                })
                .collect(),
            request
                .payload
                .sources
                .iter()
                .map(|source| ModSource {
                    instance: request.instance.id.clone(),
                    ..source.clone()
                })
                .collect(),
        )
    };
    db.commit_pack_update(&instance, &applied.files, &applied.removed_jars, &sources, &packs, &token)
        .await?;
    settle_committed(ws).await;
    report(progress, Stage::Finish, None, instance.name.clone()).await;
    Ok(instance)
}

/// Makes the release's game files ready, the way an install does: the Minecraft version, its Java runtime, and the loader.
async fn launch_game(request: &UpdateRequest, progress: &mut ProgressSender<InstallProgress>, client: &reqwest::Client) -> Result<Launch, String> {
    let paths = &request.paths;
    let mc = request.mc_version.as_str();
    report(progress, Stage::Resolve, None, format!("Minecraft {mc}")).await;
    let vanilla = {
        let _version = ensure::lock(format!("version:{mc}")).await;
        ensure::vanilla(paths, client, mc).await?
    };
    let component = vanilla
        .java_version
        .as_ref()
        .map_or_else(|| "jre-legacy".to_string(), |java| java.component.clone());
    let downloaded = download_stage(progress, paths, client, &vanilla, &component).await?;
    let (java_binary, java_name) = downloaded.java;
    let version_id = match request.loader {
        Loader::Vanilla => mc.to_string(),
        loader => {
            loader_stage(
                progress,
                paths,
                mc,
                loader,
                request.loader_version.as_deref(),
                &java_binary,
                vanilla.java_major(),
            )
            .await?
        }
    };
    Ok(Launch {
        version_id,
        java_name: java_name.to_string(),
    })
}

/// Keeps the pristine copies of the release's text files, which replace the instance's copies once the update commits. They are
/// written before the apply moves the staged files, since the apply moves them into the instance.
async fn write_copies(request: &UpdateRequest, ws: &Workspace, release: &BTreeMap<String, Option<String>>) -> Result<(), String> {
    let copies: Vec<PackFile> = release
        .keys()
        .filter(|path| !is_mod(path))
        .map(|path| PackFile {
            instance: request.instance.id.clone(),
            path: path.clone(),
            sha1: None,
        })
        .collect();
    // The folder is made even when no copy is kept, so the commit replaces the old copies rather than keeping them beside the new.
    std::fs::create_dir_all(ws.base_new()).map_err(|error| error.to_string())?;
    shipped::write_base(&ws.base_new(), &ws.staging(), &copies).await
}

/// The instance as the update leaves it: the new release's Minecraft version, loader, launch version, Java runtime, and jar count.
/// Its name, art, settings and playtime stay as they were.
fn updated_instance(request: &UpdateRequest, launch: Option<&Launch>, mod_count: u32) -> Instance {
    let mut instance = request.instance.clone();
    if let Some(launch) = launch {
        instance.mc_version = request.mc_version.clone();
        instance.loader = request.loader;
        instance.loader_version = request.loader_version.clone();
        instance.version_id = Some(launch.version_id.clone());
        instance.java_component = Some(launch.java_name.clone());
        instance.mod_count = mod_count;
    }
    instance.source_version = Some(request.release.number.clone());
    instance.source_version_id = Some(request.release.id.clone());
    instance
}

/// Backs up every world of the instance into its `backups` folder, before an update changes the Minecraft version.
async fn back_up_worlds(request: &UpdateRequest) -> Result<(), String> {
    let folder = request.paths.instance(&request.instance.directory_name);
    tokio::task::spawn_blocking(move || {
        let backups = folder.join("backups");
        for world in files::scan_worlds(&folder) {
            world_archive::back_up(&world.path, &backups, Local::now())?;
        }
        Ok(())
    })
    .await
    .map_err(|error| error.to_string())?
}

/// Stages the release's files that are not mods, and the archives it unpacks, into the workspace. The instance is not touched.
async fn stage_release(
    progress: &mut ProgressSender<InstallProgress>,
    request: &UpdateRequest,
    ws: &Workspace,
    client: &reqwest::Client,
) -> Result<(), String> {
    let mut payload = request.payload.clone();
    payload.files.retain(|file| !is_mod(&file.path));
    modpack::stage(progress, &payload, &ws.staging(), client).await?;
    Ok(())
}

/// Fetches into the workspace the mods the decisions take. Nothing is written into the instance until every fetch has finished.
async fn fetch_mods(
    progress: &mut ProgressSender<InstallProgress>,
    request: &UpdateRequest,
    ws: &Workspace,
    planned: &Planned,
    client: &reqwest::Client,
) -> Result<(), String> {
    let staging = ws.staging();
    let items: Vec<Item> = planned
        .changes
        .iter()
        .filter(|(change, decision)| is_mod(&change.path) && *decision == Decision::Take && writes_from_release(&change.kind))
        .filter_map(|(change, _)| {
            request.payload.files.iter().find(|file| file.path == change.path).map(|file| Item {
                url: Some(file.url.clone()),
                path: staging.join(&file.path),
                sha1: file.sha1.clone(),
                size: file.size,
            })
        })
        .collect();
    if items.is_empty() {
        return Ok(());
    }
    let parents: Vec<PathBuf> = items.iter().filter_map(|item| item.path.parent().map(PathBuf::from)).collect();
    tokio::task::spawn_blocking(move || {
        for parent in parents {
            let _ = std::fs::create_dir_all(parent);
        }
    })
    .await
    .map_err(|error| error.to_string())?;
    let missing = download::missing(items).await?;
    let total = missing.present + missing.missing.len();
    let (reports_tx, reports) = mpsc::channel::<PartReport>(256);
    let mut totals = Totals::default();
    let work = download::fetch(client, missing.missing, Part::Mods, missing.present, total, &reports_tx);
    drive(
        progress,
        reports,
        move |report| {
            let sum = totals.apply(report);
            InstallProgress {
                stage: Stage::Modpack,
                fraction: fraction(sum.done, sum.total),
                detail: lang::t_with(
                    "install.files_count",
                    &[("done", sum.done.to_string().as_str()), ("total", sum.total.to_string().as_str())],
                ),
                transfer: sum.transfer,
            }
        },
        work,
    )
    .await?;
    Ok(())
}

/// Whether the reviewed changes and the planned ones name the same paths with the same kinds. Both are in path order.
fn same_changes(reviewed: &[Change], planned: &[(Change, Decision)]) -> bool {
    reviewed.len() == planned.len()
        && reviewed
            .iter()
            .zip(planned)
            .all(|(reviewed, (planned, _))| reviewed.path == planned.path && reviewed.kind == planned.kind)
}

/// Whether a change of this kind writes the release's copy of its file.
fn writes_from_release(kind: &Kind) -> bool {
    matches!(kind, Kind::Add | Kind::Restore | Kind::Update | Kind::Supersede { .. } | Kind::Conflict)
}

/// Finishes a committed update: its originals are discarded and its pristine copies take the old ones' place. A failure here is left
/// to the next launch, which finishes the same workspace, since the commit already happened.
async fn settle_committed(ws: &Workspace) {
    let ws = ws.clone();
    match tokio::task::spawn_blocking(move || ws.finish()).await {
        Ok(Ok(())) => {}
        Ok(Err(error)) => warn!("the update committed, but its files were not settled: {error}"),
        Err(error) => warn!("the update committed, but its files were not settled: {error}"),
    }
}
