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
            "downloadCount": 4321,
            "hashes": [{ "value": "abc", "algo": 1 }],
            "gameVersions": ["1.20.1", "Forge"],
            "fileDate": "2026-01-01T00:00:00Z"
        },
        {
            "id": 4838286,
            "displayName": "Test Mod 1.0.0 NeoForge",
            "fileName": "testmod-1.0.0-neo.jar",
            "downloadUrl": "",
            "fileLength": 234567,
            "gameVersions": ["1.20.1", "NeoForge", "Client"],
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
    assert_eq!(class_id(ContentType::Shader), Some(6552));
    assert_eq!(class_id(ContentType::DataPack), Some(6945));
}

#[test]
fn curseforge_hosts_shaders_and_data_packs() {
    assert!(CurseForge::CONTENT_TYPES.contains(&ContentType::Shader));
    assert!(CurseForge::CONTENT_TYPES.contains(&ContentType::DataPack));
}

#[test]
fn only_mods_and_modpacks_take_a_loader() {
    assert!(takes_loader(ContentType::Mod));
    assert!(takes_loader(ContentType::Modpack));
    for content in [ContentType::ResourcePack, ContentType::World, ContentType::Shader, ContentType::DataPack] {
        assert!(!takes_loader(content), "{content} takes no loader");
    }
}

#[test]
fn a_files_loaders_are_the_loader_names_in_its_game_versions() {
    assert_eq!(loaders_of(&["1.20.1".to_string(), "Forge".to_string()]), ["forge"]);
    assert_eq!(loaders_of(&["NeoForge".to_string(), "Client".to_string()]), ["neoforge"]);
    assert!(loaders_of(&["1.20.1".to_string(), "Datapack".to_string()]).is_empty());
}

#[test]
fn every_search_loader_is_named_in_a_files_game_versions() {
    for (name, _) in LOADERS {
        let display = format!("{}{}", name[..1].to_ascii_uppercase(), &name[1..]);
        assert_eq!(loaders_of(&[display]), [name]);
    }
}

#[test]
fn a_files_game_versions_split_into_minecraft_versions_and_loaders() {
    let game_versions = ["1.21.1", "Fabric", "Client", "Java 21", "NeoForge", "Datapack", "26.3", "b1.7.3"].map(String::from);
    assert_eq!(minecraft_versions_of(&game_versions), ["1.21.1", "26.3", "b1.7.3"]);
    assert_eq!(loaders_of(&game_versions), ["fabric", "neoforge"]);
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
    assert_eq!(versions[0].downloads, 4321, "the file's own download count");
    // NeoForge file with a hidden download url: url empty, loader mapped.
    assert_eq!(versions[1].loaders, ["neoforge"]);
    assert_eq!(versions[1].minecraft_versions, ["1.20.1"], "the environment is no version");
    assert_eq!(versions[1].files[0].url, "");
}

#[test]
fn a_project_banners_its_first_screenshot_and_lists_them_all_as_its_gallery() {
    let shots: Vec<CfScreenshot> = serde_json::from_str(
        r#"[
            { "title": "Overview", "description": "", "thumbnailUrl": "https://img.test/a-thumb.png", "url": "https://img.test/a.png" },
            { "title": null, "thumbnailUrl": null, "url": "https://img.test/b.png" }
        ]"#,
    )
    .expect("recorded screenshots parse");
    assert_eq!(banner_of(&shots).as_deref(), Some("https://img.test/a-thumb.png"));
    let gallery = gallery_of(shots);
    assert_eq!(gallery.len(), 2);
    assert_eq!(gallery[0].thumbnail_url, "https://img.test/a-thumb.png");
    assert_eq!(gallery[0].full_url, "https://img.test/a.png");
    assert_eq!(gallery[0].title.as_deref(), Some("Overview"));
    assert_eq!(gallery[0].description, None, "an empty description is no description");
    // A screenshot with no thumbnail shows its full picture as its thumbnail.
    assert_eq!(gallery[1].thumbnail_url, "https://img.test/b.png");
    assert_eq!(gallery[1].title, None);
}

#[test]
fn a_file_names_its_release_channel_and_the_projects_it_relates_to() {
    let file: CfFile = serde_json::from_str(
        r#"{
            "id": 9, "fileName": "testmod-2.0.0-beta.jar", "fileLength": 1, "releaseType": 2,
            "dependencies": [
                { "modId": 111, "relationType": 3 },
                { "modId": 222, "relationType": 2 },
                { "modId": 333, "relationType": 4 },
                { "modId": 444, "relationType": 5 }
            ]
        }"#,
    )
    .expect("recorded file parses");
    let version = file_to_content_version("306612", file, ContentType::Mod);
    assert_eq!(version.channel, Some(Channel::Beta));
    // Tools (relation 4) are not modelled, so they are left out.
    assert_eq!(
        version.dependencies,
        vec![
            VersionDependency {
                kind: DependencyKind::Required,
                project_id: Some("111".to_string()),
                version_id: None,
            },
            VersionDependency {
                kind: DependencyKind::Optional,
                project_id: Some("222".to_string()),
                version_id: None,
            },
            VersionDependency {
                kind: DependencyKind::Incompatible,
                project_id: Some("444".to_string()),
                version_id: None,
            },
        ]
    );
}

#[test]
fn the_files_listing_names_how_many_files_the_project_has() {
    let list: CfFileList = serde_json::from_str(FILES).expect("recorded payload parses");
    assert_eq!(list.pagination.map(|pagination| pagination.total_count), Some(2));
}

/// Recorded slice of the real `/v1/mods/{id}/files/{fileId}` response. The download url is hidden.
const FILE: &str = r#"{
    "data": {
        "id": 4838285,
        "fileName": "testmod-1.0.0.jar",
        "downloadUrl": null,
        "fileLength": 123456,
        "hashes": [
            { "value": "0123456789abcdef0123456789abcdef01234567", "algo": 1 },
            { "value": "ff", "algo": 2 }
        ]
    }
}"#;

#[test]
fn a_file_detail_names_the_jar_and_its_sha1_and_size() {
    let envelope: CfFileEnvelope = serde_json::from_str(FILE).expect("recorded file parses");
    assert_eq!(envelope.data.file_name, "testmod-1.0.0.jar");
    assert_eq!(
        sha1_of(&envelope.data.hashes).as_deref(),
        Some("0123456789abcdef0123456789abcdef01234567")
    );
    assert_eq!(envelope.data.file_length, 123456);
    assert!(envelope.data.download_url.is_none());
}

const FINGERPRINT_URL: &str = "https://api.curseforge.com/v1/fingerprints/432";

#[test]
fn a_fingerprint_names_the_project_and_file_of_its_exact_match() {
    let answer = serde_json::json!({
        "data": {
            "isCacheBuilt": true,
            "exactMatches": [
                { "id": 4123456, "file": { "id": 4123456, "modId": 238222, "fileName": "pack.zip" }, "latestFiles": [] }
            ],
            "exactFingerprints": [1540447798],
            "unmatchedFingerprints": []
        }
    });
    assert_eq!(
        fingerprint_match_of(answer, FINGERPRINT_URL).expect("answer parses"),
        Some(FingerprintMatch {
            project_id: "238222".to_string(),
            file_id: "4123456".to_string(),
        })
    );
}

#[test]
fn an_answer_with_no_exact_match_names_no_file() {
    let answer = serde_json::json!({
        "data": { "isCacheBuilt": true, "exactMatches": [], "unmatchedFingerprints": [1540447798] }
    });
    assert_eq!(fingerprint_match_of(answer, FINGERPRINT_URL).expect("answer parses"), None);
}

#[test]
fn an_exact_match_with_no_file_record_is_passed_over() {
    let answer = serde_json::json!({
        "data": { "exactMatches": [{ "id": 9 }, { "id": 10, "file": { "id": 10, "modId": 77 } }] }
    });
    assert_eq!(
        fingerprint_match_of(answer, FINGERPRINT_URL).expect("answer parses"),
        Some(FingerprintMatch {
            project_id: "77".to_string(),
            file_id: "10".to_string(),
        })
    );
}

#[test]
fn an_answer_that_is_not_a_fingerprint_answer_is_an_error_and_not_a_match() {
    let answer = serde_json::json!({ "unexpected": true });
    assert!(fingerprint_match_of(answer, FINGERPRINT_URL).is_err());
}

#[test]
fn a_listing_leaves_out_the_files_curseforge_says_are_unavailable() {
    let list: CfFileList = serde_json::from_str(
        r#"{ "data": [
            { "id": 1, "fileName": "a.jar", "isAvailable": true },
            { "id": 2, "fileName": "b.jar", "isAvailable": false },
            { "id": 3, "fileName": "c.jar" }
        ] }"#,
    )
    .expect("recorded listing parses");
    let kept: Vec<i64> = available(list.data).into_iter().map(|file| file.id).collect();
    assert_eq!(kept, [1, 3], "a file that says nothing about availability is kept");
}

#[test]
fn a_file_id_splits_into_all_but_its_last_three_digits_and_those_three() {
    assert_eq!(
        direct_url(9394038, "FreshAnimations_v1.11.2_26.1.zip"),
        "https://mediafilez.forgecdn.net/files/9394/38/FreshAnimations_v1.11.2_26.1.zip"
    );
    assert_eq!(
        direct_url(9323162, "xaerominimap-forge-26.3-26.6.2.jar"),
        "https://mediafilez.forgecdn.net/files/9323/162/xaerominimap-forge-26.3-26.6.2.jar"
    );
}

#[test]
fn a_direct_url_percent_encodes_the_file_name() {
    assert_eq!(
        direct_url(9394038, "FA 1.11.2 + x.zip"),
        "https://mediafilez.forgecdn.net/files/9394/38/FA%201.11.2%20%2B%20x.zip"
    );
}

#[test]
fn a_file_record_without_a_url_gets_its_direct_cdn_address() {
    let direct = "https://mediafilez.forgecdn.net/files/9394/38/FreshAnimations_v1.11.2_26.1.zip";
    assert_eq!(download_link(None, 9394038, "FreshAnimations_v1.11.2_26.1.zip"), direct);
    assert_eq!(
        download_link(Some(String::new()), 9394038, "FreshAnimations_v1.11.2_26.1.zip"),
        direct,
        "an empty url is no url"
    );
    assert_eq!(
        download_link(Some("https://edge.forgecdn.net/files/1/2/a.jar".to_string()), 9394038, "a.jar"),
        "https://edge.forgecdn.net/files/1/2/a.jar",
        "the record's own url is used as it is"
    );
}
