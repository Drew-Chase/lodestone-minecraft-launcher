use super::*;

/// Recorded slice of the real `/v1/pack/SkyFactory` response.
const PACK: &str = r#"{
    "error": false,
    "code": 200,
    "message": null,
    "data": {
        "id": 190,
        "name": "Sky Factory",
        "safeName": "SkyFactory",
        "type": "public",
        "description": "A skyblock pack",
        "versions": [
            { "version": "2.4.1", "minecraft": "1.7.10", "published": 1639190283 },
            { "version": "2.4", "minecraft": "1.7.10", "published": 1639190283 }
        ]
    }
}"#;

/// A slice of a recorded modern pack's `Configs.json`, with one mod of each kind the install treats
/// differently: a server mod, an optional mod nobody selected, a server-only mod, and an archive.
const CONFIGS: &str = r#"{
    "version": "v5.11.0",
    "minecraft": "1.20.1",
    "loader": {
        "type": "fabric",
        "choose": false,
        "metadata": { "minecraft": "1.20.1", "loader": "0.16.7", "yarn": "1.20.1+build.10" },
        "className": "com.atlauncher.data.loaders.fabric.FabricLoader"
    },
    "mods": [
        { "name": "Ad Astra", "version": "1.15.18", "url": "packs/AllTheFabric5/files/ad_astra.jar", "file": "ad_astra.jar",
          "download": "server", "md5": "6351032dcd329824553a1ca29b44094f", "filesize": 7270527, "type": "mods",
          "client": true, "optional": false, "selected": false },
        { "name": "Extras", "version": "1", "url": "packs/AllTheFabric5/files/extras.jar", "file": "extras.jar",
          "download": "server", "filesize": 10, "type": "mods", "client": true, "optional": true, "selected": false },
        { "name": "Server Only", "version": "1", "url": "packs/AllTheFabric5/files/server.jar", "file": "server.jar",
          "download": "server", "filesize": 10, "type": "mods", "client": false, "optional": false, "selected": false },
        { "name": "Resources", "version": "1", "url": "packs/AllTheFabric5/files/resources.zip", "file": "resources.zip",
          "download": "server", "filesize": 20, "type": "extract", "extractTo": "mods", "extractFolder": "/mods/",
          "client": true, "optional": false, "selected": false }
    ],
    "configs": { "filesize": 5028823, "sha1": "768aedf6972e5f7d2e6fc50f181b5b142240637d" },
    "noConfigs": false,
    "java": { "min": 17, "max": 17 },
    "memory": 2048
}"#;

fn version() -> ContentVersion {
    ContentVersion {
        id: "v5.11.0".to_string(),
        project_id: Some("AllTheFabric5".to_string()),
        name: Some("v5.11.0".to_string()),
        version_number: "v5.11.0".to_string(),
        content_type: ContentType::Modpack,
        minecraft_versions: vec!["1.20.1".to_string()],
        loaders: Vec::new(),
        downloads: 0,
        date: None,
        changelog_url: None,
        files: Vec::new(),
    }
}

fn configs(json: &str) -> PackConfigs {
    serde_json::from_str(json).expect("recorded configs parse")
}

#[test]
fn the_pack_envelope_parses() {
    let envelope: Envelope<Pack> = serde_json::from_str(PACK).expect("recorded payload parses");
    let pack = envelope.data.expect("data present");
    assert_eq!(pack.name, "Sky Factory");
    assert_eq!(pack.safe_name.as_deref(), Some("SkyFactory"));
    assert_eq!(pack.versions.len(), 2);
    assert_eq!(pack.versions[0].version, "2.4.1");
    assert_eq!(pack.versions[0].minecraft_version, "1.7.10");
}

#[test]
fn versions_map_with_their_minecraft_version() {
    let envelope: Envelope<Pack> = serde_json::from_str(PACK).expect("recorded payload parses");
    let pack = envelope.data.expect("data present");
    let versions: Vec<ContentVersion> = pack
        .versions
        .into_iter()
        .map(|version| ContentVersion {
            id: version.version.clone(),
            project_id: None,
            name: Some(version.version.clone()),
            version_number: version.version,
            content_type: ContentType::Modpack,
            minecraft_versions: vec![version.minecraft_version],
            loaders: Vec::new(),
            downloads: 0,
            date: version.published.map(|published| published.to_string()),
            changelog_url: None,
            files: Vec::new(),
        })
        .collect();
    assert_eq!(versions.len(), 2);
    assert_eq!(versions[0].minecraft_versions, ["1.7.10"]);
}

#[test]
fn a_modern_pack_maps_onto_a_fabric_modpack() {
    let modpack = modpack_from("All the Fabric 5".to_string(), "AllTheFabric5", &version(), configs(CONFIGS)).expect("pack maps");
    assert_eq!(modpack.minecraft, "1.20.1");
    assert_eq!(modpack.loader, Some("fabric"));
    assert_eq!(modpack.loader_version.as_deref(), Some("0.16.7"));
    assert_eq!(modpack.files.len(), 1, "the optional and server-only mods are left out");
    assert_eq!(modpack.files[0].path, "mods/ad_astra.jar");
    assert_eq!(
        modpack.files[0].url,
        "https://download.nodecdn.net/containers/atl/packs/AllTheFabric5/files/ad_astra.jar"
    );
    assert_eq!(modpack.files[0].size, Some(7270527));
}

#[test]
fn archives_unpack_into_their_folders_and_the_configs_are_fetched() {
    let modpack = modpack_from("All the Fabric 5".to_string(), "AllTheFabric5", &version(), configs(CONFIGS)).expect("pack maps");
    assert_eq!(modpack.archives.len(), 2);
    let resources = &modpack.archives[0];
    assert_eq!(resources.name, "resources.zip");
    assert_eq!(resources.target, "mods");
    assert_eq!(resources.prefix, "mods");
    let configs_zip = &modpack.archives[1];
    assert_eq!(configs_zip.name, "Configs.zip");
    assert_eq!(
        configs_zip.url,
        "https://download.nodecdn.net/containers/atl/packs/AllTheFabric5/versions/v5.11.0/Configs.zip"
    );
    assert_eq!(configs_zip.sha1.as_deref(), Some("768aedf6972e5f7d2e6fc50f181b5b142240637d"));
    assert_eq!(configs_zip.target, "");
}

#[test]
fn forge_takes_the_minecraft_version_in_front_of_its_own() {
    let json = CONFIGS
        .replace("\"type\": \"fabric\"", "\"type\": \"forge\"")
        .replace("\"loader\": \"0.16.7\"", "\"loader\": \"47.2.0\"");
    let modpack = modpack_from("Forge pack".to_string(), "ForgePack", &version(), configs(&json)).expect("pack maps");
    assert_eq!(modpack.loader, Some("forge"));
    assert_eq!(modpack.loader_version.as_deref(), Some("1.20.1-47.2.0"));
}

#[test]
fn a_mod_only_a_browser_can_fetch_stops_the_install() {
    let json = CONFIGS.replace("\"download\": \"server\", \"md5\"", "\"download\": \"browser\", \"md5\"");
    let error = modpack_from("Browser pack".to_string(), "BrowserPack", &version(), configs(&json)).expect_err("the install stops");
    assert!(matches!(error, PlatformError::Download { .. }));
}

#[test]
fn a_loader_the_launcher_cannot_install_stops_the_install() {
    let json = CONFIGS.replace("\"type\": \"fabric\"", "\"type\": \"bukkit\"");
    let error = modpack_from("Bukkit pack".to_string(), "BukkitPack", &version(), configs(&json)).expect_err("unknown loader");
    assert!(matches!(error, PlatformError::UnsupportedPlatform { .. }));
}

#[test]
fn a_pack_with_no_configs_skips_the_config_archive() {
    let json = CONFIGS.replace("\"noConfigs\": false", "\"noConfigs\": true");
    let modpack = modpack_from("Plain pack".to_string(), "PlainPack", &version(), configs(&json)).expect("pack maps");
    assert!(modpack.archives.iter().all(|archive| archive.name != "Configs.zip"));
}
