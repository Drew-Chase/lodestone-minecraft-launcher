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
        match self.back.pop() {
            Some(page) => {
                self.forward.push(self.current);
                self.current = page;
                true
            }
            None => false,
        }
    }

    /// Steps forward one entry, returning `true` when the page changed.
    pub fn go_forward(&mut self) -> bool {
        match self.forward.pop() {
            Some(page) => {
                self.back.push(self.current);
                self.current = page;
                true
            }
            None => false,
        }
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
}

impl Default for Router {
    fn default() -> Self {
        Self::new(Page::default())
    }
}

#[cfg(test)]
#[path = "../../tests/app/router.rs"]
mod tests;
