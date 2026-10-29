use super::*;

const SAMPLE: &str = r#"{
    "manifestType": "minecraftModpack",
    "manifestVersion": 1,
    "name": "Test Pack",
    "version": "1.2.3",
    "author": "Alice",
    "minecraft": {
        "version": "1.20.1",
        "modLoaders": [
            { "id": "forge-47.4.0", "primary": true }
        ]
    },
    "files": [
        { "projectID": 306612, "fileID": 4838285, "required": true },
        { "projectID": 238222, "fileID": 4655861, "required": false }
    ],
    "overrides": "overrides"
}"#;

#[test]
fn the_manifest_maps_onto_modpack() {
    let modpack = parse(SAMPLE, Path::new("test.zip")).expect("sample parses");
    assert_eq!(modpack.name, "Test Pack");
    assert_eq!(modpack.version.as_deref(), Some("1.2.3"));
    assert_eq!(modpack.minecraft, "1.20.1");
    assert_eq!(modpack.loader, Some("forge"));
    assert_eq!(modpack.loader_version.as_deref(), Some("47.4.0"));
    assert_eq!(modpack.overrides.as_deref(), Some("overrides"));
    assert_eq!(modpack.files.len(), 2);
    assert_eq!(modpack.files[0].project_id.as_deref(), Some("306612"));
    assert_eq!(modpack.files[0].file_id.as_deref(), Some("4838285"));
    assert!(modpack.files[0].url.is_empty(), "urls need the CurseForge API at install time");
}

#[test]
fn loader_ids_split_on_the_first_dash() {
    assert_eq!(split_loader("forge-47.4.0"), (Some("forge"), Some("47.4.0".to_string())));
    assert_eq!(split_loader("neoforge-21.1.95"), (Some("neoforge"), Some("21.1.95".to_string())));
    assert_eq!(split_loader("mystery-1.0"), (None, None));
    assert_eq!(split_loader("bare"), (None, None));
}

#[test]
fn a_non_primary_loader_is_used_when_no_primary_exists() {
    let document = r#"{
        "minecraft": { "version": "1.20.1", "modLoaders": [ { "id": "fabric-0.16.14" } ] }
    }"#;
    let modpack = parse(document, Path::new("test.zip")).expect("parses");
    assert_eq!(modpack.loader, Some("fabric"));
    assert_eq!(modpack.loader_version.as_deref(), Some("0.16.14"));
}
