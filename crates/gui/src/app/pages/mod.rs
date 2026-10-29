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
//! 2. Add a variant to [`Page`] with a `title` arm.
//! 3. Add `Navigate(Page)` to the page's `Message` — the `<Link>` of this
//!    framework. The root intercepts it before the page's `update` runs.
//! 4. Add `Message::PageName(page::Message)` to `LodestoneAppMessage`.
//! 5. In the root `update`, add the interception arm
//!    (`PageName(page::Message::Navigate(page)) => self.navigate(page)`)
//!    and a delegation arm for the rest.
//! 6. Add an arm to the root's `outlet()` so the page renders.

pub mod instance;
pub mod library;
pub mod login;
pub mod settings;
pub mod showcase;

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
    /// Landing page — the instance Library.
    Home,
    /// A single instance's detail view. The instance itself lives in the
    /// root's `InstancePage`; the route stays parameterless so the router
    /// keeps `Copy`/`Eq` history semantics.
    Instance,
    /// Launcher preferences.
    Settings,
    /// Style-guide gallery of every core control.
    Showcase,
}

impl Page {
    /// Human-readable name, for titles and logs.
    pub fn title(self) -> &'static str {
        match self {
            Page::Login => "Login",
            Page::Home => "Library",
            Page::Instance => "Instance",
            Page::Settings => "Settings",
            Page::Showcase => "Showcase",
        }
    }
}

#[cfg(test)]
#[path = "../../../tests/app/pages/mod.rs"]
mod tests;
