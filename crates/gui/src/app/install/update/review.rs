//! The review of an update: what it would do, laid out before the user decides. The release is staged and planned as the update would plan
//! it, and each text file that changes on both sides is laid out in hunks, so the user can choose a side for each one. The staged files
//! stay in the workspace, where the update the user applies finds them. The instance itself is not changed.

use sipper::Sender as ProgressSender;
use sipper::{Sipper, sipper};

use super::apply::plan_release;
use super::journal::Workspace;
use super::merge::{self, Piece};
use super::plan::{Change, Decision, Kind};
use super::{UpdateRequest, stage_release, workspace_of};
use crate::app::database::Database;
use crate::app::install::shipped::is_text;
use crate::app::install::{InstallProgress, Stage, report};

/// One place a text file changes on both sides, as the review lays it out: the lines the user's file has there, and the lines the
/// release ships there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hunk {
    /// The lines the user's file has here.
    pub yours: Vec<String>,
    /// The lines the release ships here.
    pub theirs: Vec<String>,
}

/// A text file that changes on both sides, with its hunks in the order they fall in the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictView {
    /// The file's path, relative to the instance folder.
    pub path: String,
    pub hunks: Vec<Hunk>,
}

/// What an update would do, for the user to decide. `request` is the update the user applies, `changes` is the plan with its default
/// decisions, and `conflicts` lays out the text conflicts in hunks.
#[derive(Debug, Clone)]
pub struct Review {
    pub request: UpdateRequest,
    pub changes: Vec<Change>,
    pub conflicts: Vec<ConflictView>,
}

/// Reviews `request` the way its update would run, without changing the instance. The sipper reports progress, and resolves to the
/// review or to the reason it could not be made.
pub(crate) fn review(request: UpdateRequest, db: Database) -> impl Sipper<Result<Review, String>, InstallProgress> {
    sipper(move |mut progress: ProgressSender<InstallProgress>| async move {
        let ws = workspace_of(&request.paths, &request.instance);
        ws.create()?;
        let client = lodestone_net::client();
        stage_release(&mut progress, &request, &ws, &client).await?;
        report(&mut progress, Stage::Plan, None, Stage::Plan.label().to_string()).await;
        let planned = plan_release(&request, &db, &ws).await?;
        let conflicts = conflicts_of(&ws, &planned.changes).await?;
        let changes = planned.changes.into_iter().map(|(change, _)| change).collect();
        Ok(Review { request, changes, conflicts })
    })
}

/// The hunks of each text file in the plan that changes on both sides. A binary file has none, since it is decided whole.
pub(super) async fn conflicts_of(ws: &Workspace, changes: &[(Change, Decision)]) -> Result<Vec<ConflictView>, String> {
    let paths: Vec<String> = changes
        .iter()
        .filter(|(change, _)| change.kind == Kind::Conflict)
        .map(|(change, _)| change.path.clone())
        .collect();
    let instance = ws.instance().to_path_buf();
    let staging = ws.staging();
    let base = ws.base().to_path_buf();
    tokio::task::spawn_blocking(move || {
        paths
            .into_iter()
            .filter_map(|path| hunks_of(&instance.join(&path), &base.join(&path), &staging.join(&path)).map(|hunks| ConflictView { path, hunks }))
            .collect()
    })
    .await
    .map_err(|error| error.to_string())
}

/// The hunks of the text file `yours` against `theirs`, merged against the pristine copy at `base` when there is one. `None` when the
/// file is binary, or unreadable, or has no hunk.
fn hunks_of(yours: &std::path::Path, base: &std::path::Path, theirs: &std::path::Path) -> Option<Vec<Hunk>> {
    let yours = std::fs::read(yours).ok()?;
    let theirs = std::fs::read(theirs).ok()?;
    let base = std::fs::read(base).ok().filter(|bytes| is_text(bytes));
    if !is_text(&yours) || !is_text(&theirs) {
        return None;
    }
    let yours = std::str::from_utf8(&yours).ok()?;
    let theirs = std::str::from_utf8(&theirs).ok()?;
    let pieces = match base.as_deref() {
        Some(base) => merge::merge(std::str::from_utf8(base).ok()?, yours, theirs),
        None => merge::compare(yours, theirs),
    };
    let hunks: Vec<Hunk> = pieces
        .iter()
        .filter_map(|piece| match piece {
            Piece::Conflict { yours, theirs } => Some(Hunk {
                yours: yours.iter().map(|line| line.to_string()).collect(),
                theirs: theirs.iter().map(|line| line.to_string()).collect(),
            }),
            Piece::Lines(_) => None,
        })
        .collect();
    (!hunks.is_empty()).then_some(hunks)
}
