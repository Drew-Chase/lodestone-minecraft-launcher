use super::*;

#[test]
fn display_names_capitalize_each_word() {
    assert_eq!(display_name("kitchen-sink"), "Kitchen Sink");
    assert_eq!(display_name("magic"), "Magic");
    assert_eq!(display_name("fabric_api"), "Fabric Api");
    assert_eq!(display_name(""), "");
}

#[test]
fn a_new_query_starts_at_the_first_page_with_no_filters() {
    let query = SearchQuery::new(ContentType::Shader);
    assert_eq!(query.content, ContentType::Shader);
    assert_eq!(query.sort, SearchSort::Relevance);
    assert_eq!((query.offset, query.limit), (0, 20));
    assert!(query.text.is_empty());
    assert!(query.categories.is_empty() && query.loaders.is_empty() && query.game_versions.is_empty());
    assert!(query.environments.is_empty() && !query.open_source);
    assert!(query.excluded_categories.is_empty() && query.excluded_loaders.is_empty());
}

#[test]
fn a_banner_is_the_featured_image_then_the_first_gallery_image() {
    let gallery = || vec!["first.webp".to_string(), "second.webp".to_string()];
    assert_eq!(
        banner_from(Some("featured.webp".to_string()), gallery()).as_deref(),
        Some("featured.webp")
    );
    assert_eq!(banner_from(None, gallery()).as_deref(), Some("first.webp"));
    assert_eq!(banner_from(None, Vec::new()), None);
}
