//! The mod queue: the updates Update all installs, and the changes a rollback makes, one mod after another.
//!
//! The steps run one at a time. Two that ran at once could both fetch the same dependency, and each would write its jar. A step
//! that fails does not stop the rest: its failure is kept, and one alert names them all at the end.

use std::collections::VecDeque;

use lodestone_snapshots::Kind;
use lodestone_snapshots::store;

use super::*;
use crate::app::pages::discover::Source;
use crate::app::pages::instance::mods::Row;
use crate::app::snapshot_actions;

/// What a queue is for. The summary that ends a queue names its steps in these words.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Purpose {
    /// Update all installs the newer releases.
    #[default]
    UpdateAll,
    /// A rollback puts the mods of a snapshot back.
    Rollback,
}

/// One step a queue runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Job {
    /// Installs the newer release the platform has for an installed mod.
    Update { file_name: String, name: String },
    /// Replaces an installed mod with the release a snapshot holds.
    Restore {
        file_name: String,
        name: String,
        source: Source,
        project_id: String,
        version_id: String,
    },
    /// Installs the release a snapshot holds, for a mod the instance no longer has.
    Install {
        name: String,
        source: Source,
        project_id: String,
        version_id: String,
    },
}

impl Job {
    /// The name the summary shows for the mod.
    pub(in crate::app) fn name(&self) -> &str {
        match self {
            Job::Update { name, .. } | Job::Restore { name, .. } | Job::Install { name, .. } => name.as_str(),
        }
    }

    /// The installed jar the step replaces, when it replaces one.
    pub(in crate::app) fn file_name(&self) -> Option<&str> {
        match self {
            Job::Update { file_name, .. } | Job::Restore { file_name, .. } => Some(file_name.as_str()),
            Job::Install { .. } => None,
        }
    }
}

/// A mod that could not be updated, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    pub name: String,
    pub error: String,
}

/// The steps a queue still has to run, and the ones that failed so far.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct UpdateQueue {
    /// The generation of the instance page the queue was started on. A queue for another instance is dropped.
    generation: u64,
    purpose: Purpose,
    /// The steps still to run, in the order the list shows the mods.
    pending: VecDeque<Job>,
    /// The steps that could not be carried out, with why.
    failed: Vec<Failure>,
}

impl UpdateQueue {
    /// A queue of Update all's `jobs` for the instance page at `generation`.
    pub fn new(generation: u64, jobs: Vec<Job>) -> Self {
        Self::with(generation, Purpose::UpdateAll, jobs)
    }

    /// A queue of a rollback's `jobs` for the instance page at `generation`.
    pub fn rollback(generation: u64, jobs: Vec<Job>) -> Self {
        Self::with(generation, Purpose::Rollback, jobs)
    }

    fn with(generation: u64, purpose: Purpose, jobs: Vec<Job>) -> Self {
        Self {
            generation,
            purpose,
            pending: jobs.into(),
            failed: Vec::new(),
        }
    }

    /// The step to run next, taken off the queue.
    fn next(&mut self) -> Option<Job> {
        self.pending.pop_front()
    }

    /// The installed jars the steps still waiting would replace, so a queue that stops can release them.
    fn waiting_names(&self) -> Vec<String> {
        self.pending.iter().filter_map(|job| job.file_name().map(str::to_string)).collect()
    }

    /// Records that the step for the mod `name` failed.
    fn fail(&mut self, name: String, error: String) {
        self.failed.push(Failure { name, error });
    }

    /// Fails every step still waiting with `error`, and returns the installed jars they would have replaced.
    fn fail_rest(&mut self, error: &str) -> Vec<String> {
        let rest: Vec<Job> = self.pending.drain(..).collect();
        rest.into_iter()
            .filter_map(|job| {
                self.fail(job.name().to_string(), error.to_string());
                job.file_name().map(str::to_string)
            })
            .collect()
    }

    /// The text of the alert that ends the queue, when a step failed: one line per mod.
    fn summary(&self) -> Option<String> {
        if self.failed.is_empty() {
            return None;
        }
        let mut text = match self.purpose {
            Purpose::UpdateAll => lang::t_count("instance.mods.updates_failed", self.failed.len()),
            Purpose::Rollback => lang::t_count("snapshots.rollback.failed", self.failed.len()),
        };
        for failure in &self.failed {
            text.push('\n');
            text.push_str(&lang::t_with(
                "instance.mods.updates_failed_line",
                &[("name", failure.name.as_str()), ("error", failure.error.as_str())],
            ));
        }
        Some(text)
    }
}

impl LodestoneLauncher {
    /// Update all was pressed: every mod with a newer release is queued, and the first update starts. Nothing is queued for
    /// a locked instance, or while a game runs on it, since the game may have the jars open.
    pub(in crate::app) fn update_all_mods(&mut self) -> Task<LodestoneAppMessage> {
        if self.shown_locked() || self.updating_all_mods() {
            return Task::none();
        }
        let Some(context) = self.mods_context() else {
            return Task::none();
        };
        if self.games.is_active(&context.instance.id) {
            return self.alerts.show(Alert::warn(lang::t("instance.mods.running"))).discard();
        }
        let mods = self.instance_page.mods_mut();
        let queued: Vec<Job> = mods
            .updatable()
            .into_iter()
            .map(|file_name| {
                let name = mods.row(&file_name).map_or_else(|| file_name.clone(), |row| row.file.name.clone());
                Job::Update { file_name, name }
            })
            .collect();
        if queued.is_empty() {
            return Task::none();
        }
        // Every queued mod waits with its controls off, so none is switched or removed before its turn.
        for job in &queued {
            if let Some(file_name) = job.file_name() {
                mods.set_busy(file_name, true);
            }
        }
        mods.set_updating_all(true);
        let names: Vec<String> = queued.iter().filter_map(|job| job.file_name().map(str::to_string)).collect();
        self.mod_update_queue = Some(UpdateQueue::new(self.instance_page.generation(), queued));
        // With snapshots on, the mods are written down before the first one is replaced, and the queue starts once that is written.
        if self.snapshot_limit() > 0 {
            return self.write_update_snapshot(&names);
        }
        self.next_mod_update()
    }

    /// Writes the snapshot Update all takes of the mods `names`, before the first of them is replaced. The answer comes back as
    /// [`snapshot_actions::Message::UpdateAll`].
    fn write_update_snapshot(&self, names: &[String]) -> Task<LodestoneAppMessage> {
        let Some(instance) = self.instance_page.instance() else {
            return Task::none();
        };
        let folder = self.paths.instance(&instance.directory_name);
        let rows: Vec<&Row> = names.iter().filter_map(|name| self.instance_page.mods().row(name)).collect();
        let (snapshot, _) = snapshot_actions::gather(&instance.mc_version, instance.loader.to_db().to_string(), &folder.join("mods"), &rows);
        let limit = self.snapshot_limit();
        let generation = self.instance_page.generation();
        Task::perform(
            snapshot_actions::write(store::dir(&folder), Kind::UpdateAll, snapshot, Vec::new(), limit),
            move |result| LodestoneAppMessage::Snapshots(snapshot_actions::Message::UpdateAll { generation, result }),
        )
    }

    /// The snapshot Update all took was written, or could not be. Written, the queue starts. Not written, no mod is replaced: the
    /// queue is dropped, its mods are released, and an alert says why.
    pub(in crate::app) fn update_all_snapshot_written(&mut self, generation: u64, result: Result<(), String>) -> Task<LodestoneAppMessage> {
        if generation != self.instance_page.generation() {
            return Task::none();
        }
        match result {
            Ok(()) => Task::batch([self.next_mod_update(), self.reload_snapshots()]),
            Err(error) => {
                warn!("the snapshot before Update all could not be saved: {error}");
                let waiting = self.mod_update_queue.take().map(|queue| queue.waiting_names()).unwrap_or_default();
                let mods = self.instance_page.mods_mut();
                for name in &waiting {
                    mods.set_busy(name, false);
                }
                mods.set_updating_all(false);
                self.alerts
                    .show(Alert::error(lang::t_with("snapshots.update_all.failed", &[("error", error.as_str())])))
                    .discard()
            }
        }
    }

    /// Starts a rollback's queue: its `jobs`, and the `failures` it already has, such as a local jar that differs from its
    /// snapshot. The queue runs with the Update all controls off, and its first step starts now.
    pub(in crate::app) fn start_rollback_queue(&mut self, jobs: Vec<Job>, failures: Vec<Failure>) -> Task<LodestoneAppMessage> {
        let generation = self.instance_page.generation();
        let mut queue = UpdateQueue::rollback(generation, jobs);
        queue.failed = failures;
        for job in &queue.pending {
            if let Some(file_name) = job.file_name() {
                self.instance_page.mods_mut().set_busy(file_name, true);
            }
        }
        self.instance_page.mods_mut().set_updating_all(true);
        self.mod_update_queue = Some(queue);
        self.next_mod_update()
    }

    /// Whether a queue of Update all or a rollback is running for the instance the page shows.
    pub(in crate::app) fn updating_all_mods(&self) -> bool {
        self.mod_update_queue.as_ref().is_some_and(|queue| self.mods_fresh(queue.generation))
    }

    /// Keeps the failure of the mod `file_name` for the queue's summary. Returns `false` when no queue is running, and the
    /// failure is then the caller's to show.
    pub(super) fn keep_update_failure(&mut self, file_name: &str, error: &str) -> bool {
        if !self.updating_all_mods() {
            return false;
        }
        let name = self
            .instance_page
            .mods()
            .row(file_name)
            .map_or_else(|| file_name.to_string(), |row| row.file.name.clone());
        if let Some(queue) = &mut self.mod_update_queue {
            queue.fail(name, error.to_string());
        }
        true
    }

    /// Keeps the failure of an install step for the mod `name`, which has no jar on the page yet. Returns `false` when no queue
    /// is running.
    pub(super) fn keep_install_failure(&mut self, name: &str, error: &str) -> bool {
        if !self.updating_all_mods() {
            return false;
        }
        if let Some(queue) = &mut self.mod_update_queue {
            queue.fail(name.to_string(), error.to_string());
        }
        true
    }

    /// Starts the next step of the queue, or ends the queue when none is left. A mod whose update is gone by its turn, because an
    /// earlier update replaced it as a dependency, is passed over. A game that started meanwhile fails the rest, and a step that
    /// cannot start is failed and passed over.
    pub(super) fn next_mod_update(&mut self) -> Task<LodestoneAppMessage> {
        if !self.updating_all_mods() {
            self.mod_update_queue = None;
            return Task::none();
        }
        let playing = self.mods_context().is_none_or(|context| self.games.is_active(&context.instance.id));
        if playing && let Some(queue) = &mut self.mod_update_queue {
            for file_name in queue.fail_rest(lang::t("instance.mods.running")) {
                self.instance_page.mods_mut().set_busy(&file_name, false);
            }
        }
        while let Some(job) = self.mod_update_queue.as_mut().and_then(UpdateQueue::next) {
            match job {
                Job::Update { file_name, .. } => {
                    let release = self
                        .instance_page
                        .mods()
                        .row(&file_name)
                        .filter(|row| row.source.is_some())
                        .and_then(|row| row.update.clone());
                    match release {
                        Some(release) => return self.replace_mod(file_name, release.id),
                        None => self.instance_page.mods_mut().set_busy(&file_name, false),
                    }
                }
                Job::Restore {
                    file_name,
                    name,
                    source,
                    project_id,
                    version_id,
                } => {
                    if !self.can_change_mods() {
                        self.refuse_step(&name, Some(&file_name));
                        continue;
                    }
                    return self.replace_with(file_name, source, project_id, name, version_id);
                }
                Job::Install {
                    name,
                    source,
                    project_id,
                    version_id,
                } => {
                    if !self.can_change_mods() {
                        self.refuse_step(&name, None);
                        continue;
                    }
                    return self.install_version(source, project_id, name, version_id);
                }
            }
        }
        self.finish_mod_updates()
    }

    /// Whether the shown instance's mods can be changed right now: the database is open, and the instance is not locked.
    fn can_change_mods(&self) -> bool {
        self.db.is_some() && !self.shown_locked() && self.mods_context().is_some()
    }

    /// Fails a step that cannot start, and releases the jar it would have replaced.
    fn refuse_step(&mut self, name: &str, file_name: Option<&str>) {
        if let Some(queue) = &mut self.mod_update_queue {
            queue.fail(name.to_string(), lang::t("snapshots.rollback.blocked").to_string());
        }
        if let Some(file_name) = file_name {
            self.instance_page.mods_mut().set_busy(file_name, false);
        }
    }

    /// Ends the queue: Update all is offered again, and the steps that failed are named in one alert.
    fn finish_mod_updates(&mut self) -> Task<LodestoneAppMessage> {
        self.instance_page.mods_mut().set_updating_all(false);
        match self.mod_update_queue.take().and_then(|queue| queue.summary()) {
            Some(summary) => self.alerts.show(Alert::error(summary)).discard(),
            None => Task::none(),
        }
    }
}

#[cfg(test)]
#[path = "../../../tests/app/mod_actions/update_all.rs"]
mod tests;
