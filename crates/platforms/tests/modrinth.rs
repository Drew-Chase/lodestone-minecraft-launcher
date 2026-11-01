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

/// A search hit with the given gallery fields, written as the JSON they would be.
fn hit_with_gallery(featured: &str, gallery: &str) -> SearchResult {
    let hit: ModrinthSearchHit = serde_json::from_str(&format!(
        r#"{{ "project_id": "AANobbMI", "title": "Sodium", "author": "jellysquid3", "description": "Fast rendering",
             "featured_gallery": {featured}, "gallery": {gallery} }}"#
    ))
    .expect("recorded hit parses");
    search_result(hit, ContentType::Mod)
}

#[test]
fn a_search_hit_banner_is_its_featured_image_else_its_first_gallery_image() {
    let featured = hit_with_gallery("\"https://cdn.example/featured.webp\"", "[\"https://cdn.example/a.webp\"]");
    assert_eq!(featured.banner_url.as_deref(), Some("https://cdn.example/featured.webp"));
    let first = hit_with_gallery("null", "[\"https://cdn.example/a.webp\", \"https://cdn.example/b.webp\"]");
    assert_eq!(first.banner_url.as_deref(), Some("https://cdn.example/a.webp"));
    assert_eq!(hit_with_gallery("null", "[]").banner_url, None);
}

#[test]
fn a_project_runs_on_the_sides_it_is_required_or_optional_on() {
    assert_eq!(environments(SideType::Required, SideType::Unsupported), vec![Side::Client]);
    assert_eq!(environments(SideType::Unknown, SideType::Optional), vec![Side::Server]);
    assert_eq!(environments(SideType::Required, SideType::Required), vec![Side::Client, Side::Server]);
}

#[test]
fn a_licence_without_a_name_shows_its_id() {
    assert_eq!(
        license_name(String::new(), "LicenseRef-Polyform-Shield-1.0.0".to_string()).as_deref(),
        Some("LicenseRef-Polyform-Shield-1.0.0")
    );
    assert_eq!(license_name("MIT License".to_string(), "MIT".to_string()).as_deref(), Some("MIT License"));
    assert_eq!(license_name(String::new(), String::new()), None);
}

#[test]
fn the_owner_of_a_team_comes_first_and_the_rest_keep_their_order() {
    let creator = |name: &str, role: &str| Creator {
        name: name.to_string(),
        role: Some(role.to_string()),
        avatar_url: None,
    };
    let names: Vec<String> = owner_first(vec![
        creator("IMS", "Maintainer"),
        creator("robotkoer", "Owner"),
        creator("alexjs", "Member"),
    ])
    .into_iter()
    .map(|creator| creator.name)
    .collect();
    assert_eq!(names, ["robotkoer", "IMS", "alexjs"]);
}

#[test]
fn a_version_names_its_channel_and_keeps_the_dependencies_it_models() {
    assert_eq!(channel(VersionType::Beta), Channel::Beta);
    let embedded: Dependency =
        serde_json::from_str(r#"{ "version_id": "cqRSJ7PC", "project_id": "ZDCwiIc3", "file_name": null, "dependency_type": "embedded" }"#)
            .expect("recorded dependency parses");
    assert_eq!(
        dependency(embedded),
        Some(VersionDependency {
            kind: DependencyKind::Embedded,
            project_id: Some("ZDCwiIc3".to_string()),
            version_id: Some("cqRSJ7PC".to_string()),
        })
    );
    let unknown: Dependency =
        serde_json::from_str(r#"{ "version_id": null, "project_id": "AANobbMI", "file_name": null, "dependency_type": "something-new" }"#)
            .expect("recorded dependency parses");
    assert_eq!(dependency(unknown), None);
}
