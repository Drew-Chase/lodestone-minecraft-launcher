use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read as IoRead;
use std::path::Path;
use thiserror::Error;
use zip::ZipArchive;

#[derive(Debug, Error)]
pub enum ForgeModTomlError {
    #[error("Failed to open JAR file at {path}: {source}")]
    FileOpen {
        path: String,
        source: std::io::Error,
    },
    #[error("Failed to read ZIP archive: {0}")]
    ZipRead(#[from] zip::result::ZipError),
    #[error("mods.toml not found in JAR file")]
    TomlNotFound,
    #[error("Failed to read mods.toml: {0}")]
    TomlRead(std::io::Error),
    #[error("Failed to parse mods.toml: {0}")]
    TomlParse(#[from] toml::de::Error),
}

/// Forge mods.toml configuration structure
/// Located at META-INF/mods.toml in mod JAR files
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForgeModsToml {
    /// Language loader identifier (e.g., "javafml", "lowcodefml")
    pub mod_loader: String,
    /// Acceptable loader version range (Maven format)
    pub loader_version: String,
    /// License identifier (SPDX format recommended)
    pub license: String,
    /// Display resources as separate pack
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_as_resource_pack: Option<bool>,
    /// Skip loading on dedicated servers
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_side_only: Option<bool>,
    /// Services the mod uses (deprecated)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub services: Option<Vec<String>>,
    /// Substitution properties for ${file.<key>} replacement
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub properties: Option<HashMap<String, String>>,
    /// URL for issue reporting
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "issueTrackerURL")]
    pub issue_tracker_url: Option<String>,
    /// Array of mod definitions
    pub mods: Vec<ModDefinition>,
    /// Dependencies grouped by mod ID
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<HashMap<String, Vec<Dependency>>>,
}

/// Individual mod definition within mods.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModDefinition {
    /// Unique mod identifier (pattern: ^[a-z][a-z0-9_]{1,63}$)
    pub mod_id: String,
    /// Override namespace (pattern: ^[a-z][a-z0-9_.-]{1,63}$)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    /// Mod version (supports ${file.jarVersion} substitution)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// User-facing mod name
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// Mod description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Image filename for mod list
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logo_file: Option<String>,
    /// Use linear (true) vs nearest filtering
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logo_blur: Option<bool>,
    /// Update checker JSON endpoint
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "updateJSONURL")]
    pub update_json_url: Option<String>,
    /// Feature requirements (e.g., java_version)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub features: Option<HashMap<String, String>>,
    /// Custom key-value metadata
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modproperties: Option<HashMap<String, String>>,
    /// Mod download page URL
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "modUrl")]
    pub mod_url: Option<String>,
    /// Attribution and acknowledgments
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credits: Option<String>,
    /// Mod author names
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authors: Option<String>,
    /// Display/project page URL
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "displayURL")]
    pub display_url: Option<String>,
    /// Version matching behavior
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_test: Option<String>,
}

/// Dependency specification for a mod
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dependency {
    /// Dependency mod identifier
    pub mod_id: String,
    /// Crash if unmet
    pub mandatory: bool,
    /// Acceptable version range (Maven format)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version_range: Option<String>,
    /// Load order: BEFORE, AFTER, or NONE
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ordering: Option<DependencyOrdering>,
    /// Required side: CLIENT, SERVER, or BOTH
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub side: Option<DependencySide>,
    /// Dependency download URL
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "referralUrl")]
    pub referral_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DependencyOrdering {
    Before,
    After,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DependencySide {
    Client,
    Server,
    Both,
}

impl ForgeModsToml {
    /// Parse mods.toml from a Forge mod JAR file
    ///
    /// # Arguments
    /// * `jar_path` - Path to the Forge mod JAR file
    ///
    /// # Returns
    /// * `Ok(ForgeModsToml)` - Successfully parsed mods.toml
    /// * `Err(ForgeModTomlError)` - Failed to read or parse the file
    ///
    /// # Example
    /// ```no_run
    /// use minecraft_modloaders::forge::ForgeModsToml;
    ///
    /// let mods_toml = ForgeModsToml::from_jar("path/to/mod.jar").unwrap();
    /// println!("Mod loader: {}", mods_toml.mod_loader);
    /// ```
    pub fn from_jar<P: AsRef<Path>>(jar_path: P) -> Result<Self, ForgeModTomlError> {
        let jar_path = jar_path.as_ref();
        let file = File::open(jar_path).map_err(|e| ForgeModTomlError::FileOpen {
            path: jar_path.display().to_string(),
            source: e,
        })?;

        let mut archive = ZipArchive::new(file)?;

        let mut toml_file = archive
            .by_name("META-INF/mods.toml")
            .map_err(|_| ForgeModTomlError::TomlNotFound)?;

        let mut contents = String::new();
        toml_file
            .read_to_string(&mut contents)
            .map_err(ForgeModTomlError::TomlRead)?;

        let mods_toml: ForgeModsToml = toml::from_str(&contents)?;
        Ok(mods_toml)
    }

    /// Get a specific mod definition by ID
    pub fn get_mod(&self, mod_id: &str) -> Option<&ModDefinition> {
        self.mods.iter().find(|m| m.mod_id == mod_id)
    }

    /// Get dependencies for a specific mod ID
    pub fn get_dependencies(&self, mod_id: &str) -> Option<&Vec<Dependency>> {
        self.dependencies.as_ref()?.get(mod_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    fn create_test_jar_with_toml(toml_content: &str) -> Vec<u8> {
        let mut buffer = Vec::new();
        {
            let mut zip = ZipWriter::new(std::io::Cursor::new(&mut buffer));

            zip.start_file("META-INF/mods.toml", SimpleFileOptions::default())
                .unwrap();
            zip.write_all(toml_content.as_bytes()).unwrap();

            zip.start_file("com/example/TestMod.class", SimpleFileOptions::default())
                .unwrap();
            zip.write_all(b"fake class content").unwrap();

            zip.finish().unwrap();
        }
        buffer
    }

    #[test]
    fn test_minimal_mods_toml() {
        let toml = r#"
modLoader = "javafml"
loaderVersion = "[40,)"
license = "MIT"

[[mods]]
modId = "testmod"
version = "1.0.0"
displayName = "Test Mod"
"#;

        let jar_data = create_test_jar_with_toml(toml);
        let temp_dir = std::env::temp_dir();
        let jar_path = temp_dir.join("test_minimal_forge.jar");
        std::fs::write(&jar_path, jar_data).unwrap();

        let result = ForgeModsToml::from_jar(&jar_path);
        assert!(result.is_ok());

        let mods_toml = result.unwrap();
        assert_eq!(mods_toml.mod_loader, "javafml");
        assert_eq!(mods_toml.loader_version, "[40,)");
        assert_eq!(mods_toml.license, "MIT");
        assert_eq!(mods_toml.mods.len(), 1);
        assert_eq!(mods_toml.mods[0].mod_id, "testmod");

        std::fs::remove_file(&jar_path).ok();
    }

    #[test]
    fn test_comprehensive_mods_toml() {
        let toml = r#"
modLoader = "javafml"
loaderVersion = "[40,)"
license = "All Rights Reserved"
showAsResourcePack = false
issueTrackerURL = "https://github.com/example/mod/issues"

[properties]
customProperty = "customValue"

[[mods]]
modId = "examplemod"
namespace = "examplemod"
version = "${file.jarVersion}"
displayName = "Example Mod"
description = '''
This is a comprehensive example mod
with multiple lines of description.
'''
logoFile = "logo.png"
logoBlur = true
updateJSONURL = "https://example.com/update.json"
modUrl = "https://example.com/mod"
credits = "Thanks to everyone"
authors = "Author1, Author2"
displayURL = "https://example.com"
displayTest = "MATCH_VERSION"

[mods.features]
java_version = "[17,)"

[mods.modproperties]
customKey = "customValue"

[[dependencies.examplemod]]
modId = "forge"
mandatory = true
versionRange = "[40,)"
ordering = "NONE"
side = "BOTH"

[[dependencies.examplemod]]
modId = "minecraft"
mandatory = true
versionRange = "[1.19,1.20)"
ordering = "NONE"
side = "BOTH"
"#;

        let jar_data = create_test_jar_with_toml(toml);
        let temp_dir = std::env::temp_dir();
        let jar_path = temp_dir.join("test_comprehensive_forge.jar");
        std::fs::write(&jar_path, jar_data).unwrap();

        let result = ForgeModsToml::from_jar(&jar_path);
        assert!(result.is_ok());

        let mods_toml = result.unwrap();

        // Check top-level fields
        assert_eq!(mods_toml.mod_loader, "javafml");
        assert_eq!(mods_toml.show_as_resource_pack, Some(false));
        assert!(mods_toml.properties.is_some());
        assert!(mods_toml.issue_tracker_url.is_some());

        // Check mod definition
        assert_eq!(mods_toml.mods.len(), 1);
        let mod_def = &mods_toml.mods[0];
        assert_eq!(mod_def.mod_id, "examplemod");
        assert_eq!(mod_def.display_name, Some("Example Mod".to_string()));
        assert!(mod_def.description.is_some());
        assert!(mod_def.features.is_some());
        assert!(mod_def.modproperties.is_some());

        // Check dependencies
        assert!(mods_toml.dependencies.is_some());
        let deps = mods_toml.get_dependencies("examplemod").unwrap();
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].mod_id, "forge");
        assert_eq!(deps[0].mandatory, true);

        std::fs::remove_file(&jar_path).ok();
    }

    #[test]
    fn test_multiple_mods() {
        let toml = r#"
modLoader = "javafml"
loaderVersion = "[40,)"
license = "MIT"

[[mods]]
modId = "mod1"
version = "1.0.0"
displayName = "Mod 1"

[[mods]]
modId = "mod2"
version = "2.0.0"
displayName = "Mod 2"
"#;

        let jar_data = create_test_jar_with_toml(toml);
        let temp_dir = std::env::temp_dir();
        let jar_path = temp_dir.join("test_multiple_mods_forge.jar");
        std::fs::write(&jar_path, jar_data).unwrap();

        let result = ForgeModsToml::from_jar(&jar_path);
        assert!(result.is_ok());

        let mods_toml = result.unwrap();
        assert_eq!(mods_toml.mods.len(), 2);
        assert_eq!(mods_toml.mods[0].mod_id, "mod1");
        assert_eq!(mods_toml.mods[1].mod_id, "mod2");

        // Test get_mod
        let mod1 = mods_toml.get_mod("mod1");
        assert!(mod1.is_some());
        assert_eq!(mod1.unwrap().display_name, Some("Mod 1".to_string()));

        std::fs::remove_file(&jar_path).ok();
    }

    #[test]
    fn test_dependency_ordering_and_side() {
        let toml = r#"
modLoader = "javafml"
loaderVersion = "[40,)"
license = "MIT"

[[mods]]
modId = "testmod"

[[dependencies.testmod]]
modId = "forge"
mandatory = true
ordering = "BEFORE"
side = "CLIENT"
"#;

        let jar_data = create_test_jar_with_toml(toml);
        let temp_dir = std::env::temp_dir();
        let jar_path = temp_dir.join("test_dep_ordering_forge.jar");
        std::fs::write(&jar_path, jar_data).unwrap();

        let result = ForgeModsToml::from_jar(&jar_path);
        assert!(result.is_ok());

        let mods_toml = result.unwrap();
        let deps = mods_toml.get_dependencies("testmod").unwrap();
        assert_eq!(deps[0].ordering, Some(DependencyOrdering::Before));
        assert_eq!(deps[0].side, Some(DependencySide::Client));

        std::fs::remove_file(&jar_path).ok();
    }

    #[test]
    fn test_toml_not_found() {
        let mut buffer = Vec::new();
        {
            let mut zip = ZipWriter::new(std::io::Cursor::new(&mut buffer));
            zip.start_file("META-INF/MANIFEST.MF", SimpleFileOptions::default())
                .unwrap();
            zip.write_all(b"Manifest-Version: 1.0\n").unwrap();
            zip.finish().unwrap();
        }

        let temp_dir = std::env::temp_dir();
        let jar_path = temp_dir.join("test_no_toml_forge.jar");
        std::fs::write(&jar_path, buffer).unwrap();

        let result = ForgeModsToml::from_jar(&jar_path);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ForgeModTomlError::TomlNotFound));

        std::fs::remove_file(&jar_path).ok();
    }

    #[test]
    fn test_invalid_toml() {
        let toml = r#"
modLoader = "javafml"
loaderVersion = [40,)  # Missing quotes
license = "MIT"
"#;

        let jar_data = create_test_jar_with_toml(toml);
        let temp_dir = std::env::temp_dir();
        let jar_path = temp_dir.join("test_invalid_toml_forge.jar");
        std::fs::write(&jar_path, jar_data).unwrap();

        let result = ForgeModsToml::from_jar(&jar_path);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ForgeModTomlError::TomlParse(_)));

        std::fs::remove_file(&jar_path).ok();
    }

    #[test]
    fn test_file_not_found() {
        let result = ForgeModsToml::from_jar("/nonexistent/path.jar");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ForgeModTomlError::FileOpen { .. }
        ));
    }
}
