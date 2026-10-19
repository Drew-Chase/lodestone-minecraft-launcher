//! `extract_mod_info` through the public API, against fixture jars built in-test
//! with the `zip` crate — no network, no binary fixtures (DESIGN.md §8).

mod fixture;

use fixture::jar;
use lodestone_loaders::{Fabric, Forge, Loader, LoaderError, NeoForge, Quilt, Vanilla};

#[test]
fn fabric_mod_json_maps_onto_mod_info() {
    let path = jar(
        "fabric",
        &[(
            "fabric.mod.json",
            r#"{
            "schemaVersion": 1,
            "id": "testmod",
            "version": "1.0.0",
            "name": "Test Mod",
            "description": "A test mod",
            "authors": [ "Alice", { "name": "Bob", "contact": {} } ],
            "license": "MIT",
            "contact": { "homepage": "https://example.com" },
            "depends": { "fabric-api": ">=0.50.0" },
            "recommends": { "sodium": "*" }
        }"#,
        )],
    );

    let info = Loader::<Fabric>::new().extract_mod_info(&path).expect("fabric metadata parses");
    assert_eq!(info.loader, "fabric");
    assert_eq!(info.id, "testmod");
    assert_eq!(info.version.as_deref(), Some("1.0.0"));
    assert_eq!(info.name.as_deref(), Some("Test Mod"));
    assert_eq!(info.description.as_deref(), Some("A test mod"));
    assert_eq!(info.authors, ["Alice", "Bob"]);
    assert_eq!(info.license.as_deref(), Some("MIT"));
    assert_eq!(info.homepage.as_deref(), Some("https://example.com"));

    let fabric_api = info
        .dependencies
        .iter()
        .find(|dependency| dependency.id == "fabric-api")
        .expect("declared");
    assert_eq!(fabric_api.kind, lodestone_loaders::ModDependencyKind::Required);
    assert_eq!(fabric_api.version_range.as_deref(), Some(">=0.50.0"));
    let sodium = info.dependencies.iter().find(|dependency| dependency.id == "sodium").expect("declared");
    assert_eq!(sodium.kind, lodestone_loaders::ModDependencyKind::Recommended);
}

#[test]
fn forge_mods_toml_maps_onto_mod_info() {
    let path = jar(
        "forge",
        &[(
            "META-INF/mods.toml",
            r#"
modLoader = "javafml"
loaderVersion = "[52,)"
license = "MIT"
displayURL = "https://example.com"

[[mods]]
modId = "testmod"
version = "1.0.0"
displayName = "Test Mod"
description = "A test mod"
authors = "Alice, Bob"

[[dependencies.testmod]]
modId = "forge"
mandatory = true
versionRange = "[52,)"
ordering = "NONE"
side = "BOTH"
"#,
        )],
    );

    let info = Loader::<Forge>::new().extract_mod_info(&path).expect("mods.toml parses");
    assert_eq!(info.loader, "forge");
    assert_eq!(info.id, "testmod");
    assert_eq!(info.version.as_deref(), Some("1.0.0"));
    assert_eq!(info.name.as_deref(), Some("Test Mod"));
    assert_eq!(info.description.as_deref(), Some("A test mod"));
    assert_eq!(info.authors, ["Alice", "Bob"]);
    assert_eq!(info.license.as_deref(), Some("MIT"));
    assert_eq!(info.homepage.as_deref(), Some("https://example.com"));
    let forge_dep = info.dependencies.iter().find(|dependency| dependency.id == "forge").expect("declared");
    assert_eq!(forge_dep.kind, lodestone_loaders::ModDependencyKind::Required);
}

#[test]
fn neoforge_mods_toml_reports_its_loader() {
    let path = jar(
        "neoforge",
        &[(
            "META-INF/neoforge.mods.toml",
            r#"
modLoader = "javafml"
loaderVersion = "[2,)"

[[mods]]
modId = "testmod"
version = "1.0.0"
displayName = "Test Mod"
description = "A test mod"
"#,
        )],
    );

    let info = Loader::<NeoForge>::new().extract_mod_info(&path).expect("neoforge.mods.toml parses");
    assert_eq!(info.loader, "neoforge");
    assert_eq!(info.id, "testmod");
    assert_eq!(info.name.as_deref(), Some("Test Mod"));
}

#[test]
fn legacy_mcmod_info_maps_onto_mod_info() {
    let path = jar(
        "legacy",
        &[(
            "mcmod.info",
            r#"[{
            "modid": "oldmod",
            "name": "Old Mod",
            "version": "0.9",
            "description": "An old mod",
            "authorList": ["Alice"]
        }]"#,
        )],
    );

    let info = Loader::<Forge>::new().extract_mod_info(&path).expect("mcmod.info parses");
    assert_eq!(info.loader, "forge");
    assert_eq!(info.id, "oldmod");
    assert_eq!(info.name.as_deref(), Some("Old Mod"));
    assert_eq!(info.authors, ["Alice"]);
}

#[test]
fn quilt_mod_json_maps_onto_mod_info() {
    let path = jar(
        "quilt",
        &[(
            "quilt.mod.json",
            r#"{
            "schema_version": 1,
            "quilt_loader": {
                "id": "testmod",
                "version": "1.0.0",
                "metadata": {
                    "name": "Test Mod",
                    "description": "A test mod",
                    "contributors": { "Alice": "Owner" },
                    "contact": { "homepage": "https://example.com" },
                    "license": "MIT"
                },
                "depends": [ { "id": "quilt_loader", "version": ">=0.17.0" } ]
            }
        }"#,
        )],
    );

    let info = Loader::<Quilt>::new().extract_mod_info(&path).expect("quilt.mod.json parses");
    assert_eq!(info.loader, "quilt");
    assert_eq!(info.id, "testmod");
    assert_eq!(info.authors, ["Alice"]);
    assert_eq!(info.dependencies.len(), 1);
    assert_eq!(info.dependencies[0].id, "quilt_loader");
}

#[test]
fn quilt_jars_fall_back_to_fabric_metadata() {
    // Quilt's loader runs fabric-compatible jars: no quilt.mod.json, but a
    // fabric.mod.json. The returned loader tag honestly reports "fabric".
    let path = jar(
        "quilt-fabric",
        &[("fabric.mod.json", r#"{ "schemaVersion": 1, "id": "fabmod", "version": "1.0.0" }"#)],
    );

    let info = Loader::<Quilt>::new().extract_mod_info(&path).expect("fabric fallback applies");
    assert_eq!(info.loader, "fabric");
    assert_eq!(info.id, "fabmod");
}

#[test]
fn extraction_is_format_driven_not_kind_driven() {
    // The marker is a convenience, not a filter: any non-vanilla kind reads
    // any jar the detectors recognize.
    let path = jar(
        "cross",
        &[("fabric.mod.json", r#"{ "schemaVersion": 1, "id": "any", "version": "1.0" }"#)],
    );
    for info in [
        Loader::<Fabric>::new().extract_mod_info(&path),
        Loader::<Forge>::new().extract_mod_info(&path),
        Loader::<NeoForge>::new().extract_mod_info(&path),
        Loader::<Quilt>::new().extract_mod_info(&path),
    ] {
        let info = info.expect("every kind reads the same jar");
        assert_eq!(info.loader, "fabric");
        assert_eq!(info.id, "any");
    }
}

#[test]
fn jars_without_metadata_report_it() {
    let path = jar("empty", &[]);
    for result in [
        Loader::<Fabric>::new().extract_mod_info(&path),
        Loader::<Forge>::new().extract_mod_info(&path),
        Loader::<NeoForge>::new().extract_mod_info(&path),
        Loader::<Quilt>::new().extract_mod_info(&path),
    ] {
        assert!(matches!(result, Err(LoaderError::ModMetadataMissing { .. })));
    }
}

#[test]
fn server_plugin_jars_are_out_of_scope() {
    let path = jar(
        "bukkit",
        &[("plugin.yml", "name: TestPlugin\nversion: 1.0\nmain: org.example.TestPlugin\n")],
    );
    let result = Loader::<Fabric>::new().extract_mod_info(&path);
    assert!(matches!(result, Err(LoaderError::ModMetadataMissing { .. })));
}

#[test]
fn vanilla_has_no_mod_metadata() {
    let path = jar("vanilla", &[]);
    let result = Loader::<Vanilla>::new().extract_mod_info(&path);
    assert!(matches!(result, Err(LoaderError::ModInfoUnsupported { loader: "vanilla" })));
}
