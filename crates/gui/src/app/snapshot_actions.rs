//! Mod snapshots, as the root carries them out. An Update all writes one before it replaces a mod, a manual snapshot saves every
//! mod the instance holds, and a rollback puts a snapshot's mods back. The files are written and read on the blocking pool, and
//! each answer comes back here as a [`Message`] for the page it was started on.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use chrono::{Local, NaiveDateTime};
use iced::Task;
use lodestone_snapshots::hash::sha1_of_file;
use lodestone_snapshots::plan::{self, Installed, LocalState, Refusal, State, Target};
use lodestone_snapshots::store::{self, Listed, LocalFile};
use lodestone_snapshots::{Entry, Kind, LocalJar, Platform, Snapshot};
use tracing::warn;

use super::LodestoneAppMessage;
use super::LodestoneLauncher;
use super::components::alert::Alert;
use super::database::Database;
use super::lang;
use super::mod_actions::{Failure, Job};
use super::pages::discover::Source;
use super::pages::instance::browser;
use super::pages::instance::files::{self, SnapshotFile};
use super::pages::instance::mods::Row;
use super::pages::instance::snapshots::{Action, Loaded, Message as TabMessage, Prepared};
use super::pages::instance::{Message as InstanceMessage, Tab};
use super::shared_library::ModSource;

/// Why a rollback is being read: to ask first, or to let the user pick the versions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ask {
    /// Asks whether to roll every mod back.
    Confirm,
    /// Opens the diff, so the versions can be picked one by one.
    Compare,
}

/// What the local jars of a rollback did, once the snapshot a rollback saves first is written.
#[derive(Debug, Clone, Default)]
pub struct LocalOutcome {
    /// The local jars that were copied back into `mods/`.
    pub restored: Vec<String>,
    /// The local jars that were left alone or could not be copied back, with why.
    pub failures: Vec<Failure>,
}

/// What the snapshot tasks answer. Each carries the generation of the instance page it was started for, and an answer for a page
/// that has moved on is dropped.
#[derive(Debug, Clone)]
pub enum Message {
    /// The snapshot an Update all takes before it replaces a mod was written, or could not be.
    UpdateAll { generation: u64, result: Result<(), String> },
    /// A manual snapshot was written, or could not be.
    Written { generation: u64, result: Result<(), String> },
    /// The snapshots of the instance were read again.
    Listed { generation: u64, list: Vec<SnapshotFile> },
    /// A rollback was read against the instance, or could not be. `ask` is what the page wanted with it.
    Planned {
        generation: u64,
        ask: Ask,
        result: Result<Prepared, String>,
    },
    /// The question about a rollback was answered. `confirmed` is whether it was said yes.
    Confirmed { generation: u64, prepared: Prepared, confirmed: bool },
    /// The snapshot a rollback saves first was written, and its missing local jars were copied back, or could not be.
    Ready {
        generation: u64,
        jobs: Vec<Job>,
        result: Result<LocalOutcome, String>,
    },
    /// The newest snapshot of the instance `instance` was read after its game crashed, with the instance's whole list.
    CrashNewest { instance: String, list: Vec<SnapshotFile> },
    /// The question about deleting a snapshot was answered. `confirmed` is whether it was said yes.
    DeleteAnswered { generation: u64, listed: Listed, confirmed: bool },
    /// A snapshot's files were moved to the trash, or could not be.
    Deleted { generation: u64, result: Result<(), String> },
}

/// The platform and the entry of a mod that has a platform record. A jar without one, or with a platform this build does not
/// know, has none, and so is kept as a local jar instead.
fn entry_of(row: &Row) -> Option<(Platform, Entry)> {
    let record = row.source.as_ref()?;
    let platform = Platform::from_key(&record.platform)?;
    Some((
        platform,
        Entry {
            modid: record.project_id.clone(),
            version: record.version_number.clone().or_else(|| row.file.version.clone()),
            fileid: record.version_id.clone(),
            name: record.name.clone(),
        },
    ))
}

/// The snapshot of `rows`, for an instance on `minecraft` with `loader`. A mod with a platform record is a linked entry, and
/// every other jar is returned as a local file, read from `mods_dir`, for the writer to keep.
pub(super) fn gather(minecraft: &str, loader: String, mods_dir: &Path, rows: &[&Row]) -> (Snapshot, Vec<LocalFile>) {
    let mut snapshot = Snapshot {
        minecraft: minecraft.to_string(),
        loader,
        ..Snapshot::default()
    };
    let mut local = Vec::new();
    for row in rows {
        match entry_of(row) {
            Some((Platform::Modrinth, entry)) => snapshot.modrinth.push(entry),
            Some((Platform::CurseForge, entry)) => snapshot.curseforge.push(entry),
            None => local.push(LocalFile {
                file: row.file.file_name.clone(),
                path: mods_dir.join(&row.file.file_name),
            }),
        }
    }
    (snapshot, local)
}

/// Writes `snapshot` of `kind` into the snapshots folder `dir`, hashing `local` into its zip, and then keeps the newest `limit`.
/// Resolves once the files are whole, or with why they are not.
pub(super) async fn write(dir: PathBuf, kind: Kind, snapshot: Snapshot, local: Vec<LocalFile>, limit: u32) -> Result<(), String> {
    let joined = tokio::task::spawn_blocking(move || -> Result<(), String> {
        let when = Local::now().naive_local();
        store::write(&dir, kind, when, snapshot, &local).map_err(|error| error.to_string())?;
        store::prune(&dir, limit).map_err(|error| error.to_string())?;
        Ok(())
    })
    .await;
    joined.map_err(|error| error.to_string())?
}

/// What a rollback is read from: the snapshot, and the instance it would be put back on.
struct Reading {
    db: Database,
    instance: String,
    mods: PathBuf,
    snapshot: Snapshot,
    target: Target,
    /// The version each jar of the instance declares, by file name, for a record that names none.
    declared: HashMap<String, Option<String>>,
    stem: String,
    when: NaiveDateTime,
    zip: Option<PathBuf>,
}

/// A mod the database records for the instance, as the planner reads it. A record on a platform this build does not know is left
/// out.
fn installed_of(record: &ModSource, declared: &HashMap<String, Option<String>>) -> Option<Installed> {
    Some(Installed {
        platform: Platform::from_key(&record.platform)?,
        project_id: record.project_id.clone(),
        version_id: record.version_id.clone(),
        version: record
            .version_number
            .clone()
            .or_else(|| declared.get(&record.file_name).cloned().flatten()),
        file_name: record.file_name.clone(),
    })
}

/// The SHA-1 of each of the snapshot's local jars that `mods` holds, by file name. A jar that is not there has none.
fn hash_present(mods: &Path, local: &[LocalJar]) -> HashMap<String, String> {
    local
        .iter()
        .filter_map(|jar| {
            let sha1 = sha1_of_file(&mods.join(&jar.file)).ok()?;
            Some((jar.file.clone(), sha1))
        })
        .collect()
}

/// A loader's name for a message: its key, or Vanilla for none.
fn loader_label(key: &str) -> String {
    if key.is_empty() {
        lang::t("snapshots.rollback.vanilla").to_string()
    } else {
        key.to_string()
    }
}

/// Why a snapshot cannot be put back on the instance, as the alert shows it.
fn refusal_text(refusal: &Refusal) -> String {
    match refusal {
        Refusal::Minecraft { snapshot, instance } => lang::t_with(
            "snapshots.rollback.refused.minecraft",
            &[("snapshot", snapshot.as_str()), ("instance", instance.as_str())],
        ),
        Refusal::Loader { snapshot, instance } => {
            let snapshot = loader_label(snapshot);
            let instance = loader_label(instance);
            lang::t_with(
                "snapshots.rollback.refused.loader",
                &[("snapshot", snapshot.as_str()), ("instance", instance.as_str())],
            )
        }
    }
}

/// Reads the snapshot against the mods the database holds for the instance, and the local jars in `mods`. Resolves with what a
/// rollback would change, or with why it cannot happen at all.
async fn read(reading: Reading) -> Result<Prepared, String> {
    let records = reading.db.mod_sources_of(&reading.instance).await?;
    let joined = tokio::task::spawn_blocking(move || -> Result<Prepared, String> {
        let installed: Vec<Installed> = records.iter().filter_map(|record| installed_of(record, &reading.declared)).collect();
        let on_disk = hash_present(&reading.mods, &reading.snapshot.local);
        let plan = plan::plan(&reading.snapshot, &reading.target, &installed, &on_disk).map_err(|refusal| refusal_text(&refusal))?;
        Ok(Prepared {
            stem: reading.stem,
            when: reading.when,
            plan,
            zip: reading.zip,
        })
    })
    .await;
    joined.map_err(|error| error.to_string())?
}

/// Shows the snapshots folder in the Files tab, which the instance keeps under `.lodestone/snapshots`.
fn reveal_snapshots() -> Task<LodestoneAppMessage> {
    let open = browser::Message::Open(".lodestone/snapshots".to_string());
    Task::done(LodestoneAppMessage::Instance(InstanceMessage::TabSelected(Tab::Files)))
        .chain(Task::done(LodestoneAppMessage::Instance(InstanceMessage::Browser(open))))
}

/// The indices of the mods of `prepared` a full rollback changes: every one that is not already at the snapshot's release.
fn changes_of(prepared: &Prepared) -> Vec<usize> {
    prepared
        .plan
        .items
        .iter()
        .enumerate()
        .filter(|(_, item)| item.state != State::Same)
        .map(|(index, _)| index)
        .collect()
}

impl LodestoneLauncher {
    /// How many snapshots the shown instance keeps. Zero means none are taken.
    pub(super) fn snapshot_limit(&self) -> u32 {
        self.instance_page.instance().map_or(0, |instance| {
            instance.settings.resolved_snapshot_limit(&self.settings_values.instance_defaults)
        })
    }

    /// Reads the shown instance's snapshots again, on the blocking pool. The answer lands as [`Message::Listed`].
    pub(super) fn reload_snapshots(&self) -> Task<LodestoneAppMessage> {
        let Some(instance) = self.instance_page.instance() else {
            return Task::none();
        };
        let folder = self.paths.instance(&instance.directory_name);
        let generation = self.instance_page.generation();
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || files::snapshots::scan(&folder))
                    .await
                    .unwrap_or_default()
            },
            move |list| LodestoneAppMessage::Snapshots(Message::Listed { generation, list }),
        )
    }

    /// Saves every mod the shown instance holds as a manual snapshot. A mod with a platform record is kept by its release, and any
    /// other jar in `mods/` goes in the snapshot's zip. Nothing is taken while an Update all runs, or when the instance keeps none.
    pub(super) fn take_manual_snapshot(&mut self) -> Task<LodestoneAppMessage> {
        let limit = self.snapshot_limit();
        if limit == 0 || self.instance_page.mods().updating_all() {
            return Task::none();
        }
        let Some(instance) = self.instance_page.instance() else {
            return Task::none();
        };
        let folder = self.paths.instance(&instance.directory_name);
        let rows: Vec<&Row> = self.instance_page.mods().rows().iter().collect();
        if rows.is_empty() {
            // The folder is read as the page opens, so an empty list here means nothing is known yet, not that there are no mods.
            return self.alerts.show(Alert::info(lang::t("snapshots.manual.empty"))).discard();
        }
        let (snapshot, local) = gather(&instance.mc_version, instance.loader.to_db().to_string(), &folder.join("mods"), &rows);
        let generation = self.instance_page.generation();
        Task::perform(write(store::dir(&folder), Kind::Manual, snapshot, local, limit), move |result| {
            LodestoneAppMessage::Snapshots(Message::Written { generation, result })
        })
    }

    /// Reads what the rollback of the snapshot `stem` would change, against the shown instance. `ask` says whether the result is
    /// confirmed first, or opens the diff for the versions to be picked.
    pub(super) fn request_rollback(&mut self, stem: String, ask: Ask) -> Task<LodestoneAppMessage> {
        match self.instance_page.snapshots().find(&stem).cloned() {
            Some(loaded) => self.request_rollback_loaded(loaded, ask),
            None => Task::none(),
        }
    }

    /// Reads what the rollback of `loaded` would change, against the shown instance. See [`Self::request_rollback`].
    pub(super) fn request_rollback_loaded(&mut self, loaded: Loaded, ask: Ask) -> Task<LodestoneAppMessage> {
        let Some(instance) = self.instance_page.instance() else {
            return Task::none();
        };
        if self.shown_locked() || self.shown_instance_updating() || self.instance_page.mods().updating_all() {
            return Task::none();
        }
        if self.games.is_active(&instance.id) {
            return self.alerts.show(Alert::warn(lang::t("instance.mods.running"))).discard();
        }
        let Some(db) = self.db.clone() else {
            return Task::none();
        };
        let folder = self.paths.instance(&instance.directory_name);
        let declared = self
            .instance_page
            .mods()
            .rows()
            .iter()
            .map(|row| (row.file.file_name.clone(), row.file.version.clone()))
            .collect();
        let reading = Reading {
            db,
            instance: instance.id.clone(),
            mods: folder.join("mods"),
            snapshot: loaded.snapshot.clone(),
            target: Target {
                minecraft: instance.mc_version.clone(),
                loader: instance.loader.to_db().to_string(),
            },
            declared,
            stem: loaded.listed.name.stem(),
            when: loaded.listed.name.when,
            zip: loaded.listed.zip.clone(),
        };
        let generation = self.instance_page.generation();
        Task::perform(read(reading), move |result| {
            LodestoneAppMessage::Snapshots(Message::Planned { generation, ask, result })
        })
    }

    /// Asks before a rollback changes anything, or says there is nothing to change.
    fn ask_rollback(&mut self, generation: u64, prepared: Prepared) -> Task<LodestoneAppMessage> {
        let missing = prepared.plan.local.iter().filter(|jar| jar.state == LocalState::Missing).count();
        let count = prepared.plan.changes() + missing;
        if count == 0 {
            return self.alerts.show(Alert::info(lang::t("snapshots.rollback.nothing"))).discard();
        }
        let message = lang::t_with("snapshots.rollback.confirm.message", &[("count", count.to_string().as_str())]);
        self.alerts
            .show(
                Alert::warn(message)
                    .title(lang::t("snapshots.rollback.confirm.title"))
                    .labelled(lang::t("snapshots.rollback.confirm.roll_back"), Some(lang::t("common.cancel"))),
            )
            .map(move |confirmed| {
                LodestoneAppMessage::Snapshots(Message::Confirmed {
                    generation,
                    prepared: prepared.clone(),
                    confirmed,
                })
            })
    }

    /// Starts a rollback of `prepared`, for the mods at the `selected` indices of its plan. Each mod it replaces is first saved in a
    /// rollback snapshot, so the rollback can be undone, unless the instance keeps no snapshots. `restore_local` also copies back
    /// the local jars that are missing from `mods/`.
    pub(super) fn start_rollback(&mut self, prepared: Prepared, selected: Vec<usize>, restore_local: bool) -> Task<LodestoneAppMessage> {
        let Some(instance) = self.instance_page.instance() else {
            return Task::none();
        };
        if self.shown_locked() || self.shown_instance_updating() || self.updating_all_mods() {
            return Task::none();
        }
        if self.games.is_active(&instance.id) {
            return self.alerts.show(Alert::warn(lang::t("instance.mods.running"))).discard();
        }
        let folder = self.paths.instance(&instance.directory_name);
        let generation = self.instance_page.generation();
        let limit = self.snapshot_limit();
        let mut undo = Snapshot {
            minecraft: instance.mc_version.clone(),
            loader: instance.loader.to_db().to_string(),
            ..Snapshot::default()
        };
        let mut jobs = Vec::new();
        for index in selected {
            let Some(item) = prepared.plan.items.get(index) else {
                continue;
            };
            let Some(source) = Source::from_key(item.platform.key()) else {
                continue;
            };
            match item.state {
                State::Changed => {
                    let Some(installed) = item.installed.as_ref() else {
                        continue;
                    };
                    let entry = Entry {
                        modid: installed.project_id.clone(),
                        version: installed.version.clone(),
                        fileid: installed.version_id.clone(),
                        name: item.name.clone(),
                    };
                    match item.platform {
                        Platform::Modrinth => undo.modrinth.push(entry),
                        Platform::CurseForge => undo.curseforge.push(entry),
                    }
                    jobs.push(Job::Restore {
                        file_name: installed.file_name.clone(),
                        name: item.name.clone(),
                        source,
                        project_id: item.project_id.clone(),
                        version_id: item.wanted_id.clone(),
                    });
                }
                State::NotInstalled => jobs.push(Job::Install {
                    name: item.name.clone(),
                    source,
                    project_id: item.project_id.clone(),
                    version_id: item.wanted_id.clone(),
                }),
                State::Same => {}
            }
        }
        let mut failures = Vec::new();
        let mut locals = Vec::new();
        if restore_local {
            for jar in &prepared.plan.local {
                match jar.state {
                    LocalState::Missing => locals.push((jar.file.clone(), jar.sha1.clone())),
                    LocalState::Differs => failures.push(Failure {
                        name: jar.file.clone(),
                        error: lang::t_with("snapshots.rollback.local_differs", &[("file", jar.file.as_str())]),
                    }),
                    LocalState::Same => {}
                }
            }
        }
        // The mods are marked busy while the snapshot is written, so they cannot be changed in the meantime.
        for job in &jobs {
            if let Some(file_name) = job.file_name() {
                self.instance_page.mods_mut().set_busy(file_name, true);
            }
        }
        let save_undo = limit > 0 && undo.linked_count() > 0;
        let dir = store::dir(&folder);
        let mods = folder.join("mods");
        let zip = prepared.zip.clone();
        let work = async move {
            let joined = tokio::task::spawn_blocking(move || -> Result<LocalOutcome, String> {
                if save_undo {
                    store::write(&dir, Kind::Rollback, Local::now().naive_local(), undo, &[]).map_err(|error| error.to_string())?;
                    store::prune(&dir, limit).map_err(|error| error.to_string())?;
                }
                let mut outcome = LocalOutcome {
                    restored: Vec::new(),
                    failures,
                };
                if let Some(zip) = zip {
                    for (file, sha1) in locals {
                        match store::restore_local(&zip, &file, &sha1, &mods.join(&file)) {
                            Ok(()) => outcome.restored.push(file),
                            Err(error) => {
                                let error = error.to_string();
                                outcome.failures.push(Failure {
                                    error: lang::t_with("snapshots.rollback.local_failed", &[("file", file.as_str()), ("error", error.as_str())]),
                                    name: file,
                                });
                            }
                        }
                    }
                }
                Ok(outcome)
            })
            .await;
            joined.map_err(|error| error.to_string())?
        };
        Task::perform(work, move |result| {
            LodestoneAppMessage::Snapshots(Message::Ready { generation, jobs, result })
        })
    }

    /// Rolls back the newest snapshot of the instance `instance` after its game crashed. The instance's page is opened first, since a
    /// rollback works on the instance the page shows. The snapshot is then read from its folder, and the rollback is asked for once it
    /// is read, if the page still shows the instance.
    pub(super) fn roll_back_after_crash(&mut self, instance: &str) -> Task<LodestoneAppMessage> {
        let Some(record) = self.library.instance(instance) else {
            return Task::none();
        };
        let folder = self.paths.instance(&record.directory_name);
        let id = instance.to_string();
        let open = self.open_instance_page(instance, Tab::Content);
        let read = Task::perform(
            async move {
                tokio::task::spawn_blocking(move || files::snapshots::scan(&folder))
                    .await
                    .unwrap_or_default()
            },
            move |list| LodestoneAppMessage::Snapshots(Message::CrashNewest { instance: id.clone(), list }),
        );
        Task::batch([open, read])
    }

    /// Carries out what the Snapshots tab asked for. Its rollbacks and deletes are the root's, and the diff's ticks and close are
    /// the tab's own.
    pub(super) fn snapshots_page_message(&mut self, message: TabMessage) -> Task<LodestoneAppMessage> {
        match message {
            TabMessage::RollBack(stem) => self.request_rollback(stem, Ask::Confirm),
            TabMessage::Menu { stem, action } => match action {
                Some(Action::Compare) => self.request_rollback(stem, Ask::Compare),
                Some(Action::RollBackAll) => self.request_rollback(stem, Ask::Confirm),
                Some(Action::Reveal) => reveal_snapshots(),
                Some(Action::Delete) => self.ask_delete_snapshot(&stem),
                None => Task::none(),
            },
            TabMessage::Toggle(index) => {
                self.instance_page.snapshots_mut().toggle(index);
                Task::none()
            }
            TabMessage::CloseDialog => {
                self.instance_page.snapshots_mut().close_compare();
                Task::none()
            }
            TabMessage::RollBackSelected => match self.instance_page.snapshots_mut().take_picked() {
                Some((prepared, picked)) => self.start_rollback(prepared, picked, false),
                None => Task::none(),
            },
        }
    }

    /// Asks before the snapshot `stem` is moved to the trash. The answer comes back as [`Message::DeleteAnswered`].
    fn ask_delete_snapshot(&mut self, stem: &str) -> Task<LodestoneAppMessage> {
        let Some(listed) = self.instance_page.snapshots().find(stem).map(|loaded| loaded.listed.clone()) else {
            return Task::none();
        };
        let generation = self.instance_page.generation();
        self.alerts
            .show(
                Alert::danger(lang::t("snapshots.delete.message"))
                    .title(lang::t("snapshots.delete.title"))
                    .labelled(lang::t("snapshots.delete.confirm"), Some(lang::t("common.cancel"))),
            )
            .map(move |confirmed| {
                LodestoneAppMessage::Snapshots(Message::DeleteAnswered {
                    generation,
                    listed: listed.clone(),
                    confirmed,
                })
            })
    }

    /// Carries out what a snapshot task answered, for the page it was started on.
    pub(super) fn snapshot_message(&mut self, message: Message) -> Task<LodestoneAppMessage> {
        match message {
            Message::UpdateAll { generation, result } => self.update_all_snapshot_written(generation, result),
            Message::Written { generation, result } => {
                if generation != self.instance_page.generation() {
                    return Task::none();
                }
                match result {
                    Ok(()) => self.reload_snapshots(),
                    Err(error) => {
                        warn!("saving a snapshot failed: {error}");
                        self.alerts
                            .show(Alert::error(lang::t_with("snapshots.write_failed", &[("error", error.as_str())])))
                            .discard()
                    }
                }
            }
            Message::Listed { generation, list } => {
                if generation == self.instance_page.generation() {
                    let loaded = list.into_iter().map(Loaded::from).collect();
                    self.instance_page.snapshots_mut().set_list(loaded);
                }
                Task::none()
            }
            Message::CrashNewest { instance, list } => {
                let shown = self.instance_page.instance().is_some_and(|shown| shown.id == instance);
                match (shown, list.into_iter().next()) {
                    (true, Some(newest)) => self.request_rollback_loaded(Loaded::from(newest), Ask::Confirm),
                    _ => Task::none(),
                }
            }
            Message::DeleteAnswered {
                generation,
                listed,
                confirmed,
            } => {
                if !confirmed || generation != self.instance_page.generation() {
                    return Task::none();
                }
                // The JSON goes first, so a snapshot that is only partly trashed is not listed.
                let paths: Vec<PathBuf> = listed.files().into_iter().map(Path::to_path_buf).collect();
                Task::perform(
                    async move {
                        let joined = tokio::task::spawn_blocking(move || -> Result<(), String> {
                            paths.iter().try_for_each(|path| files::move_to_trash(path))
                        })
                        .await;
                        joined.map_err(|error| error.to_string())?
                    },
                    move |result| LodestoneAppMessage::Snapshots(Message::Deleted { generation, result }),
                )
            }
            Message::Deleted { generation, result } => {
                if generation != self.instance_page.generation() {
                    return Task::none();
                }
                match result {
                    Ok(()) => self.reload_snapshots(),
                    Err(error) => {
                        warn!("moving a snapshot to the trash failed: {error}");
                        self.alerts
                            .show(Alert::error(lang::t_with("snapshots.delete.failed", &[("error", error.as_str())])))
                            .discard()
                    }
                }
            }
            Message::Planned { generation, ask, result } => {
                if generation != self.instance_page.generation() {
                    return Task::none();
                }
                match (ask, result) {
                    (_, Err(error)) => self.alerts.show(Alert::error(error)).discard(),
                    (Ask::Confirm, Ok(prepared)) => self.ask_rollback(generation, prepared),
                    (Ask::Compare, Ok(prepared)) => {
                        self.instance_page.snapshots_mut().open_compare(prepared);
                        Task::none()
                    }
                }
            }
            Message::Confirmed {
                generation,
                prepared,
                confirmed,
            } => {
                if !confirmed || generation != self.instance_page.generation() {
                    return Task::none();
                }
                let selected = changes_of(&prepared);
                self.start_rollback(prepared, selected, true)
            }
            Message::Ready { generation, jobs, result } => {
                if generation != self.instance_page.generation() {
                    return Task::none();
                }
                match result {
                    Ok(outcome) => {
                        let rescan = if outcome.restored.is_empty() { Task::none() } else { self.rescan_mods() };
                        let started = self.start_rollback_queue(jobs, outcome.failures);
                        Task::batch([rescan, started, self.reload_snapshots()])
                    }
                    Err(error) => {
                        warn!("the rollback did not start: {error}");
                        let mods = self.instance_page.mods_mut();
                        for job in &jobs {
                            if let Some(file_name) = job.file_name() {
                                mods.set_busy(file_name, false);
                            }
                        }
                        self.alerts
                            .show(Alert::error(lang::t_with(
                                "snapshots.rollback.start_failed",
                                &[("error", error.as_str())],
                            )))
                            .discard()
                    }
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "../../tests/app/snapshot_actions.rs"]
mod tests;
