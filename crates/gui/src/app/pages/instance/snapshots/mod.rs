//! The Snapshots tab, which the Content tab shows beside the mods and the packs once the instance keeps a snapshot. Each row is
//! one snapshot, newest first, with how many mods it holds, a Roll back button and a menu. The root writes, reads and rolls back
//! the snapshots; this module holds the list it is given and the answers the rows send.

pub mod dialog;
pub mod view;

use std::path::PathBuf;

use chrono::NaiveDateTime;
use lodestone_snapshots::Snapshot;
use lodestone_snapshots::plan::{Plan, State as Standing};
use lodestone_snapshots::store::Listed;

use crate::app::pages::instance::files::SnapshotFile;

/// A snapshot the instance folder holds, as the tab lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Loaded {
    pub listed: Listed,
    pub snapshot: Snapshot,
}

/// A rollback read against the instance: what the snapshot would change, and the zip its local jars are kept in. It is what a
/// rollback starts from, once the user has confirmed it or picked the versions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prepared {
    /// The snapshot's file stem, which names it in the list.
    pub stem: String,
    /// When the snapshot was taken.
    pub when: NaiveDateTime,
    /// What the rollback would change.
    pub plan: Plan,
    /// The zip of the snapshot's local jars, when it has one.
    pub zip: Option<PathBuf>,
}

impl From<SnapshotFile> for Loaded {
    fn from(file: SnapshotFile) -> Self {
        Self {
            listed: file.listed,
            snapshot: file.snapshot,
        }
    }
}

/// What a row's menu does for its snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Opens the diff of the snapshot against the mods the instance holds, where the versions can be picked one by one.
    Compare,
    /// Puts every mod of the snapshot back, as the Roll back button does.
    RollBackAll,
    /// Shows the snapshot's file in the Files tab.
    Reveal,
    /// Moves the snapshot's files to the trash, once the user has said so.
    Delete,
}

/// Messages the Snapshots tab emits. The root carries out the rollbacks, the deletes and the reveal; the page handles the rest.
#[derive(Debug, Clone)]
pub enum Message {
    /// The Roll back button of the snapshot named `stem` was pressed.
    RollBack(String),
    /// A row's menu row was picked for the snapshot named `stem`. A separator sends `None`.
    Menu { stem: String, action: Option<Action> },
    /// The checkbox of the change at `index` of the diff was toggled.
    Toggle(usize),
    /// The diff's Roll back selected button was pressed.
    RollBackSelected,
    /// The diff was closed.
    CloseDialog,
}

/// The snapshots the tab lists, newest first, and the diff open over the tab, when one is.
#[derive(Debug, Default)]
pub struct State {
    list: Vec<Loaded>,
    /// The rollback whose diff is open: what it would change.
    compare: Option<Prepared>,
    /// Which of the diff's items are ticked, by index into the plan's items. A change starts ticked.
    picked: Vec<bool>,
}

impl State {
    /// Replaces the list with the snapshots the folder holds, newest first.
    pub fn set_list(&mut self, list: Vec<Loaded>) {
        self.list = list;
    }

    /// Opens the diff of a rollback over the tab, with every change ticked.
    pub fn open_compare(&mut self, prepared: Prepared) {
        self.picked = prepared.plan.items.iter().map(|item| item.state != Standing::Same).collect();
        self.compare = Some(prepared);
    }

    /// Closes the diff, if one is open.
    pub fn close_compare(&mut self) {
        self.compare = None;
        self.picked.clear();
    }

    /// The rollback whose diff is open, if one is.
    pub fn compare(&self) -> Option<&Prepared> {
        self.compare.as_ref()
    }

    /// Whether the item at `index` of the open diff is ticked.
    pub fn is_picked(&self, index: usize) -> bool {
        self.picked.get(index).copied().unwrap_or(false)
    }

    /// How many changes of the open diff are ticked, which is what its Roll back button names.
    pub fn picked_count(&self) -> usize {
        let Some(prepared) = self.compare.as_ref() else {
            return 0;
        };
        prepared
            .plan
            .items
            .iter()
            .enumerate()
            .filter(|(index, item)| item.state != Standing::Same && self.is_picked(*index))
            .count()
    }

    /// Ticks or unticks the item at `index` of the open diff. A mod already at the snapshot's release has no box, so it stays
    /// unticked.
    pub fn toggle(&mut self, index: usize) {
        let Some(item) = self.compare.as_ref().and_then(|prepared| prepared.plan.items.get(index)) else {
            return;
        };
        if item.state == Standing::Same {
            return;
        }
        if let Some(picked) = self.picked.get_mut(index) {
            *picked = !*picked;
        }
    }

    /// Closes the diff and takes its rollback with the items ticked in it: each one the rollback should change, in order.
    pub fn take_picked(&mut self) -> Option<(Prepared, Vec<usize>)> {
        let prepared = self.compare.take()?;
        let picked = std::mem::take(&mut self.picked);
        let indices = prepared
            .plan
            .items
            .iter()
            .enumerate()
            .filter(|(index, item)| item.state != Standing::Same && picked.get(*index).copied().unwrap_or(false))
            .map(|(index, _)| index)
            .collect();
        Some((prepared, indices))
    }

    /// The snapshots, newest first.
    pub fn list(&self) -> &[Loaded] {
        &self.list
    }

    /// Whether the instance keeps no snapshot, in which case the tab is not offered.
    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    /// The snapshot whose file stem is `stem`, if the folder holds it.
    pub fn find(&self, stem: &str) -> Option<&Loaded> {
        self.list.iter().find(|loaded| loaded.listed.name.stem() == stem)
    }
}

#[cfg(test)]
#[path = "../../../../../tests/app/pages/instance/snapshots/mod.rs"]
mod tests;
