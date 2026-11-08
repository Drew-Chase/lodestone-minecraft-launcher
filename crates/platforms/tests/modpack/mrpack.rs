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
    for (key, expected, version) in [
        ("forge", "forge", "1.0-9.9"),
        ("quilt-loader", "quilt", "9.9"),
        ("neoforge", "neoforge", "9.9"),
    ] {
        let document = format!(r#"{{ "dependencies": {{ "minecraft": "1.0", "{key}": "9.9" }} }}"#);
        let index: MrpackIndex = serde_json::from_str(&document).expect("parses");
        let (loader, loader_version) = index.dependencies.loader();
        assert_eq!(loader, Some(expected));
        assert_eq!(loader_version.as_deref(), Some(version));
    }
}

#[test]
fn a_forge_build_takes_its_minecraft_version_in_front_once() {
    let bare: MrpackIndex = serde_json::from_str(r#"{ "dependencies": { "minecraft": "1.12.2", "forge": "14.23.5.2860" } }"#).expect("parses");
    assert_eq!(bare.dependencies.loader(), (Some("forge"), Some("1.12.2-14.23.5.2860".to_string())));
    let combined: MrpackIndex =
        serde_json::from_str(r#"{ "dependencies": { "minecraft": "1.12.2", "forge": "1.12.2-14.23.5.2860" } }"#).expect("parses");
    assert_eq!(combined.dependencies.loader(), (Some("forge"), Some("1.12.2-14.23.5.2860".to_string())));
}

#[test]
fn a_vanilla_pack_has_no_loader() {
    let index: MrpackIndex = serde_json::from_str(r#"{ "dependencies": { "minecraft": "1.21.1" } }"#).expect("parses");
    assert_eq!(index.dependencies.loader(), (None, None));
}

/// An index naming one file at `path`, downloaded from `url`.
fn one_file(path: &str, url: &str) -> String {
    format!(r#"{{ "files": [ {{ "path": {path:?}, "hashes": {{ "sha1": "abc" }}, "downloads": [{url:?}] }} ] }}"#)
}

#[test]
fn a_file_path_that_leaves_the_instance_fails_the_pack() {
    for path in ["../../.bashrc", "/etc/passwd", "mods/../../x.jar", "C:\\Users\\x.jar"] {
        let raw = one_file(path, "https://cdn.modrinth.com/a.jar");
        let error = parse(&raw, Path::new("evil.mrpack")).expect_err(path);
        assert!(matches!(error, PlatformError::UnsafePath { .. }), "{path}: {error}");
    }
}

#[test]
fn only_https_mirrors_are_used() {
    let raw = r#"{ "files": [ { "path": "mods/a.jar", "downloads": ["http://plain.test/a.jar", "https://cdn.modrinth.com/a.jar"] } ] }"#;
    let modpack = parse(raw, Path::new("a.mrpack")).unwrap();
    assert_eq!(modpack.files[0].url, "https://cdn.modrinth.com/a.jar");

    let raw = one_file("mods/a.jar", "http://plain.test/a.jar");
    let error = parse(&raw, Path::new("a.mrpack")).unwrap_err();
    assert!(matches!(error, PlatformError::InsecureUrl { .. }), "{error}");
}
