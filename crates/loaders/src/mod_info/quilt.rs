//! `quilt.mod.json` parsing — modcrawl has no quilt metadata support yet, so
//! this is the one metadata format implemented in this crate.
//!
//! Schema: https://quiltmc.org/en/spec/quilt-mod-json-spec/ (v1). The loader
//! `version` may be a plain string or a maven version-range object; contributors
//! may be a `{ name: role }` map or a `[{ name, role }]` list; `license` may be
//! a string or an array of strings. All are handled.

use std::path::Path;

use serde::Deserialize;
use zipcrawl::ZipManager;

use crate::error::LoaderError;
use crate::meta::{ModDependency, ModDependencyKind, ModInfo};

pub(crate) fn extract(jar: &Path) -> Result<ModInfo, LoaderError> {
    let mut archive = ZipManager::new(jar).map_err(|source| LoaderError::ModParse {
        path: jar.to_path_buf(),
        source: Box::new(source),
    })?;
    let raw = archive
        .read_to_string("quilt.mod.json")
        .map_err(|_source| LoaderError::ModMetadataMissing { path: jar.to_path_buf() })?;

    let document: QuiltModJson = serde_json::from_str(&raw).map_err(|source| LoaderError::ModParse {
        path: jar.to_path_buf(),
        source: Box::new(source),
    })?;

    let loader = document.quilt_loader;
    let metadata = loader.metadata.unwrap_or_default();
    // Compute the borrow-taking fields before partially moving `metadata`.
    let authors = metadata.authors();
    let license = metadata.license_string();
    let mut dependencies = loader
        .depends
        .into_iter()
        .map(|entry| entry.dependency(ModDependencyKind::Required))
        .collect::<Vec<_>>();
    dependencies.extend(loader.breaks.into_iter().map(|entry| entry.dependency(ModDependencyKind::Incompatible)));

    Ok(ModInfo {
        loader: "quilt",
        id: loader.id,
        version: loader.version.as_string(),
        name: metadata.name,
        description: metadata.description,
        authors,
        license,
        homepage: metadata.contact.and_then(|contact| contact.homepage),
        dependencies,
    })
}

#[derive(Debug, Deserialize)]
struct QuiltModJson {
    quilt_loader: QuiltLoader,
}

#[derive(Debug, Deserialize)]
struct QuiltLoader {
    id: String,
    /// A plain version string or a maven-style range object; see [`QuiltVersion`].
    version: QuiltVersion,
    #[serde(default)]
    metadata: Option<QuiltMetadata>,
    #[serde(default)]
    depends: Vec<QuiltDependency>,
    #[serde(default)]
    breaks: Vec<QuiltDependency>,
}

/// The loader `version` field: `"1.0.0"` or `{ "width": 1, "points": [...] }`
/// for maven-style ranges. The raw JSON text is what a caller wants to see.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum QuiltVersion {
    Plain(String),
    Structured(serde_json::Value),
}

impl QuiltVersion {
    fn as_string(&self) -> Option<String> {
        match self {
            QuiltVersion::Plain(version) => Some(version.clone()),
            QuiltVersion::Structured(value) => serde_json::to_string(value).ok(),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
struct QuiltMetadata {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    contributors: Option<Contributors>,
    #[serde(default)]
    contact: Option<QuiltContact>,
    #[serde(default)]
    license: Option<QuiltVersion>,
}

impl QuiltMetadata {
    fn authors(&self) -> Vec<String> {
        self.contributors.as_ref().map(Contributors::names).unwrap_or_default()
    }

    fn license_string(&self) -> Option<String> {
        self.license.as_ref().and_then(QuiltVersion::as_string)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct QuiltContact {
    #[serde(default)]
    homepage: Option<String>,
}

/// Contributors come as `{ "Alice": "Author" }` or `[ { "name": "Alice", "role": "Author" } ]`.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Contributors {
    Map(std::collections::BTreeMap<String, String>),
    List(Vec<Contributor>),
}

impl Contributors {
    fn names(&self) -> Vec<String> {
        match self {
            Contributors::Map(entries) => entries.keys().cloned().collect(),
            Contributors::List(contributors) => contributors.iter().map(|contributor| contributor.name.clone()).collect(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct Contributor {
    name: String,
}

#[derive(Debug, Deserialize)]
struct QuiltDependency {
    id: String,
    /// When present, the dependency only applies in the absence of `unless` —
    /// i.e. the dependency is effectively optional.
    #[serde(default)]
    unless: Option<serde_json::Value>,
    #[serde(default)]
    version: Option<QuiltVersion>,
}

impl QuiltDependency {
    fn dependency(self, base_kind: ModDependencyKind) -> ModDependency {
        let kind = if self.unless.is_some() { ModDependencyKind::Optional } else { base_kind };
        let version_range: Option<String> = self.version.as_ref().and_then(QuiltVersion::as_string);
        ModDependency {
            id: self.id,
            kind,
            version_range,
        }
    }
}

#[cfg(test)]
#[path = "../../tests/mod_info/quilt.rs"]
mod tests;
