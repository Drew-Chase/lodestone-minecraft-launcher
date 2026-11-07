//! The Discover page: one platform's modpacks, resource packs, shaders, worlds
//! and data packs, searched and browsed in grid, compact or table layouts. A
//! card can install a modpack, or download any other project into the shared
//! folders.
//!
//! The page decides what to ask for and keeps what came back. The root turns
//! the [`Effect`]s an update returns into tasks, and sends the answers back as
//! messages.

mod card;
mod filters;
mod header;
mod hero;
mod hover;
pub(crate) mod images;
mod place;
mod query;
mod results;
mod sidebar;
mod source;
mod target;
mod toolbar;
mod types;

use std::collections::{HashMap, HashSet, VecDeque};

use crate::app::components::scrollbar;
use crate::app::lang;
use crate::app::pages::resource::Listing;
use crate::app::shared_library::{self, AddedContent, ModSource, SharedRecord};
use iced::widget::{Space, column, container, row, stack};
use iced::{Element, Fill, Padding};
use lodestone_platforms::{ContentType, Facets, SearchPage, SearchSort};

pub use card::{date_only, scene_for, short_count};
pub use images::{Picture, load as load_icon, load_image, pack_art};
pub use source::Source;
pub use target::{AddRequest, InstanceChoice, WorldChoice, takes_target};

use types::Artwork;
pub use types::{
    Destination, DiscoverView, Effect, InstallState, Message, PlaceOutcome, PlaceRequest, Placement, Project, ProjectStatus, Search, TABS,
};

/// The id of the search field, which the root focuses for Ctrl/⌘K.
pub const SEARCH_ID: &str = "discover-search";

/// The id of the scrolling part of the page, below the header. The root scrolls it back to the top when
/// a new search starts.
pub const CONTENT_ID: &str = "discover-content";

/// How close to the end of the results, in pixels, the page asks for the next one.
const LOAD_AHEAD: f32 = 900.0;

/// How many pictures load at once. A page of results asks for all its pictures together, and decoding them as one burst
/// is what stalls the window when the tabs change, so the rest wait for a place.
const ICONS_AT_ONCE: usize = 4;

/// The space between the page's blocks, and above the first of them. A pinned filter row keeps the same gap above it.
const GAP: f32 = 18.0;

/// The margin either side of the page, shared by the header and the scrolling content.
const SIDE: f32 = 28.0;

/// The Discover page's state.
pub struct Discover {
    pub(super) source: Source,
    pub(super) content: ContentType,
    pub(super) text: String,
    /// The search field's placeholder, which names the platform.
    pub(super) placeholder: String,
    pub(super) sort: SearchSort,
    /// The picks the sidebar makes, which every search asks the platform for.
    pub(super) filters: filters::Filters,
    /// What the sidebar shows beyond the picks.
    pub(super) panel: filters::Panel,
    pub(super) view: DiscoverView,
    pub(super) results: query::Results,
    /// Bumped with every change to the search, so answers to older searches are dropped.
    generation: u64,
    facets: HashMap<(Source, ContentType), Facets>,
    cache: query::Cache,
    icons: HashMap<String, Picture>,
    /// The picture addresses that could not be had, so the scene stands in for them.
    failed: HashSet<String>,
    /// The icon addresses already asked for, loaded or not, so none is asked for twice.
    requested: HashSet<String>,
    /// The pictures the results on screen need, waiting for a place among the loads in flight.
    icon_queue: VecDeque<String>,
    /// How many pictures are loading now.
    icons_loading: usize,
    /// How far the content runs past the bottom of the window, as of the last scroll.
    remaining: f32,
    /// How far the content has scrolled down, as of the last scroll. The filter row pins once it reaches the top.
    scroll_y: f32,
    /// The downloads on disk, by (shared folder, platform key, project id), with the record of the release each holds.
    library: HashMap<(String, String, String), SharedRecord>,
    /// The installs the page knows of, by (platform key, project id).
    installs: HashMap<(String, String), InstallState>,
    /// The downloads still running, by (shared folder, platform key, project id).
    busy: HashSet<(String, String, String)>,
    /// The instances a mod can be added to, in the order the library lists them.
    instances: Vec<InstanceChoice>,
    /// The instance the Mods tab adds to, by id, once one is picked.
    target: Option<String>,
    /// The mods already added to an instance, by (instance id, platform key, project id), with the record of the release.
    added: HashMap<(String, String, String), ModSource>,
    /// The mods being added or replaced now, by the same key.
    adding: HashSet<(String, String, String)>,
    /// The worlds of the target instance, for the world picker of the datapack section.
    worlds: Vec<WorldChoice>,
    /// The world the datapacks go to, by its folder name, once one is picked.
    target_world: Option<String>,
    /// The projects placed into an instance or world, by place, with the record of what was placed.
    placed: HashMap<Placement, AddedContent>,
    /// The projects being placed now, by place.
    placing: HashSet<Placement>,
    /// The project whose grid card the pointer is over, if any.
    hovered: Option<String>,
    /// The instance the page was opened from to add mods to. The page shows a way back to it while this is set.
    origin: Option<InstanceChoice>,
}

impl Discover {
    /// The page with `source` selected and nothing searched yet.
    pub fn new(source: Source) -> Self {
        Self {
            source,
            content: ContentType::Modpack,
            text: String::new(),
            placeholder: placeholder(source),
            sort: source.sorts()[0],
            filters: filters::Filters::default(),
            panel: filters::Panel::default(),
            view: DiscoverView::default(),
            results: query::Results::default(),
            generation: 0,
            facets: HashMap::new(),
            cache: query::Cache::default(),
            icons: HashMap::new(),
            failed: HashSet::new(),
            requested: HashSet::new(),
            icon_queue: VecDeque::new(),
            icons_loading: 0,
            remaining: f32::MAX,
            scroll_y: 0.0,
            library: HashMap::new(),
            installs: HashMap::new(),
            busy: HashSet::new(),
            instances: Vec::new(),
            target: None,
            added: HashMap::new(),
            adding: HashSet::new(),
            worlds: Vec::new(),
            target_world: None,
            placed: HashMap::new(),
            placing: HashSet::new(),
            hovered: None,
            origin: None,
        }
    }

    /// The platform the page searches.
    pub fn source(&self) -> Source {
        self.source
    }

    /// The instance the page was opened from to add mods to, while the page still shows that.
    pub fn origin(&self) -> Option<&InstanceChoice> {
        self.origin.as_ref()
    }

    /// Records the instance the page was opened from, or forgets it.
    pub fn set_origin(&mut self, origin: Option<InstanceChoice>) {
        self.origin = origin;
    }

    /// Shows the layout a saved preference names, as the page opens.
    pub fn set_view(&mut self, view: DiscoverView) {
        self.view = view;
    }

    /// Replaces the downloads the page knows are on disk, after the database is read or a download lands.
    pub fn set_library(&mut self, records: &[SharedRecord]) {
        self.library = records
            .iter()
            .map(|record| ((record.kind.clone(), record.platform.clone(), record.project_id.clone()), record.clone()))
            .collect();
    }

    /// Replaces the installs the launcher knows of. A pack still being prepared stays listed until its
    /// result arrives, so the card does not flicker back to its install button.
    pub fn set_installs(&mut self, installs: HashMap<(String, String), InstallState>) {
        let preparing: Vec<(String, String)> = self
            .installs
            .iter()
            .filter(|(key, state)| **state == InstallState::Preparing && !installs.contains_key(*key))
            .map(|(key, _)| key.clone())
            .collect();
        self.installs = installs;
        for key in preparing {
            self.installs.insert(key, InstallState::Preparing);
        }
    }

    /// The filters of the selected platform and tab, once they have loaded.
    pub(super) fn facets(&self) -> Option<&Facets> {
        self.facets.get(&(self.source, self.content))
    }

    /// The art a project shows: the platform's picture once it has loaded, a placeholder while it
    /// loads, and the generated scene only when the platform gave no picture or it could not be had.
    pub(super) fn artwork(&self, url: Option<&str>) -> Artwork<'_> {
        match url {
            None => Artwork::Scene,
            Some(url) => match self.icons.get(url) {
                Some(picture) => Artwork::Loaded(picture),
                None if self.failed.contains(url) => Artwork::Scene,
                None => Artwork::Loading,
            },
        }
    }

    /// Whether the project is already in its shared folder for the current tab.
    pub(super) fn in_library(&self, project_id: &str) -> bool {
        self.library
            .contains_key(&(self.folder(), self.source.key().to_string(), project_id.to_string()))
    }

    /// Whether a download of the project is running for the current tab.
    pub(super) fn downloading(&self, project_id: &str) -> bool {
        self.busy
            .contains(&(self.folder(), self.source.key().to_string(), project_id.to_string()))
    }

    /// The install state of a project, if the launcher knows of one.
    pub(super) fn install_of(&self, project_id: &str) -> Option<&InstallState> {
        self.installs.get(&(self.source.key().to_string(), project_id.to_string()))
    }

    /// The shared folder the current tab's content is kept in.
    fn folder(&self) -> String {
        shared_library::folder_name(self.content).to_string()
    }

    /// The project a card asked for, with the Minecraft version the filters ask for, and the release when one was picked.
    pub fn project(&self, project_id: &str, version_id: Option<&str>) -> Project {
        let hit = self.results.hits.iter().find(|hit| hit.id == project_id);
        Project {
            source: self.source,
            content: self.content,
            project_id: project_id.to_string(),
            name: hit.map_or_else(|| project_id.to_string(), |hit| hit.name.clone()),
            web_url: hit.and_then(|hit| hit.web_url.clone()),
            icon_url: hit.and_then(|hit| hit.icon_url.clone()),
            game_versions: self.filters.game_versions.clone(),
            version_id: version_id.map(str::to_string),
        }
    }

    /// What the card showed of a project, for its resource page to show while the page loads.
    fn listing(&self, project_id: &str) -> Listing {
        let hit = self.results.hits.iter().find(|hit| hit.id == project_id);
        Listing {
            source: self.source,
            content: self.content,
            project_id: project_id.to_string(),
            name: hit.map_or_else(|| project_id.to_string(), |hit| hit.name.clone()),
            author: hit.and_then(|hit| hit.author.clone()),
            summary: hit.map(|hit| hit.description.clone()).unwrap_or_default(),
            downloads: hit.map_or(0, |hit| hit.downloads),
            icon_url: hit.and_then(|hit| hit.icon_url.clone()),
            web_url: hit.and_then(|hit| hit.web_url.clone()),
            updated: hit.and_then(|hit| hit.updated.clone()),
        }
    }

    /// Starts a download of a project, or of one of its releases, into the shared folder of its kind. A project
    /// already in that folder, or downloading, starts nothing.
    pub fn download(&mut self, project: Project) -> Vec<Effect> {
        let key = (
            shared_library::folder_name(project.content).to_string(),
            project.source.key().to_string(),
            project.project_id.clone(),
        );
        if self.library.contains_key(&key) || self.busy.contains(&key) {
            return Vec::new();
        }
        self.busy.insert(key);
        vec![Effect::Download(project)]
    }

    /// Starts the install of a modpack, or of one of its releases. A pack already installing starts nothing.
    pub fn install(&mut self, project: Project) -> Vec<Effect> {
        let key = (project.source.key().to_string(), project.project_id.clone());
        if self.installs.contains_key(&key) {
            return Vec::new();
        }
        self.installs.insert(key, InstallState::Preparing);
        vec![Effect::Install(project)]
    }

    /// What the page knows of one project: whether it is downloaded, installed, or being added to the Mods tab's
    /// instance. The resource page's actions show this, so a project looks the same on both pages.
    pub fn status_of(&self, source: Source, content: ContentType, project_id: &str) -> ProjectStatus {
        let folder = shared_library::folder_name(content).to_string();
        let platform = source.key().to_string();
        let project = project_id.to_string();
        let target = self.target().cloned();
        // What the target holds of the project: a mod's record in the instance, or the record of a pack, a datapack or a
        // world in the place the tab names.
        let (added_version, adding) = if content == ContentType::Mod {
            let key = target.as_ref().map(|choice| (choice.id.clone(), platform.clone(), project.clone()));
            let added = key.as_ref().and_then(|key| self.added.get(key)).map(|record| record.version_id.clone());
            (added, key.as_ref().is_some_and(|key| self.adding.contains(key)))
        } else {
            let placement = self.placement_for(source, content, project_id);
            let placed = placement
                .as_ref()
                .and_then(|key| self.placed.get(key))
                .map(|record| record.version_id.clone());
            (placed, placement.as_ref().is_some_and(|key| self.placing.contains(key)))
        };
        let target_index = self
            .target
            .as_ref()
            .and_then(|id| self.instances.iter().position(|choice| &choice.id == id));
        ProjectStatus {
            in_library: self.library.contains_key(&(folder.clone(), platform.clone(), project.clone())),
            downloading: self.busy.contains(&(folder, platform.clone(), project.clone())),
            install: self.installs.get(&(platform.clone(), project.clone())).cloned(),
            added: added_version.is_some(),
            added_version,
            adding,
            instances: self.instances.iter().map(|choice| choice.name.clone()).collect(),
            target_index,
            target,
            target_world: self.target_world_name().map(str::to_string),
            worlds: self.worlds.iter().map(|world| world.name.clone()).collect(),
            world_index: self
                .target_world
                .as_ref()
                .and_then(|folder| self.worlds.iter().position(|world| &world.folder == folder)),
        }
    }

    /// The search the page is on, as a key.
    fn key(&self) -> query::Key {
        query::Key {
            source: self.source,
            content: self.content,
            text: self.text.trim().to_string(),
            sort: self.sort,
            filters: self.filters.clone(),
        }
    }

    /// Clears the filters, which belong to one platform and tab. The sort goes
    /// back to the default when the new platform cannot express the old one. On the
    /// Mods tab the instance's loader and Minecraft version are set again.
    fn reset_filters(&mut self) {
        self.filters = filters::Filters::default();
        self.panel.version_search.clear();
        if !self.source.sorts().contains(&self.sort) {
            self.sort = self.source.sorts()[0];
        }
        self.apply_target_filters();
    }

    /// Asks for the filters of the selected platform and tab, unless they are already known.
    fn facet_request(&self) -> Vec<Effect> {
        if self.facets.contains_key(&(self.source, self.content)) {
            Vec::new()
        } else {
            vec![Effect::Facets(self.source, self.content)]
        }
    }

    /// Starts the search for the current key, or the page after the hits on screen when `append` is set.
    /// A page the cache holds is shown at once, and anything else goes to the platform. A new search
    /// starts the results back at the top.
    fn start(&mut self, append: bool) -> Vec<Effect> {
        let key = self.key();
        self.generation += 1;
        let offset = if append { self.results.hits.len() as u32 } else { 0 };
        let mut effects = if append { Vec::new() } else { vec![Effect::ScrollToTop] };
        let cached = self.cache.get(&key, offset).map(|(page, elapsed_ms)| (page.clone(), elapsed_ms));
        if let Some((page, elapsed_ms)) = cached {
            self.show(page, offset, elapsed_ms, true);
            return effects;
        }
        if append {
            self.results.loading = true;
        } else {
            self.results = query::Results {
                loading: true,
                ..query::Results::default()
            };
        }
        effects.push(Effect::Search(Search {
            generation: self.generation,
            key,
            offset,
        }));
        effects
    }

    /// Puts a page of hits on screen. The first page of a search replaces the hits, and a later page is
    /// added to them.
    fn show(&mut self, page: SearchPage, offset: u32, elapsed_ms: u128, cached: bool) {
        self.results.last_page = page.hits.len();
        self.results.total = page.total;
        if offset == 0 {
            self.results.hits = page.hits;
        } else {
            self.results.hits.extend(page.hits);
        }
        self.results.loading = false;
        self.results.error = None;
        self.results.elapsed_ms = Some(elapsed_ms);
        self.results.cached = cached;
    }

    /// Carries out one message. Display state changes here; anything that needs
    /// the network comes back as an [`Effect`] for the root to run.
    pub fn update(&mut self, message: Message) -> Vec<Effect> {
        // Any message but a hover ends the hover: the card under the pointer may have moved or gone, and then its
        // exit would never arrive.
        if !matches!(message, Message::Hover { .. }) {
            self.hovered = None;
        }
        match message {
            Message::SourceSelected(source) => {
                if source == self.source || !source.enabled() {
                    return Vec::new();
                }
                self.source = source;
                self.placeholder = placeholder(source);
                if !source.supports(self.content) {
                    self.content = ContentType::Modpack;
                }
                self.reset_filters();
                let mut effects = self.facet_request();
                effects.extend(self.start(false));
                effects
            }
            Message::ContentSelected(content) => {
                if content == self.content || !self.source.supports(content) {
                    return Vec::new();
                }
                self.content = content;
                self.reset_filters();
                let mut effects = self.facet_request();
                effects.extend(self.start(false));
                effects
            }
            Message::QueryChanged(text) => {
                self.text = text;
                self.generation += 1;
                vec![Effect::Debounce(self.generation)]
            }
            Message::SearchNow => self.start(false),
            Message::SearchDue(generation) => {
                if generation == self.generation {
                    self.start(false)
                } else {
                    Vec::new()
                }
            }
            Message::SortSelected(index) => match self.source.sorts().get(index).copied() {
                Some(sort) if sort != self.sort => {
                    self.sort = sort;
                    self.start(false)
                }
                _ => Vec::new(),
            },
            Message::Filter(message) => self.filter_changed(message),
            Message::Place(project_id) => self.place(&project_id),
            Message::Placed { key, result } => self.placed_result(key, result),
            Message::WorldsLoaded { instance, worlds } => self.set_worlds(&instance, worlds),
            Message::WorldSelected(index) => self.select_world(index),
            Message::ViewSelected(view) => {
                self.view = view;
                vec![Effect::RememberView(view)]
            }
            Message::Back => vec![Effect::Back],
            Message::Scrolled { remaining, offset } => {
                self.remaining = remaining;
                self.scroll_y = offset;
                if remaining < LOAD_AHEAD && self.results.has_more() && !self.results.loading && self.results.error.is_none() {
                    self.start(true)
                } else {
                    Vec::new()
                }
            }
            Message::SearchFinished {
                key,
                offset,
                generation,
                result,
                elapsed_ms,
            } => {
                if generation != self.generation || key != self.key() {
                    return Vec::new();
                }
                match result {
                    Ok(page) => {
                        self.cache.put(key, offset, page.clone(), elapsed_ms);
                        self.show(page, offset, elapsed_ms, false);
                    }
                    Err(error) => {
                        self.results.loading = false;
                        self.results.error = Some(error);
                    }
                }
                Vec::new()
            }
            Message::FacetsLoaded { source, content, result } => {
                if let Ok(facets) = result {
                    self.facets.insert((source, content), facets);
                }
                Vec::new()
            }
            Message::IconReady { url, picture } => {
                // The load has ended, so its place is free for the next picture in the queue.
                self.icons_loading = self.icons_loading.saturating_sub(1);
                match picture {
                    Some(picture) => {
                        self.icons.insert(url, picture);
                    }
                    None => {
                        self.failed.insert(url);
                    }
                }
                Vec::new()
            }
            Message::Open(url) => vec![Effect::Open(url)],
            Message::FocusSearch => vec![Effect::Focus],
            Message::Download(project_id) => {
                let project = self.project(&project_id, None);
                self.download(project)
            }
            Message::Install(project_id) => {
                let project = self.project(&project_id, None);
                self.install(project)
            }
            Message::AddMod(project_id) => {
                let project = self.project(&project_id, None);
                self.add_mod(project)
            }
            Message::InstanceSelected(index) => self.select_instance(index),
            Message::OpenResource(project_id) => vec![Effect::OpenResource(self.listing(&project_id))],
            Message::Hover { project_id, hovered } => {
                self.hover(project_id, hovered);
                Vec::new()
            }
            Message::ModAdded {
                instance,
                platform,
                project_id,
                result,
                ..
            } => self.mod_added(instance, platform, project_id, result),
            Message::OpenInstance(id) => vec![Effect::OpenInstance(id)],
            Message::Downloaded {
                kind,
                platform,
                project_id,
                result,
            } => {
                self.busy.remove(&(kind.clone(), platform.clone(), project_id.clone()));
                match result {
                    Ok(record) => {
                        self.library.insert((kind, platform, project_id), record.clone());
                        vec![Effect::Record(record)]
                    }
                    Err(error) => vec![Effect::Notify(error)],
                }
            }
            Message::Prepared {
                platform,
                project_id,
                result,
            } => match result {
                Ok(request) => {
                    self.installs.insert((platform, project_id), InstallState::Queued);
                    vec![Effect::Queue(request)]
                }
                Err(error) => {
                    self.installs.remove(&(platform, project_id));
                    vec![Effect::Notify(error)]
                }
            },
        }
    }

    /// The first request for the page as it opens, so a platform's first page loads without a press.
    pub fn opening(&mut self) -> Vec<Effect> {
        let mut effects = self.facet_request();
        effects.extend(self.start(false));
        effects
    }

    /// The icon addresses the page should start loading now. The addresses of the hits on screen join a queue, each
    /// address once, and as many as the loads in flight leave room for are taken from its front. A queued address whose
    /// hit has gone is dropped, and asked for again if the hit comes back.
    pub fn wanted_icons(&mut self) -> Vec<String> {
        let shown: HashSet<String> = self
            .results
            .hits
            .iter()
            .flat_map(|hit| [&hit.icon_url, &hit.banner_url])
            .flatten()
            .cloned()
            .collect();
        let mut dropped = Vec::new();
        self.icon_queue.retain(|url| {
            let keep = shown.contains(url);
            if !keep {
                dropped.push(url.clone());
            }
            keep
        });
        for url in dropped {
            self.requested.remove(&url);
        }
        // In the order the hits are listed, so the pictures at the top of the page load first.
        for url in self.results.hits.iter().flat_map(|hit| [&hit.icon_url, &hit.banner_url]).flatten() {
            if self.requested.insert(url.clone()) {
                self.icon_queue.push_back(url.clone());
            }
        }
        let room = ICONS_AT_ONCE.saturating_sub(self.icons_loading);
        let started = room.min(self.icon_queue.len());
        self.icons_loading += started;
        self.icon_queue.drain(..started).collect()
    }

    /// The page. The header, with the source and tab rows, stays where it is, and the hero, the filters
    /// and the results scroll beneath it. Once the page scrolls up to the filters, they pin to the top of
    /// the scrolling part, and the results pass beneath them.
    pub fn view(&self) -> Element<'_, Message> {
        let fixed = container(column![header::view(self), header::source_row(self), header::tabs(self)].spacing(18))
            .padding(Padding::new(0.0).top(22).left(SIDE).right(SIDE))
            .width(Fill);
        let hero = hero::view(self);
        let pinned = filters_pinned(self.scroll_y, hero.is_some());
        let mut page = column![].spacing(GAP);
        if let Some(hero) = hero {
            page = page.push(hero);
        }
        // A pinned row leaves a space of its height behind, so the results stay where they were.
        let filters: Element<'_, Message> = if pinned {
            Space::new().width(Fill).height(toolbar::HEIGHT).into()
        } else {
            toolbar::view(self)
        };
        // The sidebar sits beside the toolbar and the results, and takes the whole height of them.
        let results = column![filters, results::view(self)].spacing(GAP).width(Fill);
        let body: Element<'_, Message> = match sidebar::view(self) {
            Some(sidebar) => row![sidebar, results].spacing(sidebar::COLUMN_GAP).width(Fill).into(),
            None => results.into(),
        };
        page = page.push(body);
        let content = container(page)
            .padding(Padding::new(0.0).top(GAP).left(SIDE).right(SIDE).bottom(40))
            .width(Fill);
        let scrolled = scrollbar::scroll(content).id(CONTENT_ID).width(Fill).height(Fill).on_scroll(|viewport| {
            let content = viewport.content_bounds();
            let bounds = viewport.bounds();
            let offset = viewport.absolute_offset().y;
            Message::Scrolled {
                remaining: content.height - (offset + bounds.height),
                offset,
            }
        });
        // The pinned row lies over the scrolling content in a stack. The stack keeps the same shape whether or
        // not the row is pinned, so the scrollable keeps its place and its scroll position when the row pins.
        let overlay: Element<'_, Message> = if pinned { toolbar::pinned(self) } else { Space::new().into() };
        let layers = stack![scrolled, overlay].width(Fill).height(Fill);
        column![fixed, layers].width(Fill).height(Fill).into()
    }
}

/// The search field's placeholder for a platform.
fn placeholder(source: Source) -> String {
    lang::t_with("discover.search_placeholder", &[("platform", source.name())])
}

/// Where the filter row sits in the scrolling content: under the top gap, and under the hero when the page
/// has one.
fn filters_top(has_hero: bool) -> f32 {
    if has_hero { GAP + hero::HEIGHT + GAP } else { GAP }
}

/// Whether the filter row has scrolled up to the top of the page. From then on it stays pinned there, and the
/// results scroll beneath it.
fn filters_pinned(scroll_y: f32, has_hero: bool) -> bool {
    scroll_y + GAP >= filters_top(has_hero)
}

#[cfg(test)]
#[path = "../../../../tests/app/pages/discover/mod.rs"]
mod tests;
