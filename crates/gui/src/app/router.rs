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
mod tests {
    use super::*;

    #[test]
    fn push_moves_current_and_clears_forward() {
        let mut router = Router::new(Page::Home);
        router.push(Page::Showcase);
        router.push(Page::Settings);
        assert_eq!(router.current(), Page::Settings);
        assert!(router.can_go_back());

        router.go_back();
        assert!(router.can_go_forward());

        router.push(Page::Home);
        assert!(!router.can_go_forward(), "push must discard the forward stack");
    }

    #[test]
    fn pushing_the_current_page_is_a_noop() {
        let mut router = Router::new(Page::Home);
        assert!(!router.push(Page::Home));
        assert!(!router.can_go_back());
        assert_eq!(router.current(), Page::Home);
    }

    #[test]
    fn back_and_forward_walk_the_stacks() {
        let mut router = Router::new(Page::Home);
        router.push(Page::Showcase);
        router.push(Page::Settings);

        assert!(router.go_back());
        assert_eq!(router.current(), Page::Showcase);
        assert!(router.go_back());
        assert_eq!(router.current(), Page::Home);
        assert!(!router.go_back(), "back stops at the first entry");

        assert!(router.go_forward());
        assert_eq!(router.current(), Page::Showcase);
        assert!(router.go_forward());
        assert_eq!(router.current(), Page::Settings);
        assert!(!router.go_forward(), "forward stops at the last entry");
    }

    #[test]
    fn replace_swaps_in_place() {
        let mut router = Router::new(Page::Home);
        router.push(Page::Showcase);
        router.replace(Page::Settings);

        assert_eq!(router.current(), Page::Settings);
        assert!(router.can_go_back());
        assert!(router.go_back());
        assert_eq!(router.current(), Page::Home, "replace must keep the back stack");
        assert!(router.can_go_forward(), "the replaced entry stays reachable via forward");
        assert!(router.go_forward());
        assert_eq!(router.current(), Page::Settings);
    }

    #[test]
    fn can_go_flags_track_the_stacks() {
        let mut router = Router::new(Page::Home);
        assert!(!router.can_go_back());
        assert!(!router.can_go_forward());

        router.push(Page::Showcase);
        assert!(router.can_go_back());
        assert!(!router.can_go_forward());
    }
}
