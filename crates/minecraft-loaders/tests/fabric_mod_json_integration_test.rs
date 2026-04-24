use minecraft_modloaders::fabric::{Environment, FabricModJson, FabricModJsonError};
use std::io::Write;
use std::path::PathBuf;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

fn create_integration_test_jar(name: &str, json_content: &str) -> PathBuf {
    let mut buffer = Vec::new();
    {
        let mut zip = ZipWriter::new(std::io::Cursor::new(&mut buffer));

        zip.start_file("fabric.mod.json", SimpleFileOptions::default()).unwrap();
        zip.write_all(json_content.as_bytes()).unwrap();

        zip.start_file("META-INF/MANIFEST.MF", SimpleFileOptions::default()).unwrap();
        zip.write_all(b"Manifest-Version: 1.0\n").unwrap();

        zip.start_file("com/example/TestClass.class", SimpleFileOptions::default()).unwrap();
        zip.write_all(b"fake class content").unwrap();

        zip.finish().unwrap();
    }

    let temp_dir = std::env::temp_dir();
    let jar_path = temp_dir.join(format!("integration_test_{}.jar", name));
    std::fs::write(&jar_path, buffer).unwrap();
    jar_path
}

#[test]
fn test_integration_realistic_fabric_mod() {
    let json = r#"{
        "schemaVersion": 1,
        "id": "example-mod",
        "version": "1.2.3",
        "name": "Example Fabric Mod",
        "description": "This is a realistic example of a Fabric mod configuration",
        "authors": [
            "Developer1",
            "Developer2"
        ],
        "contact": {
            "homepage": "https://example.com/mod",
            "sources": "https://github.com/example/fabric-mod",
            "issues": "https://github.com/example/fabric-mod/issues"
        },
        "license": "MIT",
        "icon": "assets/example-mod/icon.png",
        "environment": "*",
        "entrypoints": {
            "main": [
                "com.example.ExampleMod"
            ],
            "client": [
                "com.example.ExampleModClient"
            ]
        },
        "mixins": [
            "example-mod.mixins.json"
        ],
        "depends": {
            "fabricloader": ">=0.15.0",
            "minecraft": "~1.20.1"
        },
        "recommends": {
            "modmenu": "*"
        }
    }"#;

    let jar_path = create_integration_test_jar("realistic", json);

    let result = FabricModJson::from_jar(&jar_path);
    assert!(result.is_ok(), "Failed to parse realistic fabric.mod.json: {:?}", result.err());

    let mod_json = result.unwrap();
    assert_eq!(mod_json.id, "example-mod");
    assert_eq!(mod_json.name, Some("Example Fabric Mod".to_string()));
    assert_eq!(mod_json.version, "1.2.3");
    assert_eq!(mod_json.environment, Some(Environment::All));
    assert!(mod_json.entrypoints.is_some());
    assert!(mod_json.mixins.is_some());
    assert!(mod_json.depends.is_some());

    std::fs::remove_file(&jar_path).ok();
}

#[test]
fn test_integration_client_only_mod() {
    let json = r#"{
        "schemaVersion": 1,
        "id": "client-mod",
        "version": "0.5.0",
        "name": "Client Only Mod",
        "description": "A mod that only runs on the client",
        "environment": "client",
        "entrypoints": {
            "client": [
                "com.example.ClientMod"
            ]
        },
        "depends": {
            "fabricloader": ">=0.14.0",
            "minecraft": ">=1.19"
        }
    }"#;

    let jar_path = create_integration_test_jar("client_only", json);

    let result = FabricModJson::from_jar(&jar_path);
    assert!(result.is_ok());

    let mod_json = result.unwrap();
    assert_eq!(mod_json.environment, Some(Environment::Client));
    assert!(mod_json.entrypoints.is_some());

    if let Some(entrypoints) = mod_json.entrypoints {
        assert!(entrypoints.contains_key("client"));
        assert!(!entrypoints.contains_key("server"));
    }

    std::fs::remove_file(&jar_path).ok();
}

#[test]
fn test_integration_server_only_mod() {
    let json = r#"{
        "schemaVersion": 1,
        "id": "server-mod",
        "version": "2.0.0",
        "name": "Server Only Mod",
        "environment": "server",
        "entrypoints": {
            "server": [
                "com.example.ServerMod"
            ]
        },
        "depends": {
            "fabricloader": ">=0.15.0",
            "minecraft": "1.20.x"
        }
    }"#;

    let jar_path = create_integration_test_jar("server_only", json);

    let result = FabricModJson::from_jar(&jar_path);
    assert!(result.is_ok());

    let mod_json = result.unwrap();
    assert_eq!(mod_json.environment, Some(Environment::Server));

    std::fs::remove_file(&jar_path).ok();
}

#[test]
fn test_integration_mod_with_jars() {
    let json = r#"{
        "schemaVersion": 1,
        "id": "mod-with-jars",
        "version": "1.0.0",
        "name": "Mod With Nested JARs",
        "jars": [
            {"file": "META-INF/jars/library1.jar"},
            {"file": "META-INF/jars/library2.jar"}
        ],
        "depends": {
            "fabricloader": ">=0.14.0",
            "minecraft": "*"
        }
    }"#;

    let jar_path = create_integration_test_jar("with_jars", json);

    let result = FabricModJson::from_jar(&jar_path);
    assert!(result.is_ok());

    let mod_json = result.unwrap();
    assert!(mod_json.jars.is_some());

    if let Some(jars) = mod_json.jars {
        assert_eq!(jars.len(), 2);
        assert_eq!(jars[0].file, "META-INF/jars/library1.jar");
        assert_eq!(jars[1].file, "META-INF/jars/library2.jar");
    }

    std::fs::remove_file(&jar_path).ok();
}

#[test]
fn test_integration_error_missing_json() {
    let mut buffer = Vec::new();
    {
        let mut zip = ZipWriter::new(std::io::Cursor::new(&mut buffer));
        zip.start_file("META-INF/MANIFEST.MF", SimpleFileOptions::default()).unwrap();
        zip.write_all(b"Manifest-Version: 1.0\n").unwrap();
        zip.finish().unwrap();
    }

    let temp_dir = std::env::temp_dir();
    let jar_path = temp_dir.join("integration_test_no_fabric_json.jar");
    std::fs::write(&jar_path, buffer).unwrap();

    let result = FabricModJson::from_jar(&jar_path);
    assert!(result.is_err());

    match result.unwrap_err() {
        FabricModJsonError::JsonNotFound => {},
        other => panic!("Expected JsonNotFound error, got: {:?}", other),
    }

    std::fs::remove_file(&jar_path).ok();
}

#[test]
fn test_integration_error_malformed_json() {
    let malformed_json = r#"{
        "schemaVersion": 1,
        "id": "broken-mod",
        "version": "1.0.0",
        "name": "Broken Mod"
        "missing_comma": true
    }"#;

    let jar_path = create_integration_test_jar("malformed", malformed_json);

    let result = FabricModJson::from_jar(&jar_path);
    assert!(result.is_err());

    match result.unwrap_err() {
        FabricModJsonError::JsonParse(_) => {},
        other => panic!("Expected JsonParse error, got: {:?}", other),
    }

    std::fs::remove_file(&jar_path).ok();
}

#[test]
fn test_integration_mod_with_access_widener() {
    let json = r#"{
        "schemaVersion": 1,
        "id": "access-widener-mod",
        "version": "1.0.0",
        "name": "Access Widener Mod",
        "accessWidener": "example.accesswidener",
        "depends": {
            "fabricloader": ">=0.14.0",
            "minecraft": "~1.20"
        }
    }"#;

    let jar_path = create_integration_test_jar("access_widener", json);

    let result = FabricModJson::from_jar(&jar_path);
    assert!(result.is_ok());

    let mod_json = result.unwrap();
    assert_eq!(mod_json.access_widener, Some("example.accesswidener".to_string()));

    std::fs::remove_file(&jar_path).ok();
}

#[test]
fn test_integration_mod_with_comprehensive_features() {
    let json = r#"{
        "schemaVersion": 1,
        "id": "comprehensive-mod",
        "version": "3.0.0",
        "name": "Comprehensive Mod",
        "description": "A mod showcasing all fabric.mod.json features",
        "license": ["MIT", "Apache-2.0"],
        "authors": [
            "Simple Author",
            {
                "name": "Detailed Author",
                "contact": {
                    "email": "author@example.com"
                }
            }
        ],
        "contributors": ["Contributor 1", "Contributor 2"],
        "contact": {
            "homepage": "https://example.com",
            "sources": "https://github.com/example/mod",
            "issues": "https://github.com/example/mod/issues",
            "email": "contact@example.com",
            "irc": "irc://irc.example.com/#channel"
        },
        "icon": "icon.png",
        "environment": "client",
        "provides": ["old-mod-id", "legacy-mod-id"],
        "entrypoints": {
            "main": ["com.example.MainClass"],
            "client": ["com.example.ClientClass"],
            "server": [
                {
                    "adapter": "kotlin",
                    "value": "com.example.KotlinClass"
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
            "mod.mixins.json",
            {
                "config": "mod.client.mixins.json",
                "environment": "client"
            }
        ],
        "jars": [
            {"file": "nested-lib.jar"}
        ],
        "accessWidener": "mod.accesswidener",
        "custom": {
            "modid:custom_field": "custom_value",
            "modid:another_field": 123
        }
    }"#;

    let jar_path = create_integration_test_jar("comprehensive", json);
    let result = FabricModJson::from_jar(&jar_path);
    assert!(result.is_ok());

    let mod_json = result.unwrap();

    // Basic fields
    assert_eq!(mod_json.id, "comprehensive-mod");
    assert_eq!(mod_json.version, "3.0.0");
    assert_eq!(mod_json.name, Some("Comprehensive Mod".to_string()));
    assert_eq!(mod_json.environment, Some(Environment::Client));

    // License
    assert!(mod_json.license.is_some());

    // Authors and contributors
    assert!(mod_json.authors.is_some());
    let authors = mod_json.authors.unwrap();
    assert_eq!(authors.len(), 2);
    assert!(mod_json.contributors.is_some());

    // Contact
    assert!(mod_json.contact.is_some());
    let contact = mod_json.contact.unwrap();
    assert!(contact.email.is_some());
    assert!(contact.irc.is_some());
    assert!(contact.homepage.is_some());

    // Provides
    assert!(mod_json.provides.is_some());
    let provides = mod_json.provides.unwrap();
    assert_eq!(provides.len(), 2);

    // Dependencies
    assert!(mod_json.depends.is_some());
    assert!(mod_json.recommends.is_some());
    assert!(mod_json.suggests.is_some());
    assert!(mod_json.breaks.is_some());
    assert!(mod_json.conflicts.is_some());

    // Entrypoints
    assert!(mod_json.entrypoints.is_some());
    let entrypoints = mod_json.entrypoints.unwrap();
    assert!(entrypoints.contains_key("main"));
    assert!(entrypoints.contains_key("client"));
    assert!(entrypoints.contains_key("server"));

    // Language adapters
    assert!(mod_json.language_adapters.is_some());

    // Mixins
    assert!(mod_json.mixins.is_some());
    let mixins = mod_json.mixins.unwrap();
    assert_eq!(mixins.len(), 2);

    // Jars
    assert!(mod_json.jars.is_some());

    // Access widener
    assert_eq!(mod_json.access_widener, Some("mod.accesswidener".to_string()));

    // Custom fields
    assert!(mod_json.custom.is_some());
    let custom = mod_json.custom.unwrap();
    assert!(custom.contains_key("modid:custom_field"));

    std::fs::remove_file(&jar_path).ok();
}

#[test]
fn test_integration_mod_with_multiple_licenses() {
    let json = r#"{
        "schemaVersion": 1,
        "id": "multi-license-mod",
        "version": "1.0.0",
        "license": ["MIT", "Apache-2.0", "GPL-3.0"],
        "depends": {
            "fabricloader": ">=0.14.0",
            "minecraft": "*"
        }
    }"#;

    let jar_path = create_integration_test_jar("multi_license", json);
    let result = FabricModJson::from_jar(&jar_path);
    assert!(result.is_ok());

    let mod_json = result.unwrap();
    assert!(mod_json.license.is_some());

    std::fs::remove_file(&jar_path).ok();
}

#[test]
fn test_integration_mod_with_single_license() {
    let json = r#"{
        "schemaVersion": 1,
        "id": "single-license-mod",
        "version": "1.0.0",
        "license": "MIT",
        "depends": {
            "fabricloader": ">=0.14.0",
            "minecraft": "*"
        }
    }"#;

    let jar_path = create_integration_test_jar("single_license", json);
    let result = FabricModJson::from_jar(&jar_path);
    assert!(result.is_ok());

    let mod_json = result.unwrap();
    assert!(mod_json.license.is_some());

    std::fs::remove_file(&jar_path).ok();
}

#[test]
fn test_integration_mod_with_version_array() {
    let json = r#"{
        "schemaVersion": 1,
        "id": "version-array-mod",
        "version": "1.0.0",
        "depends": {
            "minecraft": ["~1.19", "~1.20", "~1.20.1"]
        }
    }"#;

    let jar_path = create_integration_test_jar("version_array", json);
    let result = FabricModJson::from_jar(&jar_path);
    assert!(result.is_ok());

    let mod_json = result.unwrap();
    assert!(mod_json.depends.is_some());
    let depends = mod_json.depends.unwrap();
    assert!(depends.contains_key("minecraft"));

    std::fs::remove_file(&jar_path).ok();
}

#[test]
fn test_integration_minimal_required_fields() {
    let json = r#"{
        "schemaVersion": 1,
        "id": "minimal-mod",
        "version": "1.0.0"
    }"#;

    let jar_path = create_integration_test_jar("minimal", json);
    let result = FabricModJson::from_jar(&jar_path);
    assert!(result.is_ok());

    let mod_json = result.unwrap();
    assert_eq!(mod_json.id, "minimal-mod");
    assert_eq!(mod_json.version, "1.0.0");
    assert_eq!(mod_json.schema_version, 1);
    assert!(mod_json.name.is_none());

    std::fs::remove_file(&jar_path).ok();
}
