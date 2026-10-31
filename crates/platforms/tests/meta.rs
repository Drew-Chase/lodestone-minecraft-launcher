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
    assert!(query.category.is_none() && query.loader.is_none() && query.game_version.is_none());
}
