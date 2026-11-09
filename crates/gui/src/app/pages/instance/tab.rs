//! The instance page's tabs: which they are, their labels, and which of them an instance shows.

use crate::app::lang;
use crate::app::pages::library::instance::Instance;

/// The page's tabs, in tab-bar order. The page opens on the Content tab, which holds the instance's mods and its packs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Content,
    /// The releases of the modpack the instance came from. Shown only for an instance that came from a platform.
    Versions,
    Worlds,
    Screenshots,
    Logs,
    Settings,
}

impl Tab {
    pub const ALL: [Tab; 6] = [Tab::Content, Tab::Versions, Tab::Worlds, Tab::Screenshots, Tab::Logs, Tab::Settings];

    pub fn label(self) -> &'static str {
        match self {
            Tab::Content => lang::t("instance.tab.content"),
            Tab::Versions => lang::t("instance.tab.versions"),
            Tab::Worlds => lang::t("instance.tab.worlds"),
            Tab::Screenshots => lang::t("instance.tab.screenshots"),
            Tab::Logs => lang::t("instance.tab.logs"),
            Tab::Settings => lang::t("instance.tab.settings"),
        }
    }

    /// The tabs an instance shows, in tab-bar order. The Versions tab is left out of an instance that did not come from a platform,
    /// since it has no releases to show.
    pub fn visible(instance: Option<&Instance>) -> Vec<Tab> {
        let platform = instance.is_some_and(|instance| instance.source_platform.is_some());
        Tab::ALL.into_iter().filter(|tab| platform || *tab != Tab::Versions).collect()
    }
}
