//! Page router — browser-style history over [`Page`] values.
//!
//! Mirrors react-router's history semantics: [`Router::push`] navigates and
//! discards the forward stack, [`Router::replace`] swaps the current entry in
//! place, and back/forward walk the stacks without erasing them.

use crate::app::pages::Page;

/// Navigation history for the launcher's pages.
///
/// Lives on the root model; pages never touch it directly — they request
/// navigation through their `Navigate(Page)` message (see the contract in
/// `pages/mod.rs`).
#[derive(Debug, Clone)]
pub struct Router {
    current: Page,
    back: Vec<Page>,
    forward: Vec<Page>,
}

impl Router {
    /// Creates a router rooted at `initial`.
    pub fn new(initial: Page) -> Self {
        Self {
            current: initial,
            back: Vec::new(),
            forward: Vec::new(),
        }
    }

    /// Navigates to `page`: the current page moves onto the back stack and
    /// the forward stack is discarded.
    ///
    /// Navigating to the page already on screen is a no-op returning
    /// `false`, so repeated rail clicks don't spam the history.
    pub fn push(&mut self, page: Page) -> bool {
        if page == self.current {
            return false;
        }
        self.back.push(self.current);
        self.forward.clear();
        self.current = page;
        true
    }

    /// Swaps the current entry for `page` without touching either stack —
    /// the redirect of this framework.
    pub fn replace(&mut self, page: Page) {
        self.current = page;
    }

    /// Steps back one entry, returning `true` when the page changed.
    pub fn go_back(&mut self) -> bool {
        self.go_back_where(|_| true)
    }

    /// Steps back to the nearest entry that `accepts`, passing over the entries it refuses. Those stay in the history, so a
    /// forward step can reach them. Returns `false`, and leaves the history as it is, when no entry back is accepted.
    pub fn go_back_where(&mut self, accepts: impl Fn(Page) -> bool) -> bool {
        let history = self.history();
        let here = self.back.len();
        match (0..here).rev().find(|&index| accepts(history[index])) {
            Some(index) => {
                self.seat(&history, index);
                true
            }
            None => false,
        }
    }

    /// Steps forward to the nearest entry that `accepts`, passing over the entries it refuses. Returns `false`, and leaves the
    /// history as it is, when no entry ahead is accepted.
    pub fn go_forward_where(&mut self, accepts: impl Fn(Page) -> bool) -> bool {
        let history = self.history();
        let here = self.back.len();
        match (here + 1..history.len()).find(|&index| accepts(history[index])) {
            Some(index) => {
                self.seat(&history, index);
                true
            }
            None => false,
        }
    }

    /// Replaces the current entry with `page`, and drops the entry below it when that is `page` as well, so the page a sign-in
    /// was started from is not kept twice. A sign-in that returns to the page it came from leaves no sign-in page to step back
    /// to.
    pub fn leave(&mut self, page: Page) {
        if self.back.last() == Some(&page) {
            self.back.pop();
        }
        self.current = page;
    }

    /// The whole history in order: the back entries, the current page, then the forward entries nearest first.
    fn history(&self) -> Vec<Page> {
        let mut history = self.back.clone();
        history.push(self.current);
        history.extend(self.forward.iter().rev().copied());
        history
    }

    /// Makes `history[index]` the current page. The entries before it become the back stack, and the ones after it the forward
    /// stack.
    fn seat(&mut self, history: &[Page], index: usize) {
        self.back = history[..index].to_vec();
        self.current = history[index];
        self.forward = history[index + 1..].iter().rev().copied().collect();
    }

    /// The page currently on screen.
    pub fn current(&self) -> Page {
        self.current
    }

    /// Whether a back step is possible.
    pub fn can_go_back(&self) -> bool {
        !self.back.is_empty()
    }

    /// Whether a forward step is possible.
    pub fn can_go_forward(&self) -> bool {
        !self.forward.is_empty()
    }

    /// The page a back step returns to, when there is one.
    pub fn back_page(&self) -> Option<Page> {
        self.back.last().copied()
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new(Page::default())
    }
}

#[cfg(test)]
#[path = "../../tests/app/router.rs"]
mod tests;
