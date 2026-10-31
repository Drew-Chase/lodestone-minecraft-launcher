use super::*;

#[test]
fn the_project_type_leads_and_each_filter_follows_as_its_own_group() {
    let query = SearchQuery {
        category: Some("magic".to_string()),
        loader: Some("fabric".to_string()),
        game_version: Some("1.20.1".to_string()),
        ..SearchQuery::new(ContentType::Modpack)
    };
    let groups = facet_groups(&query, "modpack");
    assert_eq!(groups.len(), 4);
    assert_eq!(groups[0], ["project_type:modpack"]);
    assert_eq!(groups[1], ["categories:magic"]);
    assert_eq!(groups[2], ["categories:fabric"]);
    assert_eq!(groups[3], ["versions:1.20.1"]);
}

#[test]
fn a_query_without_filters_only_names_the_project_type() {
    let groups = facet_groups(&SearchQuery::new(ContentType::ResourcePack), "resourcepack");
    assert_eq!(groups, vec![vec!["project_type:resourcepack".to_string()]]);
}

#[test]
fn every_sort_maps_onto_modrinths_index() {
    assert_eq!(sort_index(SearchSort::Relevance), "relevance");
    assert_eq!(sort_index(SearchSort::Popular), "downloads");
    assert_eq!(sort_index(SearchSort::Updated), "updated");
    assert_eq!(sort_index(SearchSort::Newest), "newest");
}

#[test]
fn a_search_hit_keeps_its_update_date_and_links_to_its_project() {
    let hit: ModrinthSearchHit = serde_json::from_str(
        r#"{ "project_id": "AANobbMI", "slug": "sodium", "title": "Sodium", "author": "jellysquid3",
             "description": "Fast rendering", "downloads": 42, "icon_url": null,
             "date_modified": "2026-10-01T12:00:00Z" }"#,
    )
    .expect("recorded hit parses");
    let result = search_result(hit, ContentType::Mod);
    assert_eq!(result.id, "AANobbMI");
    assert_eq!(result.web_url.as_deref(), Some("https://modrinth.com/project/AANobbMI"));
    assert_eq!(result.updated.as_deref(), Some("2026-10-01T12:00:00Z"));
    assert_eq!(result.downloads, 42);
}
