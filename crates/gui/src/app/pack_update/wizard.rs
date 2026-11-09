//! The migration wizard of an in-place update: the review the user goes through before the update starts. Its first step says what
//! changes, and asks whether to back up the worlds first. Its second lists each change with its decision, and lays out each text
//! conflict in hunks, so the user picks a side for each one. Its last step confirms, and the root starts the update from the
//! decisions the wizard holds.

mod view;

use crate::app::install::update::Decisions;
use crate::app::install::update::merge::Side;
use crate::app::install::update::plan::{Decision, Kind};
use crate::app::install::update::review::Review;
use crate::app::install::update::{Reviewed, UpdateRequest};

/// The steps of the wizard, in order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// What changes, and whether to back up the worlds first.
    Summary,
    /// Each change with its decision, and each conflict's hunks.
    Changes,
    /// The decisions, and the start of the update.
    Confirm,
}

impl Step {
    /// The step after this one. The last step stays where it is.
    fn next(self) -> Self {
        match self {
            Step::Summary => Step::Changes,
            Step::Changes | Step::Confirm => Step::Confirm,
        }
    }

    /// The step before this one. The first step stays where it is.
    fn back(self) -> Self {
        match self {
            Step::Confirm => Step::Changes,
            Step::Changes | Step::Summary => Step::Summary,
        }
    }
}

/// The messages the wizard's view sends. The root handles `Close` and `Apply`, and the rest the wizard handles itself.
#[derive(Debug, Clone)]
pub enum Message {
    /// Moves to the next step.
    Next,
    /// Moves to the step before.
    Back,
    /// Closes the wizard without updating.
    Close,
    /// Starts the update with the decisions the wizard holds.
    Apply,
    /// Sets what happens to the change to `path`.
    Decide { path: String, decision: Decision },
    /// Sets which side the conflict `index` of the text file `path` takes.
    Choose { path: String, index: usize, side: Side },
    /// Sets whether the worlds are backed up before the update.
    SetBackup(bool),
}

/// The wizard's state: the review it shows, the step it is on, and the decisions the user has made so far.
#[derive(Debug)]
pub struct PackWizard {
    review: Review,
    step: Step,
    decisions: Decisions,
    backup_worlds: bool,
}

impl PackWizard {
    /// The wizard for `review`, on its first step, with the review's defaults.
    pub fn new(review: Review) -> Self {
        let backup_worlds = review.request.backup_worlds;
        Self {
            review,
            step: Step::Summary,
            decisions: Decisions::default(),
            backup_worlds,
        }
    }

    /// The step the wizard is on.
    pub fn step(&self) -> Step {
        self.step
    }

    /// The review the wizard shows.
    pub fn review(&self) -> &Review {
        &self.review
    }

    /// The decisions made so far, which a change without one takes its default for.
    pub fn decisions(&self) -> &Decisions {
        &self.decisions
    }

    /// Whether the worlds are backed up before the update.
    pub fn backup_worlds(&self) -> bool {
        self.backup_worlds
    }

    /// Handles a message the root does not handle.
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Next => self.step = self.step.next(),
            Message::Back => self.step = self.step.back(),
            Message::Decide { path, decision } => self.decisions.set(path, decision),
            Message::Choose { path, index, side } => self.decisions.choose(path, index, side),
            Message::SetBackup(backup) => self.backup_worlds = backup,
            Message::Close | Message::Apply => {}
        }
    }

    /// The update the user has decided on: the review's request, with the decisions, the backup choice, and the changes the review
    /// showed. The job checks those changes against its own plan before it writes anything.
    pub fn into_request(self) -> UpdateRequest {
        let Review { request, changes, conflicts } = self.review;
        let hunks = conflicts.iter().map(|conflict| (conflict.path.clone(), conflict.hunks.len())).collect();
        UpdateRequest {
            decisions: self.decisions,
            backup_worlds: self.backup_worlds,
            reviewed: Some(Reviewed { changes, hunks }),
            ..request
        }
    }

    /// How many changes the update applies, and how many it leaves as the instance has them, under the decisions so far. The
    /// instance's own jars are not counted, since the update never touches them.
    pub fn counts(&self) -> (usize, usize) {
        self.review
            .changes
            .iter()
            .filter(|change| change.kind != Kind::Yours)
            .fold((0, 0), |(apply, keep), change| match self.decisions.decide(change) {
                Decision::Take => (apply + 1, keep),
                Decision::Keep => (apply, keep + 1),
            })
    }

    /// The conflict that the text file `path` has, when it has one laid out in hunks.
    pub fn conflict_of(&self, path: &str) -> Option<&crate::app::install::update::review::ConflictView> {
        self.review.conflicts.iter().find(|conflict| conflict.path == path)
    }
}

#[cfg(test)]
#[path = "../../../tests/app/pack_update/wizard.rs"]
mod tests;
