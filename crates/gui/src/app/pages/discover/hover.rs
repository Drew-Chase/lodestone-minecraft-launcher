//! The grid card under the pointer. The page keeps the one project the pointer is over, so the card grows and lights
//! its border the way a library card does.

use super::Discover;

impl Discover {
    /// Records the pointer entering (`hovered`) or leaving the card of `project_id`. An exit clears only the card it
    /// was for, so an enter and an exit can arrive in either order.
    pub(super) fn hover(&mut self, project_id: String, hovered: bool) {
        if hovered {
            self.hovered = Some(project_id);
        } else if self.hovered.as_deref() == Some(project_id.as_str()) {
            self.hovered = None;
        }
    }

    /// Whether the pointer is over the card of `project_id`.
    pub(super) fn is_hovered(&self, project_id: &str) -> bool {
        self.hovered.as_deref() == Some(project_id)
    }
}
