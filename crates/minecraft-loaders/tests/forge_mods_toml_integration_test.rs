use minecraft_modloaders::forge::{
    DependencyOrdering, DependencySide, ForgeModTomlError, ForgeModsToml,
};
use std::io::Write;
use std::path::PathBuf;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

fn create_integration_test_jar(name: &str, toml_content: &str) -> PathBuf {
    let mut buffer = Vec::new();
    {
        let mut zip = ZipWriter::new(std::io::Cursor::new(&mut buffer));

        zip.start_file("META-INF/mods.toml", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(toml_content.as_bytes()).unwrap();

        zip.start_file("META-INF/MANIFEST.MF", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"Manifest-Version: 1.0\n").unwrap();

        zip.start_file("com/example/TestMod.class", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"fake class content").unwrap();

        zip.finish().unwrap();
    }

    let temp_dir = std::env::temp_dir();
    let jar_path = temp_dir.join(format!("integration_test_forge_{}.jar", name));
    std::fs::write(&jar_path, buffer).unwrap();
    jar_path
}

#[test]
fn test_integration_realistic_forge_mod() {
    let toml = r#"
modLoader = "javafml"
loaderVersion = "[40,)"
license = "MIT"
issueTrackerURL = "https://github.com/example/examplemod/issues"

[[mods]]
modId = "examplemod"
version = "1.0.0"
displayName = "Example Mod"
description = "A realistic Forge mod"
authors = "Author1, Author2"
credits = "Thanks to everyone"

[[dependencies.examplemod]]
modId = "forge"
mandatory = true
versionRange = "[40,)"
ordering = "NONE"
side = "BOTH"

[[dependencies.examplemod]]
modId = "minecraft"
mandatory = true
versionRange = "[1.19.2,1.20)"
ordering = "NONE"
side = "BOTH"
    "#;

    let jar_path = create_integration_test_jar("realistic", toml);

    let result = ForgeModsToml::from_jar(&jar_path);
    assert!(
        result.is_ok(),
        "Failed to parse realistic mods.toml: {:?}",
        result.err()
    );

    let mods_toml = result.unwrap();
    assert_eq!(mods_toml.mod_loader, "javafml");
    assert_eq!(mods_toml.loader_version, "[40,)");
    assert_eq!(mods_toml.license, "MIT");
    assert_eq!(mods_toml.mods.len(), 1);
    assert_eq!(mods_toml.mods[0].mod_id, "examplemod");

    let deps = mods_toml.get_dependencies("examplemod");
    assert!(deps.is_some());
    assert_eq!(deps.unwrap().len(), 2);

    std::fs::remove_file(&jar_path).ok();
}

#[test]
fn test_integration_client_side_only() {
    let toml = r#"
modLoader = "javafml"
loaderVersion = "[40,)"
license = "MIT"
clientSideOnly = true

[[mods]]
modId = "clientmod"
version = "1.0.0"
displayName = "Client Mod"
    "#;

    let jar_path = create_integration_test_jar("client_only", toml);

    let result = ForgeModsToml::from_jar(&jar_path);
    assert!(result.is_ok());

    let mods_toml = result.unwrap();
    assert_eq!(mods_toml.client_side_only, Some(true));
    assert_eq!(mods_toml.mods[0].mod_id, "clientmod");

    std::fs::remove_file(&jar_path).ok();
}

#[test]
fn test_integration_multiple_mods() {
    let toml = r#"
modLoader = "javafml"
loaderVersion = "[40,)"
license = "MIT"

[[mods]]
modId = "mod1"
version = "1.0.0"
displayName = "Mod One"

[[mods]]
modId = "mod2"
version = "2.0.0"
displayName = "Mod Two"

[[dependencies.mod1]]
modId = "minecraft"
mandatory = true

[[dependencies.mod2]]
modId = "minecraft"
mandatory = true
    "#;

    let jar_path = create_integration_test_jar("multiple_mods", toml);

    let result = ForgeModsToml::from_jar(&jar_path);
    assert!(result.is_ok());

    let mods_toml = result.unwrap();
    assert_eq!(mods_toml.mods.len(), 2);
    assert_eq!(mods_toml.mods[0].mod_id, "mod1");
    assert_eq!(mods_toml.mods[1].mod_id, "mod2");

    // Check get_mod method
    let mod1 = mods_toml.get_mod("mod1");
    assert!(mod1.is_some());
    assert_eq!(
        mod1.unwrap().display_name,
        Some("Mod One".to_string())
    );

    std::fs::remove_file(&jar_path).ok();
}

#[test]
fn test_integration_dependency_ordering() {
    let toml = r#"
modLoader = "javafml"
loaderVersion = "[40,)"
license = "MIT"

[[mods]]
modId = "testmod"

[[dependencies.testmod]]
modId = "dependency1"
mandatory = true
ordering = "BEFORE"
side = "CLIENT"

[[dependencies.testmod]]
modId = "dependency2"
mandatory = false
ordering = "AFTER"
side = "SERVER"
    "#;

    let jar_path = create_integration_test_jar("ordering", toml);

    let result = ForgeModsToml::from_jar(&jar_path);
    assert!(result.is_ok());

    let mods_toml = result.unwrap();
    let deps = mods_toml.get_dependencies("testmod").unwrap();

    assert_eq!(deps.len(), 2);
    assert_eq!(deps[0].ordering, Some(DependencyOrdering::Before));
    assert_eq!(deps[0].side, Some(DependencySide::Client));
    assert_eq!(deps[0].mandatory, true);

    assert_eq!(deps[1].ordering, Some(DependencyOrdering::After));
    assert_eq!(deps[1].side, Some(DependencySide::Server));
    assert_eq!(deps[1].mandatory, false);

    std::fs::remove_file(&jar_path).ok();
}

#[test]
fn test_integration_with_properties() {
    let toml = r#"
modLoader = "javafml"
loaderVersion = "[40,)"
license = "MIT"

[properties]
customKey = "customValue"
anotherKey = "anotherValue"

[[mods]]
modId = "testmod"
version = "${file.jarVersion}"
    "#;

    let jar_path = create_integration_test_jar("properties", toml);

    let result = ForgeModsToml::from_jar(&jar_path);
    assert!(result.is_ok());

    let mods_toml = result.unwrap();
    assert!(mods_toml.properties.is_some());

    let props = mods_toml.properties.unwrap();
    assert_eq!(props.get("customKey"), Some(&"customValue".to_string()));
    assert_eq!(props.get("anotherKey"), Some(&"anotherValue".to_string()));

    std::fs::remove_file(&jar_path).ok();
}

#[test]
fn test_integration_with_features() {
    let toml = r#"
modLoader = "javafml"
loaderVersion = "[40,)"
license = "MIT"

[[mods]]
modId = "testmod"

[mods.features]
java_version = "[17,)"
    "#;

    let jar_path = create_integration_test_jar("features", toml);

    let result = ForgeModsToml::from_jar(&jar_path);
    assert!(result.is_ok());

    let mods_toml = result.unwrap();
    assert!(mods_toml.mods[0].features.is_some());

    let features = mods_toml.mods[0].features.as_ref().unwrap();
    assert_eq!(features.get("java_version"), Some(&"[17,)".to_string()));

    std::fs::remove_file(&jar_path).ok();
}

#[test]
fn test_integration_error_missing_toml() {
    let mut buffer = Vec::new();
    {
        let mut zip = ZipWriter::new(std::io::Cursor::new(&mut buffer));
        zip.start_file("META-INF/MANIFEST.MF", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"Manifest-Version: 1.0\n").unwrap();
        zip.finish().unwrap();
    }

    let temp_dir = std::env::temp_dir();
    let jar_path = temp_dir.join("integration_test_forge_no_toml.jar");
    std::fs::write(&jar_path, buffer).unwrap();

    let result = ForgeModsToml::from_jar(&jar_path);
    assert!(result.is_err());

    match result.unwrap_err() {
        ForgeModTomlError::TomlNotFound => {}
        other => panic!("Expected TomlNotFound error, got: {:?}", other),
    }

    std::fs::remove_file(&jar_path).ok();
}

#[test]
fn test_integration_error_malformed_toml() {
    let malformed_toml = r#"
modLoader = "javafml"
loaderVersion = [40,)  # Missing quotes
license = "MIT"
    "#;

    let jar_path = create_integration_test_jar("malformed", malformed_toml);

    let result = ForgeModsToml::from_jar(&jar_path);
    assert!(result.is_err());

    match result.unwrap_err() {
        ForgeModTomlError::TomlParse(_) => {}
        other => panic!("Expected TomlParse error, got: {:?}", other),
    }

    std::fs::remove_file(&jar_path).ok();
}

#[test]
fn test_integration_comprehensive_features() {
    let toml = r#"
modLoader = "javafml"
loaderVersion = "[40,)"
license = "All Rights Reserved"
showAsResourcePack = true
clientSideOnly = false
issueTrackerURL = "https://github.com/example/mod/issues"

[properties]
buildVersion = "1.0.0-beta"

[[mods]]
modId = "comprehensivemod"
namespace = "comprehensivemod"
version = "${file.jarVersion}"
displayName = "Comprehensive Mod"
description = "A mod with all features"
logoFile = "logo.png"
logoBlur = false
updateJSONURL = "https://example.com/update.json"
modUrl = "https://example.com/mod"
credits = "Thanks to all"
authors = "Author1, Author2"
displayURL = "https://example.com"
displayTest = "MATCH_VERSION"

[mods.features]
java_version = "[17,)"

[mods.modproperties]
customProperty = "value"

[[dependencies.comprehensivemod]]
modId = "forge"
mandatory = true
versionRange = "[40,)"
ordering = "NONE"
side = "BOTH"
referralUrl = "https://files.minecraftforge.net"
    "#;

    let jar_path = create_integration_test_jar("comprehensive", toml);

    let result = ForgeModsToml::from_jar(&jar_path);
    assert!(result.is_ok());

    let mods_toml = result.unwrap();

    // Check top-level fields
    assert_eq!(mods_toml.mod_loader, "javafml");
    assert_eq!(mods_toml.show_as_resource_pack, Some(true));
    assert_eq!(mods_toml.client_side_only, Some(false));
    assert!(mods_toml.issue_tracker_url.is_some());
    assert!(mods_toml.properties.is_some());

    // Check mod definition
    let mod_def = &mods_toml.mods[0];
    assert_eq!(mod_def.mod_id, "comprehensivemod");
    assert_eq!(mod_def.namespace, Some("comprehensivemod".to_string()));
    assert!(mod_def.display_name.is_some());
    assert!(mod_def.description.is_some());
    assert!(mod_def.logo_file.is_some());
    assert_eq!(mod_def.logo_blur, Some(false));
    assert!(mod_def.update_json_url.is_some());
    assert!(mod_def.mod_url.is_some());
    assert!(mod_def.credits.is_some());
    assert!(mod_def.authors.is_some());
    assert!(mod_def.display_url.is_some());
    assert!(mod_def.features.is_some());
    assert!(mod_def.modproperties.is_some());

    // Check dependencies
    let deps = mods_toml.get_dependencies("comprehensivemod").unwrap();
    assert_eq!(deps.len(), 1);
    assert_eq!(deps[0].mod_id, "forge");
    assert!(deps[0].referral_url.is_some());

    std::fs::remove_file(&jar_path).ok();
}

#[test]
fn test_integration_lowcode_loader() {
    let toml = r#"
modLoader = "lowcodefml"
loaderVersion = "[40,)"
license = "MIT"
showAsResourcePack = true

[[mods]]
modId = "resourcepack"
version = "1.0.0"
displayName = "Resource Pack Mod"
    "#;

    let jar_path = create_integration_test_jar("lowcode", toml);

    let result = ForgeModsToml::from_jar(&jar_path);
    assert!(result.is_ok());

    let mods_toml = result.unwrap();
    assert_eq!(mods_toml.mod_loader, "lowcodefml");
    assert_eq!(mods_toml.show_as_resource_pack, Some(true));

    std::fs::remove_file(&jar_path).ok();
}
