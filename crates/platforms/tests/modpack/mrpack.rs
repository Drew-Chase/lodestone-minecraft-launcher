use super::*;

const SAMPLE: &str = r#"{
    "formatVersion": 1,
    "game": "minecraft",
    "versionId": "1.2.3",
    "name": "Test Pack",
    "summary": "A test pack",
    "dependencies": {
        "minecraft": "1.21.1",
        "fabric-loader": "0.16.14"
    },
    "files": [
        {
            "path": "mods/sodium.jar",
            "hashes": { "sha1": "abc", "sha512": "def" },
            "env": { "client": "required", "server": "required" },
            "downloads": ["https://cdn.modrinth.com/data/x/versions/y/sodium.jar"],
            "fileSize": 1234
        },
        {
            "path": "mods/no-url.jar",
            "hashes": { "sha1": "zzz" },
            "downloads": []
        }
    ],
    "overrides": "overrides"
}"#;

#[test]
fn the_mrpack_manifest_maps_onto_modpack() {
    let modpack = parse(SAMPLE, Path::new("test.mrpack")).expect("sample parses");
    assert_eq!(modpack.name, "Test Pack");
    assert_eq!(modpack.version.as_deref(), Some("1.2.3"));
    assert_eq!(modpack.minecraft, "1.21.1");
    assert_eq!(modpack.loader, Some("fabric"));
    assert_eq!(modpack.loader_version.as_deref(), Some("0.16.14"));
    assert_eq!(modpack.overrides.as_deref(), Some("overrides"));
    // The file without a mirror is dropped; it cannot be installed.
    assert_eq!(modpack.files.len(), 1);
    assert_eq!(modpack.files[0].path, "mods/sodium.jar");
    assert_eq!(modpack.files[0].sha1.as_deref(), Some("abc"));
    assert_eq!(modpack.files[0].size, Some(1234));
}

#[test]
fn every_loader_dependency_key_maps() {
    for (key, expected) in [("forge", "forge"), ("quilt-loader", "quilt"), ("neoforge", "neoforge")] {
        let document = format!(r#"{{ "dependencies": {{ "minecraft": "1.0", "{key}": "9.9" }} }}"#);
        let index: MrpackIndex = serde_json::from_str(&document).expect("parses");
        let (loader, version) = index.dependencies.loader();
        assert_eq!(loader, Some(expected));
        assert_eq!(version.as_deref(), Some("9.9"));
    }
}

#[test]
fn a_vanilla_pack_has_no_loader() {
    let index: MrpackIndex = serde_json::from_str(r#"{ "dependencies": { "minecraft": "1.21.1" } }"#).expect("parses");
    assert_eq!(index.dependencies.loader(), (None, None));
}
