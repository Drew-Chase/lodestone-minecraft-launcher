//! Translation from CurseForge wire-format types into the crate's public model.

use crate::model::common::{Author, ContentBase, License, Links, SideSupport};
use crate::model::{
    DatapackItem, ModItem, PackItem, ResourcePackItem, ShaderPackItem, WorldItem,
};
use crate::platform::{ContentType, Platform, Sort};

use super::dto::{CfMod, FileIndex};

/// Minecraft's `gameId` on CurseForge.
pub(crate) const MINECRAFT_GAME_ID: u64 = 432;

/// `classId` values for Minecraft on CurseForge.
pub(crate) const CLASS_MODS: u64 = 6;
pub(crate) const CLASS_RESOURCEPACKS: u64 = 12;
pub(crate) const CLASS_WORLDS: u64 = 17;
pub(crate) const CLASS_MODPACKS: u64 = 4471;
pub(crate) const CLASS_SHADERS: u64 = 6552;
pub(crate) const CLASS_DATAPACKS: u64 = 6945;

/// CurseForge `modLoaderType` enum values.
#[allow(dead_code)]
pub(crate) const LOADER_ANY: u32 = 0;
pub(crate) const LOADER_FORGE: u32 = 1;
pub(crate) const LOADER_CAULDRON: u32 = 2;
pub(crate) const LOADER_LITELOADER: u32 = 3;
pub(crate) const LOADER_FABRIC: u32 = 4;
pub(crate) const LOADER_QUILT: u32 = 5;
pub(crate) const LOADER_NEOFORGE: u32 = 6;

/// Map our [`ContentType`] to CurseForge's `classId`. Returns `None` when
/// CurseForge does not expose a matching class (none today — all six map).
pub(crate) fn content_type_to_class_id(kind: ContentType) -> Option<u64> {
    match kind {
        ContentType::Mod => Some(CLASS_MODS),
        ContentType::Modpack => Some(CLASS_MODPACKS),
        ContentType::ResourcePack => Some(CLASS_RESOURCEPACKS),
        ContentType::ShaderPack => Some(CLASS_SHADERS),
        ContentType::Datapack => Some(CLASS_DATAPACKS),
        ContentType::World => Some(CLASS_WORLDS),
    }
}

/// Map our [`Sort`] enum to CurseForge's (`sortField` int, `sortOrder` string).
///
/// - `Relevance` → Featured(1) desc. When a search query is present CF
///   implicitly biases Featured toward matches, so this is close enough to
///   Modrinth's "relevance".
/// - `Downloads` → TotalDownloads(6) desc.
/// - `Follows` → Rating(12) desc. CurseForge has no follower count; rating
///   is the closest popularity signal.
/// - `Latest` → ReleasedDate(11) desc.
/// - `Updated` → LastUpdated(3) desc.
pub(crate) fn sort_to_cf(sort: Sort) -> (u32, &'static str) {
    match sort {
        Sort::Relevance => (1, "desc"),
        Sort::Downloads => (6, "desc"),
        Sort::Follows => (12, "desc"),
        Sort::Latest => (11, "desc"),
        Sort::Updated => (3, "desc"),
    }
}

/// Integer loader id → human-readable name used by our public model.
/// Unknown ids are skipped rather than surfaced as numeric strings.
pub(crate) fn loader_name(id: u32) -> Option<&'static str> {
    match id {
        LOADER_FORGE => Some("forge"),
        LOADER_CAULDRON => Some("cauldron"),
        LOADER_LITELOADER => Some("liteloader"),
        LOADER_FABRIC => Some("fabric"),
        LOADER_QUILT => Some("quilt"),
        LOADER_NEOFORGE => Some("neoforge"),
        _ => None, // includes LOADER_ANY(0) and any unknown future values
    }
}

/// Inverse of [`loader_name`]: human-readable loader name → CurseForge
/// `modLoaderType` integer. Returns `None` for unknown names.
pub(crate) fn loader_name_to_id(name: &str) -> Option<u32> {
    match name.to_ascii_lowercase().as_str() {
        "forge" => Some(LOADER_FORGE),
        "cauldron" => Some(LOADER_CAULDRON),
        "liteloader" => Some(LOADER_LITELOADER),
        "fabric" => Some(LOADER_FABRIC),
        "quilt" => Some(LOADER_QUILT),
        "neoforge" => Some(LOADER_NEOFORGE),
        _ => None,
    }
}

/// Return the distinct set of loader names present on a project, preserving
/// first-seen order so the output is stable across runs.
fn collect_loaders(indexes: &[FileIndex]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for idx in indexes {
        if let Some(id) = idx.mod_loader
            && let Some(name) = loader_name(id)
            && !out.iter().any(|s| s == name)
        {
            out.push(name.to_string());
        }
    }
    out
}

/// Is this string a pure `X.Y` or `X.Y.Z` Minecraft version (not a loader name
/// or channel like "1.20-Snapshot")?
fn is_mc_version(s: &str) -> bool {
    let mut dots = 0;
    let mut any_digit = false;
    for c in s.chars() {
        match c {
            '.' => dots += 1,
            '0'..='9' => any_digit = true,
            _ => return false,
        }
    }
    any_digit && (dots == 1 || dots == 2)
}

/// Distinct set of MC versions present across the project's file indexes,
/// preserving first-seen order.
fn collect_game_versions(indexes: &[FileIndex]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for idx in indexes {
        if is_mc_version(&idx.game_version) && !out.iter().any(|v| v == &idx.game_version) {
            out.push(idx.game_version.clone());
        }
    }
    out
}

fn authors_from(src: &[super::dto::ModAuthor]) -> Vec<Author> {
    src.iter()
        .map(|a| Author {
            name: a.name.clone(),
            url: a.url.clone(),
        })
        .collect()
}

fn icon_from(logo: Option<&super::dto::ModAsset>) -> Option<String> {
    logo.and_then(|l| l.thumbnail_url.clone().or_else(|| l.url.clone()))
}

fn gallery_from(screenshots: &[super::dto::ModAsset]) -> Vec<String> {
    screenshots
        .iter()
        .filter_map(|s| s.url.clone().or_else(|| s.thumbnail_url.clone()))
        .collect()
}

fn links_from(src: &super::dto::ModLinks) -> Links {
    Links {
        website: src.website_url.clone(),
        source: src.source_url.clone(),
        issues: src.issues_url.clone(),
        wiki: src.wiki_url.clone(),
        discord: None,
        donation: Vec::new(),
    }
}

fn base_from(m: &CfMod) -> ContentBase {
    ContentBase {
        id: m.id.to_string(),
        slug: m.slug.clone(),
        platform: Platform::CurseForge,
        title: m.name.clone(),
        summary: m.summary.clone(),
        // CF exposes the long description only via a separate `/description`
        // call — leave `None` here; consumers who want it can hit the raw
        // endpoint.
        description: None,
        authors: authors_from(&m.authors),
        icon_url: icon_from(m.logo.as_ref()),
        gallery: gallery_from(&m.screenshots),
        links: links_from(&m.links),
        downloads: m.download_count,
        // CurseForge has no "followers" concept. Thumbs-up is the closest
        // popularity signal and is what we surface under `follows`.
        follows: m.thumbs_up_count,
        // No license field anywhere in the CurseForge v1 response shape.
        license: None::<License>,
        created: m.date_created,
        updated: m.date_modified,
        categories: m.categories.iter().map(|c| c.name.clone()).collect(),
        game_versions: collect_game_versions(&m.latest_files_indexes),
    }
}

// ---------------------------------------------------------------------------
// Per-kind conversions
// ---------------------------------------------------------------------------

pub(crate) fn mod_from(m: CfMod) -> ModItem {
    let loaders = collect_loaders(&m.latest_files_indexes);
    ModItem {
        base: base_from(&m),
        loaders,
        // CF doesn't expose client/server-side split on the project object;
        // it's only inferable from per-file metadata we don't fetch here.
        client_side: SideSupport::Unknown,
        server_side: SideSupport::Unknown,
        dependencies: Vec::new(),
    }
}

pub(crate) fn pack_from(m: CfMod) -> PackItem {
    let loaders = collect_loaders(&m.latest_files_indexes);
    let base = base_from(&m);
    let mc_version = base.game_versions.first().cloned();
    PackItem {
        base,
        loaders,
        mc_version,
        // Would require parsing the modpack's manifest.json — skip for now.
        included_mods_count: None,
        has_server_pack: false,
    }
}

pub(crate) fn datapack_from(m: CfMod) -> DatapackItem {
    DatapackItem {
        base: base_from(&m),
        pack_format: None,
    }
}

pub(crate) fn resourcepack_from(m: CfMod) -> ResourcePackItem {
    ResourcePackItem {
        base: base_from(&m),
        pack_format: None,
        resolution: None,
    }
}

pub(crate) fn shaderpack_from(m: CfMod) -> ShaderPackItem {
    ShaderPackItem {
        base: base_from(&m),
        shader_loaders: Vec::new(),
    }
}

pub(crate) fn world_from(m: CfMod) -> WorldItem {
    WorldItem {
        base: base_from(&m),
        mc_version: None,
        size_bytes: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn class_ids_cover_every_content_type() {
        for kind in [
            ContentType::Mod,
            ContentType::Modpack,
            ContentType::ResourcePack,
            ContentType::ShaderPack,
            ContentType::Datapack,
            ContentType::World,
        ] {
            assert!(
                content_type_to_class_id(kind).is_some(),
                "missing classId for {kind:?}"
            );
        }
        assert_eq!(content_type_to_class_id(ContentType::Mod), Some(CLASS_MODS));
        assert_eq!(
            content_type_to_class_id(ContentType::Modpack),
            Some(CLASS_MODPACKS)
        );
        assert_eq!(
            content_type_to_class_id(ContentType::ShaderPack),
            Some(CLASS_SHADERS)
        );
    }

    #[test]
    fn sort_mapping_is_complete() {
        assert_eq!(sort_to_cf(Sort::Relevance), (1, "desc"));
        assert_eq!(sort_to_cf(Sort::Downloads), (6, "desc"));
        assert_eq!(sort_to_cf(Sort::Follows), (12, "desc"));
        assert_eq!(sort_to_cf(Sort::Latest), (11, "desc"));
        assert_eq!(sort_to_cf(Sort::Updated), (3, "desc"));
    }

    #[test]
    fn loader_name_covers_known_ids_and_drops_unknown() {
        assert_eq!(loader_name(LOADER_FORGE), Some("forge"));
        assert_eq!(loader_name(LOADER_FABRIC), Some("fabric"));
        assert_eq!(loader_name(LOADER_QUILT), Some("quilt"));
        assert_eq!(loader_name(LOADER_NEOFORGE), Some("neoforge"));
        assert_eq!(loader_name(LOADER_ANY), None);
        assert_eq!(loader_name(99), None);
    }

    #[test]
    fn mc_version_filter_is_strict() {
        assert!(is_mc_version("1.20"));
        assert!(is_mc_version("1.20.1"));
        assert!(is_mc_version("1.7.10"));
        assert!(!is_mc_version("Forge"));
        assert!(!is_mc_version("1.20-Snapshot"));
        assert!(!is_mc_version("Fabric"));
        assert!(!is_mc_version(""));
        assert!(!is_mc_version("1"));
        assert!(!is_mc_version("1.2.3.4"));
    }

    #[test]
    fn collect_loaders_dedupes_and_preserves_order() {
        let idx = vec![
            FileIndex {
                mod_loader: Some(LOADER_FORGE),
                ..dummy_index("1.20.1")
            },
            FileIndex {
                mod_loader: Some(LOADER_FABRIC),
                ..dummy_index("1.20.1")
            },
            FileIndex {
                mod_loader: Some(LOADER_FORGE),
                ..dummy_index("1.19.2")
            },
            FileIndex {
                mod_loader: Some(LOADER_ANY),
                ..dummy_index("1.19.2")
            },
        ];
        assert_eq!(collect_loaders(&idx), vec!["forge", "fabric"]);
    }

    #[test]
    fn collect_game_versions_filters_and_dedupes() {
        let idx = vec![
            dummy_index("1.20.1"),
            dummy_index("Forge"),
            dummy_index("1.19.2"),
            dummy_index("1.20.1"),
            dummy_index("Fabric"),
        ];
        assert_eq!(collect_game_versions(&idx), vec!["1.20.1", "1.19.2"]);
    }

    fn dummy_index(game_version: &str) -> FileIndex {
        FileIndex {
            game_version: game_version.to_string(),
            file_id: 0,
            filename: String::new(),
            release_type: None,
            game_version_type_id: None,
            mod_loader: None,
        }
    }
}
