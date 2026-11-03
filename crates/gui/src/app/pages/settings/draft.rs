//! The Settings page's draft: the values as they are saved, and the copy the tabs edit.

use crate::app::preferences::SettingsValues;

/// The saved values, and the copy the page edits. Save changes makes the copy the saved values, and Reset copies the
/// saved values back over the copy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Draft {
    saved: SettingsValues,
    current: SettingsValues,
}

impl Draft {
    /// A draft of `saved`, with no edits.
    pub fn new(saved: SettingsValues) -> Self {
        Self {
            current: saved.clone(),
            saved,
        }
    }

    /// Takes `saved` as the saved values. The copy follows them unless the page has edits in progress, which stay.
    pub fn reload(&mut self, saved: &SettingsValues) {
        if !self.dirty() {
            self.current = saved.clone();
        }
        self.saved = saved.clone();
    }

    /// The values as they are saved.
    pub fn saved(&self) -> &SettingsValues {
        &self.saved
    }

    /// The copy the tabs show.
    pub fn current(&self) -> &SettingsValues {
        &self.current
    }

    /// The copy, for an edit to write into.
    pub fn current_mut(&mut self) -> &mut SettingsValues {
        &mut self.current
    }

    /// Whether the copy differs from the saved values.
    pub fn dirty(&self) -> bool {
        self.current != self.saved
    }

    /// Throws the edits away, so the copy shows the saved values again.
    pub fn reset(&mut self) {
        self.current = self.saved.clone();
    }

    /// Makes the copy the saved values, and returns them for the owner to store.
    pub fn commit(&mut self) -> SettingsValues {
        self.saved = self.current.clone();
        self.saved.clone()
    }
}

#[cfg(test)]
#[path = "../../../../tests/app/pages/settings/draft.rs"]
mod tests;
