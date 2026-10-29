use super::*;

/// The documented search shape (currently 401-gated live; shape from the
/// historical public API).
const SEARCH: &str = r#"{
    "builds": [
        { "id": 1, "name": "Hexxit", "slug": "hexxit", "description": "Adventure awaits", "icon_url": "https://icon.test/hexxit.png" }
    ]
}"#;

/// The documented pack shape (currently 401-gated live).
const PACK: &str = r#"{
    "name": "Hexxit",
    "user": "Technic",
    "description": "Adventure awaits",
    "recommended": "1.0.1",
    "latest": "1.0.2",
    "builds": ["1.0.1", "1.0.2"]
}"#;

#[test]
fn the_search_shape_maps_onto_search_results() {
    let hits: TechnicSearchResponse = serde_json::from_str(SEARCH).expect("documented shape parses");
    let results: Vec<SearchResult> = hits.builds.into_iter().map(search_to_content_version).collect();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "hexxit");
    assert_eq!(results[0].name, "Hexxit");
    assert_eq!(results[0].web_url.as_deref(), Some("https://www.technicpack.net/modpack/hexxit"));
}

#[test]
fn the_pack_shape_maps_onto_details_and_versions() {
    let pack: TechnicPack = serde_json::from_str(PACK).expect("documented shape parses");
    let details = pack.into_details("hexxit");
    assert_eq!(details.name, "Hexxit");
    assert_eq!(details.author.as_deref(), Some("Technic"));
    assert_eq!(details.web_url.as_deref(), Some("https://www.technicpack.net/modpack/hexxit"));
}

#[test]
fn builds_map_onto_versions_with_download_archives() {
    let pack: TechnicPack = serde_json::from_str(PACK).expect("documented shape parses");
    let versions: Vec<ContentVersion> = pack.builds.into_iter().map(|build| build_to_content_version("hexxit", build)).collect();
    assert_eq!(versions.len(), 2);
    assert_eq!(versions[0].version_number, "1.0.1");
    assert_eq!(versions[0].files[0].url, "https://api.technicpack.net/modpack/hexxit/download/1.0.1");
    assert_eq!(versions[0].files[0].name, "hexxit-1.0.1.zip");
}
