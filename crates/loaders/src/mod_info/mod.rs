//! Extraction of client-mod metadata from jar files.
//!
//! The heavy lifting is delegated to `modcrawl` (sentinel detection, JSON/TOML
//! parsing for fabric / forge mods.toml / neoforge.mods.toml / legacy mcmod.info,
//! plus dependency analysis with jar-in-jar handling). Only quilt's
//! `quilt.mod.json` is parsed locally — see [`quilt`] — because modcrawl has no
//! quilt metadata support yet.
//!
//! modcrawl's inner metadata types are `pub(crate)`, so the match arms below
//! work on them by inference and reshape them into the neutral [`ModEntry`] /
//! [`MetadataFallbacks`] structs here.

use std::path::Path;

use modcrawl::{DepKind, ModMetadata};
use serde_json::Value;

use crate::error::LoaderError;
use crate::meta::{ModDependency, ModDependencyKind, ModInfo};

pub(crate) mod quilt;

/// The `[[mods]]` entry of a `mods.toml`-shaped metadata, flattened.
struct ModEntry {
    id: String,
    version: Option<String>,
    name: Option<String>,
    description: Option<String>,
    authors: Option<Value>,
    homepage: Option<String>,
    license: Option<String>,
}

/// The top-level fields a mod entry falls back to.
struct MetadataFallbacks {
    license: Option<String>,
    display_url: Option<String>,
}

/// Extracts metadata from one jar. The detected format wins: `ModInfo.loader`
/// reports what was actually found, never the requested kind.
pub(crate) fn extract(jar: &Path) -> Result<ModInfo, LoaderError> {
    let metadata = modcrawl::read_metadata(jar).map_err(|error| match error {
        modcrawl::Error::UnsupportedMetadata(_) => LoaderError::ModMetadataMissing { path: jar.to_path_buf() },
        other => LoaderError::ModParse {
            path: jar.to_path_buf(),
            source: Box::new(other),
        },
    })?;

    let mut info = match metadata {
        ModMetadata::Fabric(m) => {
            let contact = m.contact;
            ModInfo {
                loader: "fabric",
                id: m.id,
                version: Some(m.version),
                name: m.name,
                description: m.description,
                authors: m.authors.iter().filter_map(author_name).collect(),
                license: m.license,
                homepage: contact.and_then(|contact| contact.homepage),
                dependencies: Vec::new(),
            }
        }
        ModMetadata::ForgeModern(m) => {
            let m = *m;
            let entry = m.mods.into_iter().next().map(|mod_entry| ModEntry {
                id: mod_entry.mod_id,
                version: mod_entry.version,
                name: mod_entry.display_name,
                description: mod_entry.description,
                authors: mod_entry.authors,
                homepage: mod_entry.display_url,
                license: mod_entry.license,
            });
            entry_info(
                jar,
                entry,
                MetadataFallbacks {
                    license: m.license,
                    display_url: m.display_url,
                },
                "forge",
            )?
        }
        ModMetadata::NeoForge(m) => {
            let m = *m;
            let entry = m.mods.into_iter().next().map(|mod_entry| ModEntry {
                id: mod_entry.mod_id,
                version: mod_entry.version,
                name: mod_entry.display_name,
                description: mod_entry.description,
                authors: mod_entry.authors,
                homepage: mod_entry.display_url,
                license: mod_entry.license,
            });
            entry_info(
                jar,
                entry,
                MetadataFallbacks {
                    license: m.license,
                    display_url: m.display_url,
                },
                "neoforge",
            )?
        }
        ModMetadata::ForgeLegacy(entries) => {
            let Some(entry) = entries.into_iter().next() else {
                return Err(LoaderError::ModMetadataMissing { path: jar.to_path_buf() });
            };
            let mut authors = entry.author_list;
            authors.extend(entry.authors);
            ModInfo {
                loader: "forge",
                id: entry.modid,
                version: entry.version,
                name: entry.name,
                description: entry.description,
                authors,
                license: None,
                homepage: entry.url,
                dependencies: Vec::new(),
            }
        }
        // Server-plugin metadata is out of scope: Lodestone launches clients.
        ModMetadata::Bukkit(_) | ModMetadata::Paper(_) => {
            return Err(LoaderError::ModMetadataMissing { path: jar.to_path_buf() });
        }
    };

    info.dependencies = dependencies(jar)?;
    Ok(info)
}

/// Builds the `ModInfo` for a `mods.toml`-shaped metadata (forge and neoforge
/// share the schema). The first `[[mods]]` entry is the jar's primary mod; the
/// loader-level license / display URL are fallbacks for the mod entry's.
fn entry_info(jar: &Path, entry: Option<ModEntry>, fallbacks: MetadataFallbacks, loader: &'static str) -> Result<ModInfo, LoaderError> {
    let Some(entry) = entry else {
        // A mods.toml without any [[mods]] entry carries nothing usable.
        return Err(LoaderError::ModMetadataMissing { path: jar.to_path_buf() });
    };
    Ok(ModInfo {
        loader,
        id: entry.id,
        version: entry.version,
        name: entry.name,
        description: entry.description,
        authors: entry.authors.as_ref().map(authors_field).unwrap_or_default(),
        license: entry.license.or(fallbacks.license),
        homepage: entry.homepage.or(fallbacks.display_url),
        dependencies: Vec::new(),
    })
}

/// Pulls author names out of the shapes the formats use: plain strings,
/// fabric's `{ "name": ..., "contact": ... }` objects, or arrays of either.
fn author_name(value: &Value) -> Option<String> {
    match value {
        Value::String(name) => Some(name.clone()),
        Value::Object(object) => object.get("name").and_then(Value::as_str).map(str::to_string),
        _ => None,
    }
}

/// Flattens forge's authors field: a comma-separated string or an array.
fn authors_field(value: &Value) -> Vec<String> {
    match value {
        Value::String(authors) => authors
            .split(',')
            .map(str::trim)
            .filter(|author| !author.is_empty())
            .map(str::to_string)
            .collect(),
        Value::Array(authors) => authors.iter().filter_map(author_name).collect(),
        _ => Vec::new(),
    }
}

/// Re-runs modcrawl's dependency analysis and maps it onto [`ModDependency`].
fn dependencies(jar: &Path) -> Result<Vec<ModDependency>, LoaderError> {
    let report = modcrawl::analyze(jar, false).map_err(|source| LoaderError::ModParse {
        path: jar.to_path_buf(),
        source: Box::new(source),
    })?;
    Ok(report
        .dependencies
        .into_iter()
        .map(|entry| ModDependency {
            id: entry.name,
            kind: match entry.kind {
                DepKind::Required => ModDependencyKind::Required,
                DepKind::LoadBefore => ModDependencyKind::LoadBefore,
                DepKind::Optional => ModDependencyKind::Optional,
                DepKind::Recommended => ModDependencyKind::Recommended,
                DepKind::Suggested => ModDependencyKind::Suggested,
                DepKind::Incompatible => ModDependencyKind::Incompatible,
                DepKind::Discouraged => ModDependencyKind::Discouraged,
            },
            version_range: entry.version_range.raw,
        })
        .collect())
}
