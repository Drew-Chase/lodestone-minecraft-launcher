//! Pages — the launcher's screens and the route table that names them.
//!
//! [`Page`] is the single source of truth for which screen is on display:
//! the route table, expressed as an enum. [`crate::app::router::Router`]
//! owns the history; the root model owns one state struct per page and
//! renders the current one in its outlet (`app/mod.rs`).
//!
//! # Adding a page
//!
//! 1. Create `pages/<name>.rs` with a state struct and a `Message` enum.
//! 2. Add a variant to [`Page`], with `title` and `icon` arms — and to
//!    [`Page::RAIL_PAGES`] when it belongs in the nav rail.
//! 3. Add `Navigate(Page)` to the page's `Message` — the `<Link>` of this
//!    framework. The root intercepts it before the page's `update` runs.
//! 4. Add `Message::PageName(page::Message)` to `LodestoneAppMessage`.
//! 5. In the root `update`, add the interception arm
//!    (`PageName(page::Message::Navigate(page)) => self.navigate(page)`)
//!    and a delegation arm for the rest.
//! 6. Add an arm to the root's `outlet()` so the page renders.

pub mod home;
pub mod login;
pub mod settings;
pub mod showcase;

use aurora_leaf::Icon;

/// Every screen the launcher can display.
///
/// Variants may carry parameters once pages need route arguments (e.g.
/// `InstanceDetail { id: u64 }`); that would shed `Copy` and `Eq` — the
/// router only needs `Clone` and `PartialEq`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Page {
    /// The sign-in gate shown before the primary navigation.
    #[default]
    Login,
    /// Landing page.
    Home,
    /// Launcher preferences.
    Settings,
    /// Style-guide gallery of every core control.
    Showcase,
}

impl Page {
    /// Pages shown in the nav rail, in display order. [`Page::Login`] is
    /// deliberately absent — it renders without the primary navigation.
    pub const RAIL_PAGES: [Page; 3] = [Page::Home, Page::Showcase, Page::Settings];

    /// Human-readable name, for titles and logs.
    pub fn title(self) -> &'static str {
        match self {
            Page::Login => "Login",
            Page::Home => "Home",
            Page::Settings => "Settings",
            Page::Showcase => "Showcase",
        }
    }

    /// Rail icon as SVG markup.
    pub fn icon(self) -> &'static str {
        match self {
            Page::Login => Icon::lucide().user(),
            Page::Home => Icon::lucide().house(),
            Page::Settings => Icon::lucide().settings(),
            Page::Showcase => Icon::lucide().compass(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_page_has_a_title_and_icon() {
        for page in [Page::Login, Page::Home, Page::Settings, Page::Showcase] {
            assert!(!page.title().is_empty(), "{page:?} needs a title");
            assert!(page.icon().contains("<svg"), "{page:?} needs an icon");
        }
    }

    #[test]
    fn the_rail_lists_every_primary_page() {
        for page in [Page::Home, Page::Settings, Page::Showcase] {
            assert!(Page::RAIL_PAGES.contains(&page), "{page:?} is missing from the rail");
        }
    }

    #[test]
    fn login_boots_first_and_stays_out_of_the_rail() {
        assert_eq!(Page::default(), Page::Login, "the app must boot to the login gate");
        assert!(!Page::RAIL_PAGES.contains(&Page::Login), "login renders without primary navigation");
    }
}
