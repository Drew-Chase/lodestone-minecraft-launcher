//! Which rows of the Files tab are selected, and how a click or a key changes that, the way a file manager does: a
//! plain click picks one row, Ctrl adds or removes a row, and Shift picks the range from the row picked last.

use std::collections::BTreeSet;

/// The keys held during a click or an arrow key.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Held {
    /// Ctrl, or Command on macOS.
    pub command: bool,
    pub shift: bool,
}

/// The selected rows, by path, and the row a Shift range starts from.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Selection {
    picked: BTreeSet<String>,
    /// The row picked last without Shift. A Shift click or arrow selects from here.
    anchor: Option<String>,
    /// The row the keyboard moves from: the one picked or reached last.
    lead: Option<String>,
}

impl Selection {
    /// Whether the row at `path` is selected.
    pub fn contains(&self, path: &str) -> bool {
        self.picked.contains(path)
    }

    /// How many rows are selected.
    pub fn len(&self) -> usize {
        self.picked.len()
    }

    pub fn is_empty(&self) -> bool {
        self.picked.is_empty()
    }

    /// The selected paths, in name order.
    pub fn paths(&self) -> Vec<String> {
        self.picked.iter().cloned().collect()
    }

    /// The only selected row, when exactly one is.
    pub fn single(&self) -> Option<&str> {
        match self.picked.len() {
            1 => self.picked.iter().next().map(String::as_str),
            _ => None,
        }
    }

    /// Selects nothing.
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Selects the row at `path` alone.
    pub fn only(&mut self, path: &str) {
        self.picked = BTreeSet::from([path.to_string()]);
        self.anchor = Some(path.to_string());
        self.lead = Some(path.to_string());
    }

    /// Selects every row in `order`.
    pub fn all(&mut self, order: &[&str]) {
        self.picked = order.iter().map(|path| (*path).to_string()).collect();
        self.anchor = order.first().map(|path| (*path).to_string());
        self.lead = order.last().map(|path| (*path).to_string());
    }

    /// Applies a click on the row at `path`. `order` is the rows as the list draws them, which a Shift range follows.
    pub fn click(&mut self, order: &[&str], path: &str, held: Held) {
        if held.shift {
            self.range_to(order, path, held.command);
        } else if held.command {
            if !self.picked.remove(path) {
                self.picked.insert(path.to_string());
            }
            self.anchor = Some(path.to_string());
            self.lead = Some(path.to_string());
        } else {
            self.only(path);
        }
    }

    /// Moves the keyboard's row by `delta` rows of `order`, stopping at either end. Shift selects the range from the
    /// anchor to the row reached, and without it the row reached is selected alone. With nothing selected, a step down
    /// starts at the first row and a step up at the last.
    pub fn step(&mut self, order: &[&str], delta: isize, shift: bool) {
        if order.is_empty() {
            return;
        }
        let from = self.lead.as_deref().and_then(|lead| order.iter().position(|path| *path == lead));
        let last = order.len() - 1;
        let to = match from {
            Some(index) => index.saturating_add_signed(delta).min(last),
            None if delta < 0 => last,
            None => 0,
        };
        if shift && from.is_some() {
            self.range_to(order, order[to], false);
        } else {
            self.only(order[to]);
        }
    }

    /// The row the keyboard last reached, which the list keeps on screen.
    pub fn lead(&self) -> Option<&str> {
        self.lead.as_deref()
    }

    /// Forgets the selected rows that `exists` no longer holds, after the folder was read again.
    pub fn retain(&mut self, exists: impl Fn(&str) -> bool) {
        self.picked.retain(|path| exists(path));
        if self.anchor.as_deref().is_some_and(|path| !exists(path)) {
            self.anchor = None;
        }
        if self.lead.as_deref().is_some_and(|path| !exists(path)) {
            self.lead = None;
        }
    }

    /// Selects the rows from the anchor to `path`, in `order`. With `keep` the rows already selected stay selected.
    fn range_to(&mut self, order: &[&str], path: &str, keep: bool) {
        let end = order.iter().position(|row| *row == path);
        let start = self.anchor.as_deref().and_then(|anchor| order.iter().position(|row| *row == anchor));
        let (Some(start), Some(end)) = (start.or(end), end) else {
            return;
        };
        if !keep {
            self.picked.clear();
        }
        let (low, high) = (start.min(end), start.max(end));
        self.picked.extend(order[low..=high].iter().map(|row| (*row).to_string()));
        if self.anchor.is_none() {
            self.anchor = Some(path.to_string());
        }
        self.lead = Some(path.to_string());
    }
}

#[cfg(test)]
#[path = "../../../../../tests/app/pages/instance/browser/selection.rs"]
mod tests;
