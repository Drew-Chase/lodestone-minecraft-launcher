use super::*;

/// The search for `text`, sorted by relevance, with no filters.
fn search(text: &str) -> Key {
    Key {
        source: Source::Modrinth,
        content: ContentType::Modpack,
        text: text.to_string(),
        sort: SearchSort::Relevance,
        category: None,
        loader: None,
        game_version: None,
    }
}

/// A page with no hits. Its total tells it apart from the other pages in these tests.
fn page(total: u64) -> SearchPage {
    SearchPage { hits: Vec::new(), total }
}

/// The total of the page at `offset` of `key`, if the cache holds it fresh at `now`.
fn total_at(cache: &Cache, key: &Key, offset: u32, now: Instant) -> Option<u64> {
    cache.get_at(key, offset, now).map(|(page, _)| page.total)
}

#[test]
fn a_page_is_kept_for_ten_minutes() {
    let start = Instant::now();
    let mut cache = Cache::default();
    cache.put_at(search("sodium"), 0, page(40), 12, start);
    assert_eq!(total_at(&cache, &search("sodium"), 0, start + Duration::from_secs(599)), Some(40));
    assert_eq!(total_at(&cache, &search("sodium"), 0, start + Duration::from_secs(600)), None);
}

#[test]
fn each_offset_of_a_search_is_its_own_page() {
    let now = Instant::now();
    let mut cache = Cache::default();
    cache.put_at(search("sodium"), 0, page(40), 12, now);
    cache.put_at(search("sodium"), PAGE_SIZE, page(41), 9, now);
    assert_eq!(total_at(&cache, &search("sodium"), 0, now), Some(40));
    assert_eq!(total_at(&cache, &search("sodium"), PAGE_SIZE, now), Some(41));
    assert_eq!(total_at(&cache, &search("sodium"), 2 * PAGE_SIZE, now), None);
}

#[test]
fn a_filter_or_a_sort_makes_a_different_search() {
    let now = Instant::now();
    let mut cache = Cache::default();
    cache.put_at(search("sodium"), 0, page(40), 12, now);
    let filtered = Key {
        category: Some("optimization".to_string()),
        ..search("sodium")
    };
    let newest = Key {
        sort: SearchSort::Newest,
        ..search("sodium")
    };
    assert_eq!(total_at(&cache, &filtered, 0, now), None);
    assert_eq!(total_at(&cache, &newest, 0, now), None);
}

#[test]
fn the_oldest_page_goes_when_the_cache_is_full() {
    let start = Instant::now();
    let mut cache = Cache::default();
    let last = CAPACITY as u32;
    for index in 0..=last {
        let at = start + Duration::from_secs(u64::from(index));
        cache.put_at(search("sodium"), index * PAGE_SIZE, page(u64::from(index)), 1, at);
    }
    let now = start + Duration::from_secs(u64::from(last));
    assert_eq!(total_at(&cache, &search("sodium"), 0, now), None, "the oldest page went");
    assert_eq!(total_at(&cache, &search("sodium"), last * PAGE_SIZE, now), Some(u64::from(last)));
}
