use super::*;

/// A `/search` answer: one hit with its slug, page and picture.
const SEARCH: &str = r#"{
    "modpacks": [
        { "id": "42", "name": "Hexxit", "slug": "hexxit", "url": "https://www.technicpack.net/modpack/hexxit",
          "iconUrl": "https://icon.test/hexxit.png" }
    ]
}"#;

/// A `/modpack/{slug}` answer for a pack that ships as a zip.
const PACK: &str = r#"{
    "name": "hexxit",
    "displayName": "Hexxit",
    "user": "Technic",
    "description": "Adventure awaits",
    "installs": 1234,
    "minecraft": "1.7.10",
    "version": "1.0.2",
    "url": "https://mirror.test/hexxit-1.0.2.zip",
    "solder": null,
    "icon": { "url": "https://icon.test/hexxit.png", "md5": "d41d8cd98f00b204e9800998ecf8427e" },
    "platformUrl": "https://www.technicpack.net/modpack/hexxit"
}"#;

/// A Solder build's manifest: its Minecraft and Forge versions, and one mod.
const MANIFEST: &str = r#"{
    "minecraft": "1.7.10",
    "forge": "10.13.4.1558",
    "mods": [
        { "name": "Mod", "version": "1.0", "md5": "d41d8cd98f00b204e9800998ecf8427e",
          "url": "https://solder.test/mods/mod-1.0.jar", "filesize": 1234 }
    ]
}"#;

fn pack() -> TechnicModpack {
    serde_json::from_str(PACK).expect("recorded pack parses")
}

fn manifest(json: &str) -> SolderManifest {
    serde_json::from_str(json).expect("recorded manifest parses")
}

fn version() -> ContentVersion {
    ContentVersion {
        id: "1.0.2".to_string(),
        project_id: Some("hexxit".to_string()),
        name: Some("1.0.2".to_string()),
        version_number: "1.0.2".to_string(),
        content_type: ContentType::Modpack,
        minecraft_versions: vec!["1.7.10".to_string()],
        loaders: Vec::new(),
        downloads: 0,
        date: None,
        changelog_url: None,
        files: Vec::new(),
    }
}

#[test]
fn every_request_names_the_launcher() {
    assert_eq!(
        build_url("https://api.technicpack.net/modpack/hexxit"),
        "https://api.technicpack.net/modpack/hexxit?build=lodestone"
    );
    assert_eq!(
        build_url("https://api.technicpack.net/search?q=sky"),
        "https://api.technicpack.net/search?q=sky&build=lodestone"
    );
}

#[test]
fn search_hits_map_onto_results() {
    let hits: TechnicHits = serde_json::from_str(SEARCH).expect("recorded search parses");
    let results: Vec<SearchResult> = hits.modpacks.into_iter().map(hit_to_result).collect();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "hexxit");
    assert_eq!(results[0].name, "Hexxit");
    assert_eq!(results[0].web_url.as_deref(), Some("https://www.technicpack.net/modpack/hexxit"));
    assert_eq!(results[0].icon_url.as_deref(), Some("https://icon.test/hexxit.png"));
}

#[test]
fn a_pack_record_maps_onto_its_details() {
    let details = details_of("hexxit", pack());
    assert_eq!(details.name, "Hexxit");
    assert_eq!(details.author.as_deref(), Some("Technic"));
    assert_eq!(details.downloads, 1234);
    assert_eq!(details.icon_url.as_deref(), Some("https://icon.test/hexxit.png"));
    assert_eq!(details.web_url.as_deref(), Some("https://www.technicpack.net/modpack/hexxit"));
}

#[test]
fn solder_builds_list_newest_first() {
    let versions = solder_versions("hexxit", Some("1.7.10"), vec!["1.0.1".to_string(), "1.0.2".to_string()]);
    let numbers: Vec<&str> = versions.iter().map(|version| version.version_number.as_str()).collect();
    assert_eq!(numbers, ["1.0.2", "1.0.1"]);
    assert_eq!(versions[0].minecraft_versions, ["1.7.10"]);
    assert_eq!(versions[0].project_id.as_deref(), Some("hexxit"));
    assert!(versions[0].files.is_empty(), "a Solder build's mods come from its manifest");
}

#[test]
fn a_zip_pack_has_one_version_that_is_its_zip() {
    let versions = zip_versions("hexxit", &pack());
    assert_eq!(versions.len(), 1);
    assert_eq!(versions[0].version_number, "1.0.2");
    assert_eq!(versions[0].files[0].name, "hexxit-1.0.2.zip");
    assert_eq!(versions[0].files[0].url, "https://mirror.test/hexxit-1.0.2.zip");
}

#[test]
fn the_file_name_is_the_last_part_of_a_url() {
    assert_eq!(file_name("https://solder.test/mods/mod-1.0.jar"), "mod-1.0.jar");
    assert_eq!(file_name("plain"), "plain");
}

#[test]
fn forge_is_joined_to_its_minecraft_version_once() {
    assert_eq!(combined_forge("1.7.10", "10.13.4.1558"), "1.7.10-10.13.4.1558");
    assert_eq!(combined_forge("1.7.10", "1.7.10-10.13.4.1558"), "1.7.10-10.13.4.1558");
    assert_eq!(combined_forge("", "10.13.4.1558"), "10.13.4.1558");
}

#[test]
fn a_solder_manifest_maps_onto_a_forge_modpack() {
    let modpack = solder_modpack("Hexxit".to_string(), Some("1.12.2".to_string()), &version(), manifest(MANIFEST)).expect("build maps");
    assert_eq!(modpack.name, "Hexxit");
    assert_eq!(modpack.minecraft, "1.7.10", "the manifest's Minecraft version wins over the pack's");
    assert_eq!(modpack.loader, Some("forge"));
    assert_eq!(modpack.loader_version.as_deref(), Some("1.7.10-10.13.4.1558"));
    assert_eq!(modpack.files.len(), 1);
    assert_eq!(modpack.files[0].path, "mods/mod-1.0.jar");
    assert_eq!(modpack.files[0].url, "https://solder.test/mods/mod-1.0.jar");
    assert_eq!(modpack.files[0].size, Some(1234));
}

#[test]
fn a_manifest_without_minecraft_takes_the_packs() {
    let json = MANIFEST.replace("\"minecraft\": \"1.7.10\",", "");
    let modpack = solder_modpack("Hexxit".to_string(), Some("1.12.2".to_string()), &version(), manifest(&json)).expect("build maps");
    assert_eq!(modpack.minecraft, "1.12.2");
    assert_eq!(modpack.loader_version.as_deref(), Some("1.12.2-10.13.4.1558"));
}

#[test]
fn a_build_that_names_no_forge_stops_the_install() {
    let json = MANIFEST.replace("\"forge\": \"10.13.4.1558\"", "\"forge\": null");
    let error = solder_modpack("Hexxit".to_string(), None, &version(), manifest(&json)).expect_err("no loader");
    assert!(matches!(error, PlatformError::UnsupportedPlatform { .. }));
}
