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
