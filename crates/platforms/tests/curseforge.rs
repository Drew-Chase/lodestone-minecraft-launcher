use super::*;

/// Recorded slice of the real `/v1/mods/search` response.
const SEARCH: &str = r#"{
    "data": [
        {
            "id": 306612,
            "name": "JarMods",
            "slug": "jarmods",
            "summary": "A test mod",
            "downloadCount": 123456,
            "authors": [{ "id": 1, "name": "Alice" }],
            "logo": { "thumbnailUrl": "https://logo.test/thumb.png", "url": "https://logo.test/full.png" },
            "links": { "websiteUrl": "https://example.com" },
            "dateModified": "2026-01-01T00:00:00Z"
        }
    ],
    "pagination": { "index": 0, "pageSize": 50, "resultCount": 1, "totalCount": 1 }
}"#;

/// Recorded slice of the real `/v1/mods/{id}/files` response.
const FILES: &str = r#"{
    "data": [
        {
            "id": 4838285,
            "displayName": "Test Mod 1.0.0",
            "fileName": "testmod-1.0.0.jar",
            "downloadUrl": "https://mediafilez.forgecdn.net/files/testmod-1.0.0.jar",
            "fileLength": 123456,
            "hashes": [{ "value": "abc", "algo": 1 }],
            "gameVersions": ["1.20.1", "Forge"],
            "modLoader": 1,
            "fileDate": "2026-01-01T00:00:00Z"
        },
        {
            "id": 4838286,
            "displayName": "Test Mod 1.0.0 NeoForge",
            "fileName": "testmod-1.0.0-neo.jar",
            "downloadUrl": "",
            "fileLength": 234567,
            "gameVersions": ["1.20.1"],
            "modLoader": 4,
            "fileDate": "2026-01-02T00:00:00Z"
        }
    ],
    "pagination": { "index": 0, "pageSize": 50, "resultCount": 2, "totalCount": 2 }
}"#;

#[test]
fn class_ids_cover_the_curseforge_classes() {
    assert_eq!(class_id(ContentType::Mod), Some(6));
    assert_eq!(class_id(ContentType::Modpack), Some(4471));
    assert_eq!(class_id(ContentType::ResourcePack), Some(12));
    assert_eq!(class_id(ContentType::World), Some(17));
    assert_eq!(class_id(ContentType::Shader), None);
    assert_eq!(class_id(ContentType::DataPack), None);
}

#[test]
fn loader_ints_map_to_loader_names() {
    assert_eq!(loader_name(Some(1)), ["forge"]);
    assert_eq!(loader_name(Some(4)), ["neoforge"]);
    assert_eq!(loader_name(Some(5)), ["fabric"]);
    assert_eq!(loader_name(Some(6)), ["quilt"]);
    assert!(loader_name(Some(0)).is_empty(), "Any");
    assert!(loader_name(None).is_empty());
}

#[test]
fn every_search_loader_round_trips_through_its_mod_loader_int() {
    for (name, id) in LOADERS {
        assert_eq!(loader_name(Some(i64::from(id))), [name]);
    }
}

#[test]
fn every_sort_maps_onto_a_curseforge_sort_field() {
    assert_eq!(sort_field(SearchSort::Relevance), 1);
    assert_eq!(sort_field(SearchSort::Popular), 2);
    assert_eq!(sort_field(SearchSort::Updated), 3);
    assert_eq!(sort_field(SearchSort::Newest), 11);
}

#[test]
fn the_search_payload_maps_onto_search_results() {
    let list: CfModList = serde_json::from_str(SEARCH).expect("recorded payload parses");
    let results: Vec<SearchResult> = list
        .data
        .into_iter()
        .map(|cf_mod| mod_to_search_result(cf_mod, ContentType::Mod))
        .collect();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "306612");
    assert_eq!(results[0].slug.as_deref(), Some("jarmods"));
    assert_eq!(results[0].name, "JarMods");
    assert_eq!(results[0].author.as_deref(), Some("Alice"));
    assert_eq!(results[0].downloads, 123456);
    assert_eq!(results[0].icon_url.as_deref(), Some("https://logo.test/full.png"));
    assert_eq!(results[0].web_url.as_deref(), Some("https://example.com"));
}

#[test]
fn the_files_payload_maps_onto_versions() {
    let list: CfFileList = serde_json::from_str(FILES).expect("recorded payload parses");
    let versions: Vec<ContentVersion> = list
        .data
        .into_iter()
        .map(|file| file_to_content_version("306612", file, ContentType::Mod))
        .collect();
    assert_eq!(versions.len(), 2);
    assert_eq!(versions[0].id, "4838285");
    assert_eq!(versions[0].project_id.as_deref(), Some("306612"));
    assert_eq!(versions[0].version_number, "Test Mod 1.0.0");
    assert_eq!(versions[0].loaders, ["forge"]);
    assert!(versions[0].minecraft_versions.contains(&"1.20.1".to_string()));
    assert_eq!(versions[0].files[0].sha1.as_deref(), Some("abc"));
    assert_eq!(versions[0].files[0].size, Some(123456));
    // NeoForge file with a hidden download url: url empty, loader mapped.
    assert_eq!(versions[1].loaders, ["neoforge"]);
    assert_eq!(versions[1].files[0].url, "");
}
