//! Shared data types returned by every loader.

/// Why a mod declares a dependency, mirroring the semantics the metadata formats
/// express (fabric depends/recommends/suggests/breaks/conflicts, forge/neoforge
/// `type` + `mandatory`, quilt `depends`/`breaks`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModDependencyKind {
    /// Must be present (`depends`, `mandatory = true`).
    Required,
    /// Must be loaded before this mod.
    LoadBefore,
    /// Nice to have (`recommends`).
    Optional,
    /// Suggested by the author.
    Recommended,
    /// Mildly suggested.
    Suggested,
    /// Must NOT be present alongside (`breaks`).
    Incompatible,
    /// Should not be present alongside (`conflicts`).
    Discouraged,
}

/// One dependency declared by a mod.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModDependency {
    /// The dependency's mod id.
    pub id: String,
    /// Why the dependency is declared.
    pub kind: ModDependencyKind,
    /// The declared version range, verbatim (`>=0.50.0`, `[1.20,)`, `*`).
    pub version_range: Option<String>,
}

/// Metadata extracted from a mod jar by `extract_mod_info`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModInfo {
    /// The loader whose metadata format was actually detected — "fabric",
    /// "forge", "neoforge", or "quilt". Never the *requested* kind: extraction
    /// is a convenience, not a filter.
    pub loader: &'static str,
    /// The mod's id (`modid`).
    pub id: String,
    /// The mod's own version, when declared.
    pub version: Option<String>,
    /// The human-readable name, when declared.
    pub name: Option<String>,
    /// The description, when declared.
    pub description: Option<String>,
    /// Author names, flattened from whatever shape the format uses.
    pub authors: Vec<String>,
    /// The declared license, when present.
    pub license: Option<String>,
    /// The project homepage / display URL, when present.
    pub homepage: Option<String>,
    /// Declared dependencies, deduplicated and kind-prioritized by modcrawl.
    pub dependencies: Vec<ModDependency>,
}

/// One installable version of a loader — or, for [`crate::Vanilla`], of Minecraft
/// itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoaderVersion {
    /// The loader's own version id, verbatim from its source: `"0.16.14"`,
    /// `"1.21.1-52.1.20"`, `"1.21.1"`, `"0.25w14craftmine.3-beta"`.
    pub version: String,
    /// The Minecraft version this loader version is for, when known.
    ///
    /// `versions_for_minecraft` always binds it. `list_versions` binds it for
    /// forge (derivable from the combined version string) and vanilla (the
    /// version *is* the Minecraft version), and leaves it `None` for
    /// fabric/quilt/neoforge. `download_client` requires it.
    pub minecraft: Option<String>,
    /// Whether this is a stable build (see DESIGN.md §5 for per-kind rules).
    pub stable: bool,
    /// A human-readable changelog, when the source provides one.
    pub changelog_url: Option<String>,
}

impl LoaderVersion {
    /// Builds a version with no changelog link (no source in this crate
    /// provides one yet).
    pub(crate) fn new(version: impl Into<String>, minecraft: Option<String>, stable: bool) -> Self {
        Self {
            version: version.into(),
            minecraft,
            stable,
            changelog_url: None,
        }
    }
}
