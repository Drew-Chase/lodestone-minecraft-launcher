//! Undo and redo for the editor. iced's text editor keeps no history, so the editor keeps the text as it was before
//! each change, with where the caret was.
//!
//! Typing a word is one step, not one step a letter: an edit of the same kind as the last one, made soon after it,
//! joins its step. The history is capped by how much text it holds, and its oldest steps go first.

use std::time::{Duration, Instant};

use iced::widget::text_editor::Cursor;

/// How soon after the last edit one of the same kind joins its step.
const BURST: Duration = Duration::from_millis(900);
/// The most text the history holds, in bytes, across all its steps.
const MAX_BYTES: usize = 24 << 20;

/// The text at one moment, where the caret was, and the number that names this version of the text.
#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot {
    pub text: String,
    pub cursor: Cursor,
    /// The version of the text. The editor compares it with the version it last saved to tell whether the text changed.
    pub revision: u64,
}

/// What kind of change an edit is. Edits of one kind made close together are one step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Typing characters.
    Typing,
    /// Deleting with Backspace or Delete.
    Deleting,
    /// Anything else: a paste, a new line, an indent. It is always a step of its own.
    Other,
}

/// The steps that can be undone and redone.
#[derive(Debug, Default)]
pub struct History {
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    /// The kind of the last edit and when it was made.
    last: Option<(Kind, Instant)>,
}

impl History {
    /// Whether an edit of `kind` made at `now` starts a new step. The editor then hands the text as it is, before the
    /// edit, to [`History::push`]. An edit that joins the step before it needs nothing kept.
    pub fn begins_step(&mut self, kind: Kind, now: Instant) -> bool {
        let joins = kind != Kind::Other
            && self
                .last
                .is_some_and(|(last, at)| last == kind && now.saturating_duration_since(at) < BURST);
        self.last = Some((kind, now));
        !joins
    }

    /// Keeps `before`, the text as it was before the edit that starts a new step. Whatever was undone can no longer be
    /// redone.
    pub fn push(&mut self, before: Snapshot) {
        self.redo.clear();
        self.undo.push(before);
        self.trim();
    }

    /// Steps back: returns the text to show, and keeps `current` to redo. `None` when nothing is left to undo.
    pub fn undo(&mut self, current: Snapshot) -> Option<Snapshot> {
        let earlier = self.undo.pop()?;
        self.redo.push(current);
        self.last = None;
        Some(earlier)
    }

    /// Steps forward again: returns the text to show, and keeps `current` to undo. `None` when nothing was undone.
    pub fn redo(&mut self, current: Snapshot) -> Option<Snapshot> {
        let later = self.redo.pop()?;
        self.undo.push(current);
        self.last = None;
        Some(later)
    }

    /// Ends the step being typed, so the next edit starts one of its own. A move of the caret does this.
    pub fn break_step(&mut self) {
        self.last = None;
    }

    /// Drops the oldest steps until the history holds no more than [`MAX_BYTES`] of text. The newest step always stays.
    fn trim(&mut self) {
        let mut held: usize = self.undo.iter().map(|snapshot| snapshot.text.len()).sum();
        let mut drop = 0;
        while held > MAX_BYTES && drop + 1 < self.undo.len() {
            held -= self.undo[drop].text.len();
            drop += 1;
        }
        self.undo.drain(..drop);
    }
}

#[cfg(test)]
#[path = "../../../../tests/app/components/editor/history.rs"]
mod tests;
