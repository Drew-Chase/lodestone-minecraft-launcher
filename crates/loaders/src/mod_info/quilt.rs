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
mod tests {
    use super::*;

    const FULL: &str = r#"{
        "schema_version": 1,
        "quilt_loader": {
            "group": "org.example",
            "id": "testmod",
            "version": {
                "width": 1,
                "points": ["1.0.0"]
            },
            "metadata": {
                "name": "Test Mod",
                "description": "A test mod",
                "contributors": { "Alice": "Owner", "Bob": "Maintainer" },
                "contact": { "homepage": "https://example.com" },
                "license": ["MIT", "Apache-2.0"]
            },
            "depends": [
                { "id": "quilt_loader", "version": ">=0.17.0" },
                { "id": "fabric-api", "version": "*", "unless": "qsl" }
            ],
            "breaks": [ { "id": "evil-mod" } ]
        }
    }"#;

    fn parse(document: &str) -> ModInfo {
        let json: QuiltModJson = serde_json::from_str(document).expect("quilt.mod.json parses");
        let loader = json.quilt_loader;
        let metadata = loader.metadata.unwrap_or_default();
        // Borrow-taking fields before the partial moves, same as `extract`.
        let authors = metadata.authors();
        let license = metadata.license_string();
        ModInfo {
            loader: "quilt",
            id: loader.id,
            version: loader.version.as_string(),
            name: metadata.name,
            description: metadata.description,
            authors,
            license,
            homepage: metadata.contact.and_then(|contact| contact.homepage),
            dependencies: {
                let mut dependencies = loader
                    .depends
                    .into_iter()
                    .map(|entry| entry.dependency(ModDependencyKind::Required))
                    .collect::<Vec<_>>();
                dependencies.extend(loader.breaks.into_iter().map(|entry| entry.dependency(ModDependencyKind::Incompatible)));
                dependencies
            },
        }
    }

    #[test]
    fn the_full_schema_maps_onto_mod_info() {
        let info = parse(FULL);
        assert_eq!(info.id, "testmod");
        // Structured (maven-range object) versions serialize back to raw JSON;
        // serde_json maps sort keys, so "points" precedes "width".
        assert_eq!(info.version.as_deref(), Some(r#"{"points":["1.0.0"],"width":1}"#));
        assert_eq!(info.name.as_deref(), Some("Test Mod"));
        assert_eq!(info.description.as_deref(), Some("A test mod"));
        assert_eq!(info.authors, ["Alice", "Bob"]);
        // License arrays serialize back to raw JSON.
        assert_eq!(info.license.as_deref(), Some(r#"["MIT","Apache-2.0"]"#));
        assert_eq!(info.homepage.as_deref(), Some("https://example.com"));

        assert_eq!(info.dependencies.len(), 3);
        assert_eq!(info.dependencies[0].id, "quilt_loader");
        assert_eq!(info.dependencies[0].kind, ModDependencyKind::Required);
        assert_eq!(info.dependencies[1].id, "fabric-api");
        assert_eq!(info.dependencies[1].kind, ModDependencyKind::Optional, "`unless` marks it optional");
        assert_eq!(info.dependencies[2].id, "evil-mod");
        assert_eq!(info.dependencies[2].kind, ModDependencyKind::Incompatible);
    }

    #[test]
    fn contributor_lists_and_plain_versions_work_too() {
        let info = parse(
            r#"{
                "schema_version": 1,
                "quilt_loader": {
                    "id": "tiny",
                    "version": "2.1.0",
                    "metadata": {
                        "contributors": [ { "name": "Cara" }, { "name": "Dan", "role": "Artist" } ]
                    }
                }
            }"#,
        );
        assert_eq!(info.version.as_deref(), Some("2.1.0"));
        assert_eq!(info.authors, ["Cara", "Dan"]);
        assert!(info.license.is_none());
        assert!(info.dependencies.is_empty());
    }

    #[test]
    fn missing_metadata_section_defaults() {
        let info = parse(r#"{ "schema_version": 1, "quilt_loader": { "id": "bare", "version": "0.1" } }"#);
        assert_eq!(info.id, "bare");
        assert_eq!(info.version.as_deref(), Some("0.1"));
        assert_eq!(info.authors, Vec::<String>::new());
    }
}
