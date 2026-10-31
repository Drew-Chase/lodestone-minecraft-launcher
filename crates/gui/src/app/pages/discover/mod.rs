//! The Discover page: one platform's modpacks, resource packs, shaders, worlds
//! and data packs, searched and browsed in grid, compact or table layouts. A
//! card can install a modpack, or download any other project into the shared
//! folders.
//!
//! The page decides what to ask for and keeps what came back. The root turns
//! the [`Effect`]s an update returns into tasks, and sends the answers back as
//! messages.

mod card;
mod header;
mod hero;
mod images;
mod query;
mod results;
mod source;
mod toolbar;

use std::collections::{HashMap, HashSet};

use crate::app::components::scrollbar;
use crate::app::install::InstallRequest;
use crate::app::lang;
use crate::app::shared_library::{self, SharedRecord};
use iced::widget::{Space, column, container, stack};
use iced::{Element, Fill, Padding, Size};
use lodestone_platforms::{ContentType, Facets, SearchPage, SearchSort};

pub use card::scene_for;
pub use images::{Picture, load as load_icon, pack_art};
pub use source::Source;

/// The art a project shows while the page is open.
#[derive(Debug, Clone, Copy)]
pub(super) enum Artwork<'a> {
    /// The platform's own picture, loaded.
    Loaded(&'a Picture),
    /// The platform names a picture that is still loading.
    Loading,
    /// The platform gave no picture, or it could not be loaded: the generated scene stands in.
    Scene,
}

/// The content tabs, in the order the page shows them. The Mods tab is left out on purpose.
pub const TABS: [ContentType; 5] = [
    ContentType::Modpack,
    ContentType::ResourcePack,
    ContentType::Shader,
    ContentType::World,
    ContentType::DataPack,
];

/// The id of the search field, which the root focuses for Ctrl/⌘K.
pub const SEARCH_ID: &str = "discover-search";

/// The id of the scrolling part of the page, below the header. The root scrolls it back to the top when
/// a new search starts.
pub const CONTENT_ID: &str = "discover-content";

/// How close to the end of the results, in pixels, the page asks for the next one.
const LOAD_AHEAD: f32 = 900.0;

/// The space between the page's blocks, and above the first of them. A pinned filter row keeps the same gap above it.
const GAP: f32 = 18.0;

/// The margin either side of the page, shared by the header and the scrolling content.
const SIDE: f32 = 28.0;

/// The layouts the results take.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DiscoverView {
    /// Cards in a grid, the reference's default.
    #[default]
    Grid,
    /// Two columns of thumbnail rows.
    Compact,
    /// One row per project, with its details in columns.
    Table,
}

impl DiscoverView {
    /// The switcher's label.
    pub fn label(self) -> &'static str {
        match self {
            DiscoverView::Grid => lang::t("discover.view.grid"),
            DiscoverView::Compact => lang::t("discover.view.compact"),
            DiscoverView::Table => lang::t("discover.view.table"),
        }
    }
}

/// Where a modpack's install stands, as the card shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallState {
    /// The pack is being downloaded and checked before it is queued.
    Preparing,
    /// The install waits in, or runs in, the downloads queue.
    Queued,
    /// The instance the pack made.
    Installed(String),
}

/// Messages the Discover page handles.
#[derive(Debug, Clone)]
pub enum Message {
    /// A platform was picked in the source selector.
    SourceSelected(Source),
    /// A content tab was picked.
    ContentSelected(ContentType),
    /// The search text changed. The root sends `SearchDue` after a short pause.
    QueryChanged(String),
    /// Enter was pressed in the search field.
    SearchNow,
    /// The pause after the last keystroke ended. An older generation is stale.
    SearchDue(u64),
    /// A sort was picked, by its index in the platform's sorts.
    SortSelected(usize),
    /// A category was picked. Index 0 means any.
    CategorySelected(usize),
    /// A loader was picked. Index 0 means any.
    LoaderSelected(usize),
    /// A Minecraft version was picked. Index 0 means any.
    VersionSelected(usize),
    /// The layout changed.
    ViewSelected(DiscoverView),
    /// The page scrolled. `remaining` is how far the content runs past the bottom of the window, and
    /// `offset` is how far the content has scrolled down.
    Scrolled { remaining: f32, offset: f32 },
    /// A search came back for `offset` in `generation`.
    SearchFinished {
        key: query::Key,
        offset: u32,
        generation: u64,
        result: Result<SearchPage, String>,
        elapsed_ms: u128,
    },
    /// The filters of a platform and tab came back.
    FacetsLoaded {
        source: Source,
        content: ContentType,
        result: Result<Facets, String>,
    },
    /// A picture finished loading, or could not be had (`None`).
    IconReady { url: String, picture: Option<Picture> },
    /// A project's page should open in the browser.
    Open(String),
    /// Ctrl/⌘K was pressed: focus the search field.
    FocusSearch,
    /// A card's download button was pressed for the project with this id.
    Download(String),
    /// A card's install button was pressed for the project with this id.
    Install(String),
    /// A card's open button was pressed for the instance with this id.
    OpenInstance(String),
    /// A download into the shared folders finished, or failed.
    Downloaded {
        kind: String,
        platform: String,
        project_id: String,
        result: Result<SharedRecord, String>,
    },
    /// A modpack was downloaded and read, and its install is ready to queue, or it failed.
    Prepared {
        platform: String,
        project_id: String,
        result: Result<Box<InstallRequest>, String>,
    },
}

/// A search the root starts for the page.
#[derive(Debug, Clone)]
pub struct Search {
    /// The generation the answer must still carry to be kept.
    pub generation: u64,
    pub key: query::Key,
    pub offset: u32,
}

/// A project a card asked the root to act on.
#[derive(Debug, Clone)]
pub struct Project {
    pub source: Source,
    pub content: ContentType,
    pub project_id: String,
    pub name: String,
    /// The project's page on the platform, when it has one.
    pub web_url: Option<String>,
    /// The project's picture on the platform, when the search gave one.
    pub icon_url: Option<String>,
    /// The Minecraft version the filters ask for, when they ask for one.
    pub game_version: Option<String>,
}

/// What the root does for the page after an update.
#[derive(Debug, Clone)]
pub enum Effect {
    /// Send `SearchDue` with this generation after a short pause.
    Debounce(u64),
    /// Ask the platform for a page of a search.
    Search(Search),
    /// Load the filters of a platform and tab.
    Facets(Source, ContentType),
    /// Open a project's page in the browser.
    Open(String),
    /// Focus the search field.
    Focus,
    /// Put the results back at the top, for a new search.
    ScrollToTop,
    /// Download a project's newest matching version into its shared folder.
    Download(Project),
    /// Download a modpack and prepare its install.
    Install(Project),
    /// Queue a prepared install with the downloads.
    Queue(Box<InstallRequest>),
    /// Save a shared download in the database, so it is found again next launch.
    Record(SharedRecord),
    /// Tell the user something went wrong.
    Notify(String),
    /// Open an instance's page.
    OpenInstance(String),
}

/// The Discover page's state.
pub struct Discover {
    pub(super) source: Source,
    pub(super) content: ContentType,
    pub(super) text: String,
    /// The search field's placeholder, which names the platform.
    pub(super) placeholder: String,
    pub(super) sort: SearchSort,
    pub(super) category: Option<String>,
    pub(super) loader: Option<String>,
    pub(super) game_version: Option<String>,
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
    /// How far the content runs past the bottom of the window, as of the last scroll.
    remaining: f32,
    /// How far the content has scrolled down, as of the last scroll. The filter row pins once it reaches the top.
    scroll_y: f32,
    /// The downloads on disk, by (shared folder, platform key, project id).
    library: HashSet<(String, String, String)>,
    /// The installs the page knows of, by (platform key, project id).
    installs: HashMap<(String, String), InstallState>,
    /// The downloads still running, by (shared folder, platform key, project id).
    busy: HashSet<(String, String, String)>,
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
            category: None,
            loader: None,
            game_version: None,
            view: DiscoverView::default(),
            results: query::Results::default(),
            generation: 0,
            facets: HashMap::new(),
            cache: query::Cache::default(),
            icons: HashMap::new(),
            failed: HashSet::new(),
            requested: HashSet::new(),
            remaining: f32::MAX,
            scroll_y: 0.0,
            library: HashSet::new(),
            installs: HashMap::new(),
            busy: HashSet::new(),
        }
    }

    /// The platform the page searches.
    pub fn source(&self) -> Source {
        self.source
    }

    /// Replaces the downloads the page knows are on disk, after the database is read or a download lands.
    pub fn set_library(&mut self, records: &[SharedRecord]) {
        self.library = records
            .iter()
            .map(|record| (record.kind.clone(), record.platform.clone(), record.project_id.clone()))
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
            .contains(&(self.folder(), self.source.key().to_string(), project_id.to_string()))
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

    /// The project a card asked for, with the Minecraft version the filters ask for.
    fn project(&self, project_id: &str) -> Project {
        let hit = self.results.hits.iter().find(|hit| hit.id == project_id);
        Project {
            source: self.source,
            content: self.content,
            project_id: project_id.to_string(),
            name: hit.map_or_else(|| project_id.to_string(), |hit| hit.name.clone()),
            web_url: hit.and_then(|hit| hit.web_url.clone()),
            icon_url: hit.and_then(|hit| hit.icon_url.clone()),
            game_version: self.game_version.clone(),
        }
    }

    /// The search the page is on, as a key.
    fn key(&self) -> query::Key {
        query::Key {
            source: self.source,
            content: self.content,
            text: self.text.trim().to_string(),
            sort: self.sort,
            category: self.category.clone(),
            loader: self.loader.clone(),
            game_version: self.game_version.clone(),
        }
    }

    /// Clears the filters, which belong to one platform and tab. The sort goes
    /// back to the default when the new platform cannot express the old one.
    fn reset_filters(&mut self) {
        self.category = None;
        self.loader = None;
        self.game_version = None;
        if !self.source.sorts().contains(&self.sort) {
            self.sort = self.source.sorts()[0];
        }
    }

    /// Asks for the filters of the selected platform and tab, unless they are already known.
    fn facet_request(&self) -> Vec<Effect> {
        if !self.source.has_filters() || self.facets.contains_key(&(self.source, self.content)) {
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
            Message::CategorySelected(index) => {
                self.category = self
                    .facets()
                    .and_then(|facets| index.checked_sub(1).and_then(|index| facets.categories.get(index)))
                    .map(|facet| facet.id.clone());
                self.start(false)
            }
            Message::LoaderSelected(index) => {
                self.loader = self
                    .facets()
                    .and_then(|facets| index.checked_sub(1).and_then(|index| facets.loaders.get(index)))
                    .cloned();
                self.start(false)
            }
            Message::VersionSelected(index) => {
                self.game_version = self
                    .facets()
                    .and_then(|facets| index.checked_sub(1).and_then(|index| facets.game_versions.get(index)))
                    .cloned();
                self.start(false)
            }
            Message::ViewSelected(view) => {
                self.view = view;
                Vec::new()
            }
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
                if self.in_library(&project_id) || self.downloading(&project_id) {
                    return Vec::new();
                }
                self.busy.insert((self.folder(), self.source.key().to_string(), project_id.clone()));
                vec![Effect::Download(self.project(&project_id))]
            }
            Message::Install(project_id) => {
                let key = (self.source.key().to_string(), project_id.clone());
                if self.installs.contains_key(&key) {
                    return Vec::new();
                }
                self.installs.insert(key, InstallState::Preparing);
                vec![Effect::Install(self.project(&project_id))]
            }
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
                        self.library.insert((kind, platform, project_id));
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

    /// The icon addresses the hits on screen need and have not been asked for yet.
    /// Each address is returned once.
    pub fn wanted_icons(&mut self) -> Vec<String> {
        let mut wanted = Vec::new();
        for hit in &self.results.hits {
            if let Some(url) = &hit.icon_url
                && self.requested.insert(url.clone())
            {
                wanted.push(url.clone());
            }
        }
        wanted
    }

    /// The page. The header, with the source and tab rows, stays where it is, and the hero, the filters
    /// and the results scroll beneath it. Once the page scrolls up to the filters, they pin to the top of
    /// the scrolling part, and the results pass beneath them.
    pub fn view(&self, window: Size) -> Element<'_, Message> {
        let fixed = container(column![header::view(self), header::source_row(self), header::tabs(self)].spacing(18))
            .padding(Padding::new(0.0).top(22).left(SIDE).right(SIDE))
            .width(Fill);
        let hero = hero::view(self, window);
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
        page = page.push(filters).push(results::view(self, window));
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
