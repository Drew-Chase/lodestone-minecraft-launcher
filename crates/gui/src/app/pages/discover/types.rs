//! The Discover page's types: what the page shows and asks for, and the messages and effects it trades with the root.

use super::Source;
use super::filters::FilterMessage;
use super::images::Picture;
use super::query;
use super::target::{AddRequest, InstanceChoice, WorldChoice};
use crate::app::install::InstallRequest;
use crate::app::lang;
use crate::app::pages::resource::Listing;
use crate::app::shared_library::{self, AddedContent, ModSource, SharedRecord};
use lodestone_platforms::{ContentType, Facets, SearchPage};

/// The art a project shows while the page is open.
#[derive(Debug, Clone, Copy)]
pub enum Artwork<'a> {
    /// The platform's own picture, loaded.
    Loaded(&'a Picture),
    /// The platform names a picture that is still loading.
    Loading,
    /// The platform gave no picture, or it could not be loaded: the generated scene stands in.
    Scene,
}

/// The content tabs, in the order the page shows them.
pub const TABS: [ContentType; 6] = [
    ContentType::Modpack,
    ContentType::Mod,
    ContentType::ResourcePack,
    ContentType::Shader,
    ContentType::World,
    ContentType::DataPack,
];

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
    /// The key the layout is saved under in the preferences.
    pub fn key(self) -> &'static str {
        match self {
            DiscoverView::Grid => "grid",
            DiscoverView::Compact => "compact",
            DiscoverView::Table => "table",
        }
    }

    /// The layout a saved key names, when it names one.
    pub fn from_key(key: &str) -> Option<Self> {
        match key {
            "grid" => Some(DiscoverView::Grid),
            "compact" => Some(DiscoverView::Compact),
            "table" => Some(DiscoverView::Table),
            _ => None,
        }
    }

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
    /// The narrow window's filter panel was opened or shut, from its toolbar button.
    ToggleFilters,
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
    /// A pick, or a change to the panel, from the sidebar.
    Filter(FilterMessage),
    /// The card's add button was pressed for the project with this id: it goes into the target instance or world.
    Place(String),
    /// A project was placed, or could not be. `key` names the place it went to.
    Placed {
        key: Placement,
        result: Result<Box<PlaceOutcome>, String>,
    },
    /// The worlds of an instance were read, for the world picker of the datapack section.
    WorldsLoaded { instance: String, worlds: Vec<WorldChoice> },
    /// A world was picked in the world picker, by its index in the page's list.
    WorldSelected(usize),
    /// The layout changed. The root saves it, so the page opens on it next time.
    ViewSelected(DiscoverView),
    /// The back button was pressed: the page returns to the instance it was opened from.
    Back,
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
    /// A card's add button was pressed for the project with this id: it goes into the Mods tab's instance.
    AddMod(String),
    /// An instance was picked in the Mods tab's selector, by its index in the page's list.
    InstanceSelected(usize),
    /// A card or the hero was pressed for the project with this id: its resource page opens.
    OpenResource(String),
    /// The pointer entered (`hovered`) or left the grid card of the project with this id, which grows while hovered.
    Hover { project_id: String, hovered: bool },
    /// A mod was added to an instance, or replaced there, or could not be. The key is the instance, platform and project.
    /// `job` is the downloads job that carried the add, which the root finishes before the page sees the answer.
    ModAdded {
        job: u64,
        instance: String,
        platform: String,
        project_id: String,
        result: Result<Vec<ModSource>, String>,
    },
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
    /// The Minecraft versions the filters ask for. A release is chosen when it supports any of them.
    pub game_versions: Vec<String>,
    /// The release the user picked, when they picked one. `None` takes the newest that fits the filters.
    pub version_id: Option<String>,
}

/// What the page knows of one project, for the resource page's actions to show.
#[derive(Debug, Clone)]
pub struct ProjectStatus {
    /// Whether the project is in its shared folder.
    pub in_library: bool,
    /// Whether a download of the project is running.
    pub downloading: bool,
    /// The install of a modpack, when the launcher knows of one.
    pub install: Option<InstallState>,
    /// The instance the Mods tab adds to, when one is picked.
    pub target: Option<InstanceChoice>,
    /// The names of the instances a mod can be added to, in the order the selector lists them.
    pub instances: Vec<String>,
    /// The index of the target among the instances, when one is picked.
    pub target_index: Option<usize>,
    /// The name of the target world, when the datapacks go to one.
    pub target_world: Option<String>,
    /// The names of the worlds of the target instance, in the order the world picker lists them.
    pub worlds: Vec<String>,
    /// The index of the target world among the worlds, when one is picked.
    pub world_index: Option<usize>,
    /// Whether the project is already in the target instance, at any release.
    pub added: bool,
    /// The release of the project the target instance has, when it has one.
    pub added_version: Option<String>,
    /// Whether the project is being added to the target instance now.
    pub adding: bool,
}

/// Where a placed project is kept: the instance, the world's folder for a datapack (empty otherwise), the shared folder's
/// name for the kind, the platform's key, and the project's id. The page keys what it added by it.
pub type Placement = (String, String, String, String, String);

/// Where a project goes: the instance and the world, by their ids.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Destination {
    /// The pack folder of an instance: `resourcepacks` for a resource pack, and `shaderpacks` for a shader.
    Pack { instance: String },
    /// The datapacks of a world, by its folder name in the instance's saves.
    Datapacks { instance: String, world: String },
    /// The instance's saves, where a world is imported as a save of its own.
    World { instance: String },
}

impl Destination {
    /// The instance the project goes into.
    pub fn instance(&self) -> &str {
        match self {
            Destination::Pack { instance } | Destination::Datapacks { instance, .. } | Destination::World { instance } => instance,
        }
    }
}

/// A project to place into a target instance or world.
#[derive(Debug, Clone)]
pub struct PlaceRequest {
    /// The project, with the release the user picked when they picked one.
    pub project: Project,
    /// The Minecraft version of the instance the project goes into. The release is the newest that supports it.
    pub minecraft: String,
    /// Where the project goes.
    pub destination: Destination,
    /// The download the shared folder already holds for the project. It is used again when it is the release chosen.
    pub shared: Option<SharedRecord>,
}

impl PlaceRequest {
    /// The place this request fills, which the page keeps its record of the project under.
    pub fn placement(&self) -> Placement {
        let scope = match &self.destination {
            Destination::Datapacks { world, .. } => world.clone(),
            Destination::Pack { .. } | Destination::World { .. } => String::new(),
        };
        (
            self.destination.instance().to_string(),
            scope,
            shared_library::folder_name(self.project.content).to_string(),
            self.project.source.key().to_string(),
            self.project.project_id.clone(),
        )
    }
}

/// What a placement left behind: the record of the content placed, and the download made for it when one was made.
#[derive(Debug, Clone)]
pub struct PlaceOutcome {
    pub added: AddedContent,
    pub shared: Option<SharedRecord>,
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
    /// Open a project's resource page.
    OpenResource(Listing),
    /// Add a mod to the instance's mods folder, with the release the user picked or the newest that fits.
    AddMod(Box<AddRequest>),
    /// Swap the jar `old_file` of a mod the instance has at another release for the release the request names.
    ReplaceMod { request: Box<AddRequest>, old_file: String },
    /// Save a mod added to an instance, so the page still lists it after a restart.
    RecordModSources(Vec<ModSource>),
    /// Save the layout the page shows, so it opens on it next time.
    RememberView(DiscoverView),
    /// Return to the instance the page was opened from.
    Back,
    /// Place a project into a target instance or world, and answer with `Placed`.
    Place(Box<PlaceRequest>),
    /// Read the worlds of an instance, for the world picker, and answer with `WorldsLoaded`.
    LoadWorlds(String),
    /// Save the content a placement added, so the page still shows it as added after a restart.
    RecordAdded(AddedContent),
}
