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

pub mod discover;
pub mod friends;
pub mod instance;
pub mod library;
pub mod login;
pub mod news;
pub mod resource;
pub mod servers;
pub mod settings;
pub mod skins;
pub mod worlds;

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
    /// The instance Library, the rail's book and the startup choice of the same name.
    Library,
    /// The Minecraft news home, opened by the Lodestone logo. It lists Mojang's posts and patch notes.
    News,
    /// The content browser: modpacks, resource packs, shaders, worlds and data packs from the
    /// platforms.
    Discover,
    /// A single instance's detail view. The instance itself lives in the
    /// root's `InstancePage`; the route stays parameterless so the router
    /// keeps `Copy`/`Eq` history semantics.
    Instance,
    /// A project's resource page, opened from Discover. The project lives in the root's `ResourcePage`, as the
    /// instance page's does.
    Resource,
    /// Every saved world across every instance.
    Worlds,
    /// The player's skin and cape, the skins to choose from, and the capes the player owns.
    Skins,
    /// The servers the player saved, their live pings, and the LAN games announced on the network.
    Servers,
    /// The friends page. Friends are coming soon, so the page shows a banner in place of them.
    Friends,
    /// Launcher preferences.
    Settings,
}

impl Page {
    /// Human-readable name, for titles and logs.
    pub fn title(self) -> &'static str {
        match self {
            Page::Login => "Login",
            Page::Library => "Library",
            Page::News => "News",
            Page::Discover => "Discover",
            Page::Worlds => "Worlds",
            Page::Skins => "Skins",
            Page::Servers => "Servers",
            Page::Friends => "Friends",
            Page::Instance => "Instance",
            Page::Resource => "Resource",
            Page::Settings => "Settings",
        }
    }
}

#[cfg(test)]
#[path = "../../../tests/app/pages/mod.rs"]
mod tests;
