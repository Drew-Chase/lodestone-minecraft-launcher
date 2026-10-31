//! Shared data types returned by every platform.

/// The classes of content platforms host.
///
/// Platforms declare which of these they cover (see the capability matrix in
/// DESIGN.md §5); requests outside a platform's matrix are rejected up front.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContentType {
    /// Mods (and mod-like plugins).
    Mod,
    /// Modpacks — installable bundles of mods and configuration.
    Modpack,
    /// Shaders.
    Shader,
    /// Resource packs (CurseForge: texture packs).
    ResourcePack,
    /// Data packs.
    DataPack,
    /// Saved worlds (CurseForge only).
    World,
}

impl std::fmt::Display for ContentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            ContentType::Mod => "mod",
            ContentType::Modpack => "modpack",
            ContentType::Shader => "shader",
            ContentType::ResourcePack => "resource pack",
            ContentType::DataPack => "data pack",
            ContentType::World => "world",
        };
        write!(f, "{name}")
    }
}

/// One row of a search result list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResult {
    /// The platform's stable project id — store this, not the slug.
    pub id: String,
    /// The vanity slug, when the platform has one (it can change at any time).
    pub slug: Option<String>,
    /// The project's display name.
    pub name: String,
    /// The author's name, when the platform provides one.
    pub author: Option<String>,
    /// The short (one-line) description.
    pub description: String,
    /// Total download count, when the platform provides one.
    pub downloads: u64,
    /// The icon url, when the project has one.
    pub icon_url: Option<String>,
    /// The project's web page, when the platform provides one.
    pub web_url: Option<String>,
    /// When the project was last updated, as the platform writes it (ISO 8601), when it says.
    pub updated: Option<String>,
    /// The content type this result is.
    pub content_type: ContentType,
}

/// How a search orders its hits. Each platform maps these onto its own sort
/// fields and ignores a sort it cannot express.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SearchSort {
    /// The platform's own ranking for the query.
    #[default]
    Relevance,
    /// Most downloaded first.
    Popular,
    /// Most recently updated first.
    Updated,
    /// Most recently released first.
    Newest,
}

/// One search request: text, paging, and the optional filters a platform supports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchQuery {
    /// The text to match. Empty browses the platform.
    pub text: String,
    /// The content type to search within.
    pub content: ContentType,
    /// How to order the hits.
    pub sort: SearchSort,
    /// How many hits to skip.
    pub offset: u32,
    /// How many hits to return at most.
    pub limit: u32,
    /// A category id, from [`Facets::categories`].
    pub category: Option<String>,
    /// A loader name, from [`Facets::loaders`].
    pub loader: Option<String>,
    /// A Minecraft version, from [`Facets::game_versions`].
    pub game_version: Option<String>,
}

impl SearchQuery {
    /// The first page of `content`, with no text and no filters.
    pub fn new(content: ContentType) -> Self {
        Self {
            text: String::new(),
            content,
            sort: SearchSort::Relevance,
            offset: 0,
            limit: 20,
            category: None,
            loader: None,
            game_version: None,
        }
    }
}

/// One page of search hits.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SearchPage {
    /// The hits on this page, in the platform's order.
    pub hits: Vec<SearchResult>,
    /// How many hits match in all, when the platform reports it.
    pub total: u64,
}

/// One choice in a filter: the id to send back, and the name to show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Facet {
    /// The id a [`SearchQuery`] carries.
    pub id: String,
    /// The name to show.
    pub name: String,
}

/// The filters a platform offers for one content type. An empty list means the
/// platform has no such filter.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Facets {
    /// The categories a project can be filed under.
    pub categories: Vec<Facet>,
    /// The mod loaders, named as `lodestone_loaders` names them.
    pub loaders: Vec<String>,
    /// The Minecraft versions, newest first.
    pub game_versions: Vec<String>,
}

/// A display name for a platform id: `kitchen-sink` becomes `Kitchen Sink`.
pub(crate) fn display_name(id: &str) -> String {
    id.split(['-', '_', ' '])
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect::<String>(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
#[path = "../tests/meta.rs"]
mod tests;

/// The full details of one project or pack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentDetails {
    /// The platform's stable project id.
    pub id: String,
    /// The vanity slug, when the platform has one.
    pub slug: Option<String>,
    /// The project's display name.
    pub name: String,
    /// The author's name, when the platform provides one.
    pub author: Option<String>,
    /// The short (one-line) description.
    pub summary: String,
    /// The long-form description, when the platform provides one separately.
    pub description: Option<String>,
    /// Total download count, when the platform provides one.
    pub downloads: u64,
    /// The icon url, when the project has one.
    pub icon_url: Option<String>,
    /// The project's web page, when the platform provides one.
    pub web_url: Option<String>,
    /// The content type this project is.
    pub content_type: ContentType,
}

/// One downloadable file belonging to a [`ContentVersion`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentFile {
    /// The file's name on disk.
    pub name: String,
    /// The direct download url.
    pub url: String,
    /// The sha1 hash, when the platform provides one.
    pub sha1: Option<String>,
    /// The size in bytes, when the platform provides one.
    pub size: Option<u64>,
    /// Whether this is the version's primary file.
    pub primary: bool,
}

/// One release of a project (mod file set) or pack (build).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentVersion {
    /// The platform's stable version id — store this, not the display name.
    pub id: String,
    /// The owning project's id, when follow-up calls need it (CurseForge's
    /// download-link resolution) — `None` elsewhere.
    pub project_id: Option<String>,
    /// The release's display name, when the platform provides one.
    pub name: Option<String>,
    /// The release's version string (semver-ish).
    pub version_number: String,
    /// The content type this release belongs to (drives the capability gate on
    /// `download`).
    pub content_type: ContentType,
    /// The Minecraft versions this release targets.
    pub minecraft_versions: Vec<String>,
    /// The mod loaders this release targets ("fabric", "forge", "neoforge",
    /// "quilt", …). Empty for content that is loader-agnostic or for packs.
    ///
    /// These strings match `lodestone_loaders`' loader names.
    pub loaders: Vec<String>,
    /// Total download count, when the platform provides one.
    pub downloads: u64,
    /// The release date, when the platform provides one.
    pub date: Option<String>,
    /// A human-readable changelog, when the platform provides one.
    pub changelog_url: Option<String>,
    /// The downloadable files of this release.
    pub files: Vec<ContentFile>,
}

/// One mod file a parsed modpack wants installed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModpackFile {
    /// The destination path inside the instance (`mods/foo.jar`,
    /// `config/bar.toml`, …), relative to the instance root.
    pub path: String,
    /// The direct download url — set when the platform serves files directly
    /// (Modrinth). Empty when resolution needs the platform API (CurseForge:
    /// see `project_id`/`file_id`).
    pub url: String,
    /// The sha1 hash, when the platform provides one.
    pub sha1: Option<String>,
    /// The size in bytes, when the platform provides one.
    pub size: Option<u64>,
    /// CurseForge project id, when the file is referenced by ids.
    pub project_id: Option<String>,
    /// CurseForge file id, when the file is referenced by ids.
    pub file_id: Option<String>,
}

/// Modpack data parsed from a platform archive by `parse_modpack` — the
/// installable description of a modpack, ready for the install slice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Modpack {
    /// The modpack's display name.
    pub name: String,
    /// The modpack's own version, when the manifest declares one.
    pub version: Option<String>,
    /// The Minecraft version the modpack targets.
    pub minecraft: String,
    /// The mod loader the modpack uses ("fabric", "forge", "quilt",
    /// "neoforge"), when any.
    pub loader: Option<&'static str>,
    /// The mod loader's version, when the manifest declares one.
    pub loader_version: Option<String>,
    /// The files the modpack installs.
    pub files: Vec<ModpackFile>,
    /// The archive directory holding config overrides, when present. An empty name means the archive root.
    pub overrides: Option<String>,
    /// Archive entries the overrides leave out, as paths inside the archive.
    pub overrides_skip: Vec<String>,
    /// Archives the pack downloads and unpacks on top of its files.
    pub archives: Vec<ModpackArchive>,
}

/// An archive a modpack downloads and unpacks into the instance: a config bundle, or a mod that is
/// shipped as an archive and has to be extracted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModpackArchive {
    /// The file's name, which the download keeps.
    pub name: String,
    /// The direct download url.
    pub url: String,
    /// The sha1 hash, when the platform provides one.
    pub sha1: Option<String>,
    /// The size in bytes, when the platform provides one.
    pub size: Option<u64>,
    /// The folder inside the instance the archive unpacks into, relative to the instance root. Empty
    /// means the root.
    pub target: String,
    /// The folder inside the archive whose contents unpack, without a trailing slash. Empty means the
    /// whole archive.
    pub prefix: String,
}
