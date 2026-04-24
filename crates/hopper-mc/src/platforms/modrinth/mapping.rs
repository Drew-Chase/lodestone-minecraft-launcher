//! Translation from Modrinth wire-format types into the crate's public model.

use crate::model::common::{Author, ContentBase, License, Links, SideSupport};
use crate::model::{
    DatapackItem, ModItem, PackItem, ResourcePackItem, ShaderPackItem, WorldItem,
};
use crate::platform::{ContentType, Platform, Sort};

use super::dto::{GalleryItem, Project, SearchHit};

/// Modrinth `project_type` values. We primarily reach these via facet filters
/// but they also appear on responses.
pub(crate) const PT_MOD: &str = "mod";
pub(crate) const PT_MODPACK: &str = "modpack";
pub(crate) const PT_RESOURCEPACK: &str = "resourcepack";
pub(crate) const PT_SHADER: &str = "shader";
pub(crate) const PT_DATAPACK: &str = "datapack";

/// Translate our [`Sort`] enum to the `index` query parameter accepted by
/// Modrinth's `/search` endpoint.
pub(crate) fn sort_to_index(sort: Sort) -> &'static str {
    match sort {
        Sort::Relevance => "relevance",
        Sort::Downloads => "downloads",
        Sort::Follows => "follows",
        Sort::Latest => "newest",
        Sort::Updated => "updated",
    }
}

/// Build the `facets` JSON array that restricts a search to a given content type.
///
/// Modrinth represents datapacks two ways depending on when the project was
/// uploaded: as `project_type=datapack`, or as `project_type=mod` with a
/// `categories=datapack` facet. We emit a single facet group here — the most
/// common/current shape — and tolerate the legacy form on deserialization.
pub(crate) fn content_type_to_facets(kind: ContentType) -> Option<String> {
    let pt = match kind {
        ContentType::Mod => PT_MOD,
        ContentType::Modpack => PT_MODPACK,
        ContentType::ResourcePack => PT_RESOURCEPACK,
        ContentType::ShaderPack => PT_SHADER,
        ContentType::Datapack => PT_DATAPACK,
        ContentType::World => return None, // unsupported
    };
    Some(format!("[[\"project_type:{pt}\"]]"))
}

fn side(s: Option<&str>) -> SideSupport {
    match s {
        Some("required") => SideSupport::Required,
        Some("optional") => SideSupport::Optional,
        Some("unsupported") => SideSupport::Unsupported,
        _ => SideSupport::Unknown,
    }
}

fn authors_from_single(author: Option<String>) -> Vec<Author> {
    author
        .map(|name| {
            vec![Author {
                name,
                url: None,
            }]
        })
        .unwrap_or_default()
}

fn gallery_urls_from_hit(hit: &SearchHit) -> Vec<String> {
    let mut urls = hit.gallery.clone();
    if let Some(f) = &hit.featured_gallery
        && !urls.iter().any(|u| u == f)
    {
        urls.insert(0, f.clone());
    }
    urls
}

fn gallery_urls_from_project(items: &[GalleryItem]) -> Vec<String> {
    // Featured first, then the rest in declared order.
    let mut out: Vec<String> = items
        .iter()
        .filter(|g| g.featured)
        .map(|g| g.url.clone())
        .collect();
    for g in items.iter().filter(|g| !g.featured) {
        out.push(g.url.clone());
    }
    out
}

fn license_from_string(s: Option<String>) -> Option<License> {
    s.map(|id| License {
        id,
        name: None,
        url: None,
    })
}

fn license_from_project(lic: Option<&super::dto::ProjectLicense>) -> Option<License> {
    lic.map(|l| License {
        id: l.id.clone(),
        name: l.name.clone(),
        url: l.url.clone(),
    })
}

/// Shared core: build a [`ContentBase`] from a search hit. Summary is duplicated
/// into the `summary` field; `description` is left `None` because search hits
/// don't carry the long body.
fn base_from_hit(hit: &SearchHit) -> ContentBase {
    ContentBase {
        id: hit.project_id.clone(),
        slug: hit.slug.clone(),
        platform: Platform::Modrinth,
        title: hit.title.clone(),
        summary: hit.description.clone(),
        description: None,
        authors: authors_from_single(hit.author.clone()),
        icon_url: hit.icon_url.clone(),
        gallery: gallery_urls_from_hit(hit),
        links: Links::default(),
        downloads: hit.downloads,
        follows: hit.follows,
        license: license_from_string(hit.license.clone()),
        created: hit.date_created,
        updated: hit.date_modified,
        categories: {
            let mut c = hit.categories.clone();
            for extra in &hit.display_categories {
                if !c.iter().any(|x| x == extra) {
                    c.push(extra.clone());
                }
            }
            c
        },
        game_versions: hit.versions.clone(),
    }
}

fn base_from_project(p: &Project) -> ContentBase {
    let mut categories = p.categories.clone();
    for extra in &p.additional_categories {
        if !categories.iter().any(|c| c == extra) {
            categories.push(extra.clone());
        }
    }

    ContentBase {
        id: p.id.clone(),
        slug: p.slug.clone(),
        platform: Platform::Modrinth,
        title: p.title.clone(),
        summary: p.description.clone(),
        description: if p.body.is_empty() { None } else { Some(p.body.clone()) },
        // The search response exposes `author`, but a full project only
        // exposes a `team` id — the actual member list lives behind a
        // separate endpoint. Callers that need authors should issue the
        // team lookup themselves; we leave this empty rather than guess.
        authors: Vec::new(),
        icon_url: p.icon_url.clone(),
        gallery: gallery_urls_from_project(&p.gallery),
        links: Links {
            website: None,
            source: p.source_url.clone(),
            issues: p.issues_url.clone(),
            wiki: p.wiki_url.clone(),
            discord: p.discord_url.clone(),
            donation: p.donation_urls.iter().map(|d| d.url.clone()).collect(),
        },
        downloads: p.downloads,
        follows: p.followers,
        license: license_from_project(p.license.as_ref()),
        created: p.published,
        updated: p.updated,
        categories,
        game_versions: p.game_versions.clone(),
    }
}

// ---------------------------------------------------------------------------
// Per-kind conversions
// ---------------------------------------------------------------------------

pub(crate) fn mod_from_hit(hit: SearchHit) -> ModItem {
    let client = side(hit.client_side.as_deref());
    let server = side(hit.server_side.as_deref());
    let base = base_from_hit(&hit);
    // Modrinth search hits don't break out loaders; they appear in `categories`
    // alongside real categories. We surface them both via `categories` (for now)
    // and best-effort filter into `loaders` so consumers can check loader
    // compatibility without hitting the full project endpoint.
    let loaders = filter_loaders(&base.categories);
    ModItem {
        base,
        loaders,
        client_side: client,
        server_side: server,
        dependencies: Vec::new(),
    }
}

pub(crate) fn mod_from_project(p: Project) -> ModItem {
    let client = side(p.client_side.as_deref());
    let server = side(p.server_side.as_deref());
    let loaders = if !p.loaders.is_empty() {
        p.loaders.clone()
    } else {
        filter_loaders(&p.categories)
    };
    ModItem {
        base: base_from_project(&p),
        loaders,
        client_side: client,
        server_side: server,
        // Full dependency resolution requires `/project/{id}/version` — leave
        // empty until that endpoint is wired. Callers that need dependencies
        // should issue the versions lookup themselves.
        dependencies: Vec::new(),
    }
}

pub(crate) fn pack_from_hit(hit: SearchHit) -> PackItem {
    let base = base_from_hit(&hit);
    let loaders = filter_loaders(&base.categories);
    let mc_version = base.game_versions.first().cloned();
    PackItem {
        base,
        loaders,
        mc_version,
        included_mods_count: None,
        has_server_pack: false,
    }
}

pub(crate) fn pack_from_project(p: Project) -> PackItem {
    let loaders = if !p.loaders.is_empty() {
        p.loaders.clone()
    } else {
        filter_loaders(&p.categories)
    };
    let base = base_from_project(&p);
    let mc_version = base.game_versions.first().cloned();
    PackItem {
        base,
        loaders,
        mc_version,
        included_mods_count: None,
        has_server_pack: false,
    }
}

pub(crate) fn datapack_from_hit(hit: SearchHit) -> DatapackItem {
    DatapackItem {
        base: base_from_hit(&hit),
        pack_format: None,
    }
}

pub(crate) fn datapack_from_project(p: Project) -> DatapackItem {
    DatapackItem {
        base: base_from_project(&p),
        pack_format: None,
    }
}

pub(crate) fn resourcepack_from_hit(hit: SearchHit) -> ResourcePackItem {
    ResourcePackItem {
        base: base_from_hit(&hit),
        pack_format: None,
        resolution: None,
    }
}

pub(crate) fn resourcepack_from_project(p: Project) -> ResourcePackItem {
    ResourcePackItem {
        base: base_from_project(&p),
        pack_format: None,
        resolution: None,
    }
}

pub(crate) fn shaderpack_from_hit(hit: SearchHit) -> ShaderPackItem {
    ShaderPackItem {
        base: base_from_hit(&hit),
        shader_loaders: Vec::new(),
    }
}

pub(crate) fn shaderpack_from_project(p: Project) -> ShaderPackItem {
    ShaderPackItem {
        base: base_from_project(&p),
        shader_loaders: Vec::new(),
    }
}

#[allow(dead_code)]
pub(crate) fn world_from_hit(hit: SearchHit) -> WorldItem {
    // Modrinth doesn't serve worlds; kept for symmetry in case that changes.
    WorldItem {
        base: base_from_hit(&hit),
        mc_version: None,
        size_bytes: None,
    }
}

/// Loader names known to appear in Modrinth's `categories` array.
///
/// Keeping a small allowlist rather than a full taxonomy — new entries are a
/// one-line change and the miss is harmless (just absent from `loaders`).
const KNOWN_LOADERS: &[&str] = &[
    "fabric",
    "forge",
    "neoforge",
    "quilt",
    "liteloader",
    "modloader",
    "risugamis-modloader",
    "rift",
    "bukkit",
    "spigot",
    "paper",
    "purpur",
    "sponge",
    "bungeecord",
    "velocity",
    "waterfall",
    "folia",
    "iris",
    "optifine",
    "canvas",
    "vanilla",
    "minecraft",
    "datapack",
];

fn filter_loaders(categories: &[String]) -> Vec<String> {
    categories
        .iter()
        .filter(|c| KNOWN_LOADERS.iter().any(|k| k.eq_ignore_ascii_case(c)))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_index_mapping() {
        assert_eq!(sort_to_index(Sort::Relevance), "relevance");
        assert_eq!(sort_to_index(Sort::Downloads), "downloads");
        assert_eq!(sort_to_index(Sort::Follows), "follows");
        assert_eq!(sort_to_index(Sort::Latest), "newest");
        assert_eq!(sort_to_index(Sort::Updated), "updated");
    }

    #[test]
    fn content_type_facets() {
        assert_eq!(
            content_type_to_facets(ContentType::Mod).as_deref(),
            Some("[[\"project_type:mod\"]]")
        );
        assert_eq!(
            content_type_to_facets(ContentType::Modpack).as_deref(),
            Some("[[\"project_type:modpack\"]]")
        );
        assert_eq!(
            content_type_to_facets(ContentType::ResourcePack).as_deref(),
            Some("[[\"project_type:resourcepack\"]]")
        );
        assert_eq!(
            content_type_to_facets(ContentType::ShaderPack).as_deref(),
            Some("[[\"project_type:shader\"]]")
        );
        assert_eq!(
            content_type_to_facets(ContentType::Datapack).as_deref(),
            Some("[[\"project_type:datapack\"]]")
        );
        assert!(content_type_to_facets(ContentType::World).is_none());
    }

    #[test]
    fn loader_filter_keeps_only_known_loaders() {
        let cats = vec![
            "fabric".to_string(),
            "technology".to_string(),
            "forge".to_string(),
            "adventure".to_string(),
        ];
        let loaders = filter_loaders(&cats);
        assert_eq!(loaders, vec!["fabric", "forge"]);
    }
}
