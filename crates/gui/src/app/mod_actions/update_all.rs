//! Update all: installs the newer release of every mod that has one, one mod after another.
//!
//! The updates run one at a time. Two that ran at once could both fetch the same dependency, and each would write its
//! jar. A mod that fails does not stop the rest: its failure is kept, and one alert names them all at the end.

use std::collections::VecDeque;

use super::*;

/// The updates Update all still has to run, and the ones that failed so far.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct UpdateQueue {
    /// The generation of the instance page the queue was started on. A queue for another instance is dropped.
    generation: u64,
    /// The mods still to update, by file name, in the order the list shows them.
    pending: VecDeque<Queued>,
    /// The mods that could not be updated, with why.
    failed: Vec<Failure>,
}

/// One mod waiting for its update: its file name, and the name the summary shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Queued {
    pub file_name: String,
    pub name: String,
}

/// A mod that could not be updated, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    pub name: String,
    pub error: String,
}

impl UpdateQueue {
    /// A queue of `mods` for the instance page at `generation`.
    pub fn new(generation: u64, mods: Vec<Queued>) -> Self {
        Self {
            generation,
            pending: mods.into(),
            failed: Vec::new(),
        }
    }

    /// The next mod to update, taken off the queue.
    fn next(&mut self) -> Option<Queued> {
        self.pending.pop_front()
    }

    /// Records that the update of the mod `name` failed.
    fn fail(&mut self, name: String, error: String) {
        self.failed.push(Failure { name, error });
    }

    /// Fails every mod still waiting with `error`, and returns their file names.
    fn fail_rest(&mut self, error: &str) -> Vec<String> {
        let rest: Vec<Queued> = self.pending.drain(..).collect();
        rest.into_iter()
            .map(|queued| {
                self.fail(queued.name, error.to_string());
                queued.file_name
            })
            .collect()
    }

    /// The text of the alert that ends the queue, when a mod failed: one line per mod.
    fn summary(&self) -> Option<String> {
        if self.failed.is_empty() {
            return None;
        }
        let mut text = lang::t_count("instance.mods.updates_failed", self.failed.len());
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
        let queued: Vec<Queued> = mods
            .updatable()
            .into_iter()
            .map(|file_name| {
                let name = mods.row(&file_name).map_or_else(|| file_name.clone(), |row| row.file.name.clone());
                Queued { file_name, name }
            })
            .collect();
        if queued.is_empty() {
            return Task::none();
        }
        // Every queued mod waits with its controls off, so none is switched or removed before its turn.
        for mod_ in &queued {
            mods.set_busy(&mod_.file_name, true);
        }
        mods.set_updating_all(true);
        self.mod_update_queue = Some(UpdateQueue::new(self.instance_page.generation(), queued));
        self.next_mod_update()
    }

    /// Whether an Update all queue is running for the instance the page shows.
    fn updating_all_mods(&self) -> bool {
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

    /// Starts the next queued update, or ends the queue when none is left. A mod whose update is gone by its turn, because
    /// an earlier update replaced it as a dependency, is passed over. A game that started meanwhile fails the rest.
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
        while let Some(next) = self.mod_update_queue.as_mut().and_then(UpdateQueue::next) {
            let release = self
                .instance_page
                .mods()
                .row(&next.file_name)
                .filter(|row| row.source.is_some())
                .and_then(|row| row.update.clone());
            match release {
                Some(release) => return self.replace_mod(next.file_name, release),
                None => self.instance_page.mods_mut().set_busy(&next.file_name, false),
            }
        }
        self.finish_mod_updates()
    }

    /// Ends the queue: Update all is offered again, and the mods that failed are named in one alert.
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
