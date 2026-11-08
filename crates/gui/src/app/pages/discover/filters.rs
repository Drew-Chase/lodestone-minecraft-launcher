//! The sidebar's filters: the picks a search asks the platform for, the messages the sidebar sends, and the panel's own
//! state, which no search depends on.

use std::collections::HashSet;

use lodestone_platforms::{ContentType, SearchQuery, Side};

use super::{Discover, Effect};

/// The picks a search asks the platform for. Each list is kept sorted and free of repeats, so the same picks always
/// make the same search key.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct Filters {
    /// Categories a hit must all be filed under.
    pub categories: Vec<String>,
    /// Loaders a hit may run on: any of them.
    pub loaders: Vec<String>,
    /// Minecraft versions a hit may support: any of them.
    pub game_versions: Vec<String>,
    /// Sides a hit must run on: each of them.
    pub environments: Vec<Side>,
    /// Whether a hit must be open source.
    pub open_source: bool,
    /// Categories a hit must not be filed under.
    pub excluded_categories: Vec<String>,
    /// Loaders a hit must not run on.
    pub excluded_loaders: Vec<String>,
}

impl Filters {
    /// Whether nothing is picked.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// Puts the picks into `query`, the search about to be asked.
    pub fn apply(&self, query: &mut SearchQuery) {
        query.categories.clone_from(&self.categories);
        query.loaders.clone_from(&self.loaders);
        query.game_versions.clone_from(&self.game_versions);
        query.environments.clone_from(&self.environments);
        query.open_source = self.open_source;
        query.excluded_categories.clone_from(&self.excluded_categories);
        query.excluded_loaders.clone_from(&self.excluded_loaders);
    }

    /// Sorts each list and drops repeats, so the same picks always make the same key.
    pub(super) fn tidy(&mut self) {
        for list in [
            &mut self.categories,
            &mut self.loaders,
            &mut self.game_versions,
            &mut self.excluded_categories,
            &mut self.excluded_loaders,
        ] {
            list.sort();
            list.dedup();
        }
        self.environments.sort_by_key(|side| matches!(side, Side::Server));
        self.environments.dedup();
    }
}

/// Which list a sidebar row picks from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Category,
    Loader,
    GameVersion,
}

/// What the sidebar sends.
#[derive(Debug, Clone)]
pub enum FilterMessage {
    /// A category, loader or Minecraft version was picked, or dropped.
    Pick(Kind, String),
    /// A side was picked, or dropped.
    Environment(Side),
    /// The open-source switch was set or cleared.
    OpenSource(bool),
    /// A category or a loader was picked for exclusion, or dropped.
    Exclude(Kind, String),
    /// The Minecraft version list was asked to show snapshots and the other versions too, or not.
    ShowAllVersions(bool),
    /// The version search changed.
    VersionSearch(String),
    /// A section was asked to show every item, or only the first few. `section` names it.
    Expand { section: String, expanded: bool },
    /// Every pick was cleared.
    Clear,
}

/// The sidebar's state that no search depends on: the version search, the snapshot switch, and the sections that show
/// every item.
#[derive(Debug, Clone, Default)]
pub struct Panel {
    pub(super) show_all_versions: bool,
    pub(super) version_search: String,
    pub(super) expanded: HashSet<String>,
}

/// Picks `item` in `list`, or drops it when it is picked already. A platform that takes one value per filter keeps only
/// the pick made last.
fn toggle<T: PartialEq>(list: &mut Vec<T>, item: T, multiple: bool) {
    match list.iter().position(|picked| *picked == item) {
        Some(index) => {
            list.remove(index);
        }
        None => {
            if !multiple {
                list.clear();
            }
            list.push(item);
        }
    }
}

impl Discover {
    /// Carries out a sidebar message. A pick asks the platform again at once. The panel's own changes ask nothing.
    pub(super) fn filter_changed(&mut self, message: FilterMessage) -> Vec<Effect> {
        let multiple = self.facets().is_none_or(|facets| facets.multiple);
        let searches = match message {
            FilterMessage::Pick(kind, id) => {
                toggle(self.list_mut(kind), id.clone(), multiple);
                // A pick includes the item, so it cannot also be excluded.
                if self.list_mut(kind).contains(&id) {
                    self.drop_exclusion(kind, &id);
                }
                true
            }
            FilterMessage::Environment(side) => {
                toggle(&mut self.filters.environments, side, true);
                true
            }
            FilterMessage::OpenSource(on) => {
                self.filters.open_source = on;
                true
            }
            FilterMessage::Exclude(kind, id) => {
                toggle(self.excluded_list_mut(kind), id.clone(), true);
                // An exclusion drops the pick of the same item, so it cannot also be included.
                if self.excluded_list_mut(kind).contains(&id) {
                    self.drop_pick(kind, &id);
                }
                true
            }
            FilterMessage::Clear => {
                self.filters = Default::default();
                self.apply_target_filters();
                true
            }
            FilterMessage::ShowAllVersions(on) => {
                self.panel.show_all_versions = on;
                false
            }
            FilterMessage::VersionSearch(text) => {
                self.panel.version_search = text;
                false
            }
            FilterMessage::Expand { section, expanded } => {
                if expanded {
                    self.panel.expanded.insert(section);
                } else {
                    self.panel.expanded.remove(&section);
                }
                false
            }
        };
        if !searches {
            return Vec::new();
        }
        self.filters.tidy();
        self.start(false)
    }

    /// The list a pick of `kind` changes.
    fn list_mut(&mut self, kind: Kind) -> &mut Vec<String> {
        match kind {
            Kind::Category => &mut self.filters.categories,
            Kind::Loader => &mut self.filters.loaders,
            Kind::GameVersion => &mut self.filters.game_versions,
        }
    }

    /// The exclusion list a category or loader is put in. A Minecraft version is never excluded.
    fn excluded_list_mut(&mut self, kind: Kind) -> &mut Vec<String> {
        match kind {
            Kind::Loader => &mut self.filters.excluded_loaders,
            _ => &mut self.filters.excluded_categories,
        }
    }

    /// Drops `id` from the exclusions of `kind`, so an item is never both picked and excluded.
    fn drop_exclusion(&mut self, kind: Kind, id: &str) {
        if kind != Kind::GameVersion {
            self.excluded_list_mut(kind).retain(|excluded| excluded.as_str() != id);
        }
    }

    /// Drops `id` from the picks of `kind`, so an item is never both picked and excluded.
    fn drop_pick(&mut self, kind: Kind, id: &str) {
        if kind != Kind::GameVersion {
            self.list_mut(kind).retain(|picked| picked.as_str() != id);
        }
    }

    /// Whether the tab on screen takes a Minecraft version from its target instance. Modpacks are not filtered by one.
    pub(super) fn takes_target_version(&self) -> bool {
        self.content != ContentType::Modpack
    }
}

#[cfg(test)]
#[path = "../../../../tests/app/pages/discover/filters.rs"]
mod tests;
