use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read as IoRead;
use std::path::{Path, PathBuf};
use thiserror::Error;
use zip::ZipArchive;

#[derive(Debug, Error)]
pub enum FabricModJsonError {
    #[error("Failed to open JAR file at {path}: {source}")]
    FileOpen {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("Failed to read JAR as ZIP archive: {0}")]
    ZipRead(#[from] zip::result::ZipError),
    #[error("fabric.mod.json not found in JAR")]
    JsonNotFound,
    #[error("Failed to read fabric.mod.json contents: {0}")]
    JsonRead(#[source] std::io::Error),
    #[error("Failed to parse fabric.mod.json: {0}")]
    JsonParse(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    #[serde(rename = "*")]
    All,
    Client,
    Server,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FabricModJson {
    /// Schema version - must be 1
    pub schema_version: i32,
    /// Mod identifier (2-64 chars: Latin letters, digits, underscores)
    pub id: String,
    /// Mod version
    pub version: String,
    /// User-friendly mod name (defaults to id if not specified)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Mod description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// PNG icon path(s) - either a string path or object mapping widths to paths
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<Icon>,
    /// Mod creators
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authors: Option<Vec<Person>>,
    /// Contributors to the mod
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contributors: Option<Vec<Person>>,
    /// Project contact information
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contact: Option<ContactInfo>,
    /// SPDX license identifier(s)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<License>,
    /// Execution context: *, client, or server
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment: Option<Environment>,
    /// Main mod classes to load
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entrypoints: Option<HashMap<String, Vec<EntryPoint>>>,
    /// List of mod ID aliases
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provides: Option<Vec<String>>,
    /// Required dependencies
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub depends: Option<HashMap<String, DependencyVersion>>,
    /// Recommended dependencies (warnings if missing)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recommends: Option<HashMap<String, DependencyVersion>>,
    /// Suggested dependencies (metadata only)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggests: Option<HashMap<String, DependencyVersion>>,
    /// Incompatible mods (crashes if present)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub breaks: Option<HashMap<String, DependencyVersion>>,
    /// Problematic mods (warnings if present)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conflicts: Option<HashMap<String, DependencyVersion>>,
    /// Language adapter mappings
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language_adapters: Option<HashMap<String, String>>,
    /// Mixin configuration files
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mixins: Option<Vec<MixinConfig>>,
    /// Nested JAR files
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jars: Option<Vec<JarInfo>>,
    /// Access widener file path
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub access_widener: Option<String>,
    /// Custom data (namespaced recommended)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom: Option<HashMap<String, Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Icon {
    /// Single icon path
    Single(String),
    /// Map of icon sizes to paths (e.g., "16": "icon-16.png")
    Multiple(HashMap<String, String>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Person {
    /// Simple string name
    Name(String),
    /// Person with name and contact info
    Detailed(PersonDetails),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonDetails {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contact: Option<ContactInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactInfo {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub irc: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issues: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sources: Option<String>,
    /// Additional arbitrary contact fields
    #[serde(flatten)]
    pub additional: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum License {
    /// Single license identifier
    Single(String),
    /// Multiple licenses
    Multiple(Vec<String>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EntryPoint {
    /// Class name or method/field reference
    String(String),
    /// Detailed entrypoint with adapter
    Object(EntryPointObject),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryPointObject {
    /// Adapter to use (e.g., "default", "kotlin")
    pub adapter: Option<String>,
    /// Value (class name, method reference, etc.)
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DependencyVersion {
    /// Single version range string
    Single(String),
    /// Multiple version ranges (OR logic)
    Multiple(Vec<String>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MixinConfig {
    /// Simple mixin config file path
    String(String),
    /// Mixin config with environment specification
    Object(MixinConfigObject),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MixinConfigObject {
    /// Mixin config file path
    pub config: String,
    /// Environment where this mixin applies
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment: Option<Environment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JarInfo {
    pub file: String,
}

impl FabricModJson {
    pub fn from_jar(path: impl AsRef<Path>) -> Result<Self, FabricModJsonError> {
        let path = path.as_ref();
        let file = File::open(path).map_err(|source| FabricModJsonError::FileOpen {
            path: path.to_path_buf(),
            source,
        })?;

        let mut archive = ZipArchive::new(file)?;

        let mut json_file = archive.by_name("fabric.mod.json").map_err(|e| match e {
            zip::result::ZipError::FileNotFound => FabricModJsonError::JsonNotFound,
            other => FabricModJsonError::ZipRead(other),
        })?;

        let mut contents = String::new();
        json_file
            .read_to_string(&mut contents)
            .map_err(FabricModJsonError::JsonRead)?;

        Ok(serde_json::from_str(&contents)?)
    }
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::io::Write;
	use zip::write::SimpleFileOptions;
	use zip::ZipWriter;

	fn create_test_jar_with_json(json_content: &str) -> Vec<u8> {
        let mut buffer = Vec::new();
        {
            let mut zip = ZipWriter::new(std::io::Cursor::new(&mut buffer));
            zip.start_file("fabric.mod.json", SimpleFileOptions::default())
                .unwrap();
            zip.write_all(json_content.as_bytes()).unwrap();
            zip.finish().unwrap();
        }
        buffer
    }

    #[test]
    fn test_minimal_fabric_mod_json() {
        let json = r#"{
			"schemaVersion": 1,
			"id": "test-mod",
			"version": "1.0.0"
		}"#;

        let jar_data = create_test_jar_with_json(json);
        let temp_dir = std::env::temp_dir();
        let jar_path = temp_dir.join("test_minimal.jar");
        std::fs::write(&jar_path, jar_data).unwrap();

        let result = FabricModJson::from_jar(&jar_path);
        assert!(result.is_ok());

        let mod_json = result.unwrap();
        assert_eq!(mod_json.id, "test-mod");
        assert_eq!(mod_json.version, "1.0.0");
        assert_eq!(mod_json.schema_version, 1);

        std::fs::remove_file(&jar_path).ok();
    }

    #[test]
    fn test_full_fabric_mod_json() {
        let json = r#"{
			"schemaVersion": 1,
			"id": "example-mod",
			"version": "2.1.0",
			"name": "Example Mod",
			"description": "A test mod",
			"environment": "client",
			"authors": ["TestAuthor"],
			"contact": {
				"homepage": "https://example.com",
				"sources": "https://github.com/example/mod"
			},
			"depends": {
				"minecraft": "~1.20",
				"fabricloader": ">=0.15.0"
			}
		}"#;

        let jar_data = create_test_jar_with_json(json);
        let temp_dir = std::env::temp_dir();
        let jar_path = temp_dir.join("test_full.jar");
        std::fs::write(&jar_path, jar_data).unwrap();

        let result = FabricModJson::from_jar(&jar_path);
        assert!(result.is_ok());

        let mod_json = result.unwrap();
        assert_eq!(mod_json.id, "example-mod");
        assert_eq!(mod_json.environment, Some(Environment::Client));
        assert!(mod_json.description.is_some());
        assert!(mod_json.contact.is_some());
        assert!(mod_json.depends.is_some());

        std::fs::remove_file(&jar_path).ok();
    }

    #[test]
    fn test_environment_variants() {
        let test_cases = vec![
            (
                r#"{"schemaVersion":1,"id":"test","version":"1.0","name":"Test","environment":"*"}"#,
                Environment::All,
            ),
            (
                r#"{"schemaVersion":1,"id":"test","version":"1.0","name":"Test","environment":"client"}"#,
                Environment::Client,
            ),
            (
                r#"{"schemaVersion":1,"id":"test","version":"1.0","name":"Test","environment":"server"}"#,
                Environment::Server,
            ),
        ];

        for (json, expected_env) in test_cases {
            let jar_data = create_test_jar_with_json(json);
            let temp_dir = std::env::temp_dir();
            let jar_path = temp_dir.join(format!("test_env_{:?}.jar", expected_env));
            std::fs::write(&jar_path, jar_data).unwrap();

            let result = FabricModJson::from_jar(&jar_path);
            assert!(result.is_ok());
            assert_eq!(result.unwrap().environment, Some(expected_env));

            std::fs::remove_file(&jar_path).ok();
        }
    }

    #[test]
    fn test_missing_fabric_mod_json() {
        let mut buffer = Vec::new();
        {
            let mut zip = ZipWriter::new(std::io::Cursor::new(&mut buffer));
            zip.start_file("some_other_file.txt", SimpleFileOptions::default())
                .unwrap();
            zip.write_all(b"content").unwrap();
            zip.finish().unwrap();
        }

        let temp_dir = std::env::temp_dir();
        let jar_path = temp_dir.join("test_no_json.jar");
        std::fs::write(&jar_path, buffer).unwrap();

        let result = FabricModJson::from_jar(&jar_path);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            FabricModJsonError::JsonNotFound
        ));

        std::fs::remove_file(&jar_path).ok();
    }

    #[test]
    fn test_invalid_json() {
        let invalid_json = r#"{"not valid json"#;

        let jar_data = create_test_jar_with_json(invalid_json);
        let temp_dir = std::env::temp_dir();
        let jar_path = temp_dir.join("test_invalid.jar");
        std::fs::write(&jar_path, jar_data).unwrap();

        let result = FabricModJson::from_jar(&jar_path);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            FabricModJsonError::JsonParse(_)
        ));

        std::fs::remove_file(&jar_path).ok();
    }

    #[test]
    fn test_file_not_found() {
        let result = FabricModJson::from_jar("/nonexistent/path/to/mod.jar");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            FabricModJsonError::FileOpen { .. }
        ));
    }

    #[test]
    fn test_serialization_roundtrip() {
        let mod_json = FabricModJson {
            schema_version: 1,
            id: "test".to_string(),
            version: "1.0.0".to_string(),
            name: Some("Test".to_string()),
            description: Some("A test mod".to_string()),
            icon: None,
            authors: None,
            contributors: None,
            contact: None,
            license: None,
            environment: Some(Environment::All),
            entrypoints: None,
            provides: None,
            depends: None,
            recommends: None,
            suggests: None,
            breaks: None,
            conflicts: None,
            language_adapters: None,
            mixins: None,
            jars: None,
            access_widener: None,
            custom: None,
        };

        let json = serde_json::to_string(&mod_json).unwrap();
        let deserialized: FabricModJson = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, mod_json.id);
        assert_eq!(deserialized.version, mod_json.version);
        assert_eq!(deserialized.environment, mod_json.environment);
    }

    #[test]
    fn test_comprehensive_fabric_mod_json() {
        let json = r#"{
			"schemaVersion": 1,
			"id": "comprehensive-mod",
			"version": "3.2.1",
			"name": "Comprehensive Test Mod",
			"description": "A mod demonstrating all fabric.mod.json features",
			"icon": "icon.png",
			"authors": [
				"Author Name",
				{
					"name": "Detailed Author",
					"contact": {
						"email": "author@example.com"
					}
				}
			],
			"contributors": ["Contributor 1", "Contributor 2"],
			"contact": {
				"email": "contact@example.com",
				"irc": "irc://irc.example.com/#modchannel",
				"homepage": "https://example.com",
				"issues": "https://github.com/example/mod/issues",
				"sources": "https://github.com/example/mod"
			},
			"license": ["MIT", "Apache-2.0"],
			"environment": "client",
			"provides": ["old-mod-id", "legacy-mod-id"],
			"entrypoints": {
				"main": ["com.example.MainClass"],
				"client": ["com.example.ClientClass"],
				"server": [
					{
						"adapter": "kotlin",
						"value": "com.example.KotlinServerClass"
					}
				]
			},
			"depends": {
				"fabricloader": ">=0.15.0",
				"minecraft": ["~1.20", "~1.20.1"]
			},
			"recommends": {
				"modmenu": "*"
			},
			"suggests": {
				"sodium": ">=0.5.0"
			},
			"breaks": {
				"incompatible-mod": "*"
			},
			"conflicts": {
				"problematic-mod": "<2.0.0"
			},
			"languageAdapters": {
				"kotlin": "net.fabricmc.language.kotlin.KotlinAdapter"
			},
			"mixins": [
				"modid.mixins.json",
				{
					"config": "modid.client.mixins.json",
					"environment": "client"
				}
			],
			"jars": [
				{
					"file": "nested-lib.jar"
				}
			],
			"accessWidener": "modid.accesswidener",
			"custom": {
				"modid:custom_field": "custom_value",
				"loom:injected_interfaces": {
					"net/minecraft/class_1234": ["com/example/Interface"]
				}
			}
		}"#;

        let jar_data = create_test_jar_with_json(json);
        let temp_dir = std::env::temp_dir();
        let jar_path = temp_dir.join("test_comprehensive.jar");
        std::fs::write(&jar_path, jar_data).unwrap();

        let result = FabricModJson::from_jar(&jar_path);
        assert!(result.is_ok());

        let mod_json = result.unwrap();
        assert_eq!(mod_json.id, "comprehensive-mod");
        assert_eq!(mod_json.version, "3.2.1");
        assert_eq!(mod_json.name, Some("Comprehensive Test Mod".to_string()));
        assert_eq!(mod_json.environment, Some(Environment::Client));

        // Check provides
        assert!(mod_json.provides.is_some());
        let provides = mod_json.provides.unwrap();
        assert_eq!(provides.len(), 2);

        // Check authors
        assert!(mod_json.authors.is_some());
        let authors = mod_json.authors.unwrap();
        assert_eq!(authors.len(), 2);

        // Check contributors
        assert!(mod_json.contributors.is_some());

        // Check license
        assert!(mod_json.license.is_some());

        // Check contact
        assert!(mod_json.contact.is_some());
        let contact = mod_json.contact.unwrap();
        assert!(contact.email.is_some());
        assert!(contact.homepage.is_some());

        // Check entrypoints
        assert!(mod_json.entrypoints.is_some());
        let entrypoints = mod_json.entrypoints.unwrap();
        assert!(entrypoints.contains_key("main"));
        assert!(entrypoints.contains_key("client"));
        assert!(entrypoints.contains_key("server"));

        // Check dependencies
        assert!(mod_json.depends.is_some());
        assert!(mod_json.recommends.is_some());
        assert!(mod_json.suggests.is_some());
        assert!(mod_json.breaks.is_some());
        assert!(mod_json.conflicts.is_some());

        // Check language adapters
        assert!(mod_json.language_adapters.is_some());

        // Check mixins
        assert!(mod_json.mixins.is_some());
        let mixins = mod_json.mixins.unwrap();
        assert_eq!(mixins.len(), 2);

        // Check jars
        assert!(mod_json.jars.is_some());

        // Check access widener
        assert_eq!(mod_json.access_widener, Some("modid.accesswidener".to_string()));

        // Check custom fields
        assert!(mod_json.custom.is_some());

        std::fs::remove_file(&jar_path).ok();
    }
}
