use super::query::{PAGE_SIZE, Results};
use super::*;
use lodestone_platforms::{Facet, SearchResult};

/// A hit with `id` and no details.
fn hit(id: &str) -> SearchResult {
    SearchResult {
        id: id.to_string(),
        slug: None,
        name: format!("Pack {id}"),
        author: None,
        description: String::new(),
        downloads: 0,
        icon_url: None,
        web_url: None,
        updated: None,
        content_type: ContentType::Modpack,
    }
}

/// The search a page's effects ask for, if any.
fn search_of(effects: &[Effect]) -> Option<&Search> {
    effects.iter().find_map(|effect| match effect {
        Effect::Search(search) => Some(search),
        _ => None,
    })
}

/// A page on Modrinth with `hits` answered for its first page.
fn answered(hits: usize, total: u64) -> Discover {
    let mut page = Discover::new(Source::Modrinth);
    let effects = page.opening();
    let search = search_of(&effects).expect("opening searches").clone();
    let page_hits = (0..hits).map(|index| hit(&index.to_string())).collect();
    page.update(Message::SearchFinished {
        key: search.key,
        offset: search.offset,
        generation: search.generation,
        result: Ok(SearchPage { hits: page_hits, total }),
        elapsed_ms: 12,
    });
    page
}

#[test]
fn a_new_page_browses_modpacks_with_the_first_sort_of_its_platform() {
    let page = Discover::new(Source::Modrinth);
    assert_eq!(page.content, ContentType::Modpack);
    assert_eq!(page.sort, SearchSort::Relevance);
    assert_eq!(page.view, DiscoverView::Grid);
}

#[test]
fn a_tab_the_platform_does_not_host_is_ignored() {
    let mut page = Discover::new(Source::Ftb);
    let effects = page.update(Message::ContentSelected(ContentType::Shader));
    assert!(effects.is_empty());
    assert_eq!(page.content, ContentType::Modpack);
}

#[test]
fn switching_to_a_platform_without_the_tab_falls_back_to_modpacks() {
    let mut page = Discover::new(Source::Modrinth);
    page.update(Message::ContentSelected(ContentType::Shader));
    assert_eq!(page.content, ContentType::Shader);
    page.update(Message::SourceSelected(Source::Ftb));
    assert_eq!(page.source, Source::Ftb);
    assert_eq!(page.content, ContentType::Modpack);
    assert_eq!(page.sort, SearchSort::Popular, "FTB can only express its popular sort");
}

#[test]
fn a_disabled_platform_cannot_be_picked() {
    // Only CurseForge can be disabled, and only in a build made without its key.
    let Some(disabled) = Source::ALL.into_iter().find(|source| !source.enabled()) else {
        return;
    };
    let mut page = Discover::new(Source::Modrinth);
    let effects = page.update(Message::SourceSelected(disabled));
    assert!(effects.is_empty());
    assert_eq!(page.source, Source::Modrinth);
}

#[test]
fn typing_waits_and_only_the_latest_pause_searches() {
    let mut page = answered(3, 3);
    let first = page.update(Message::QueryChanged("so".to_string()));
    let Some(Effect::Debounce(stale)) = first.first().cloned() else {
        panic!("typing debounces");
    };
    let second = page.update(Message::QueryChanged("sodium".to_string()));
    let Some(Effect::Debounce(latest)) = second.first().cloned() else {
        panic!("typing debounces");
    };
    assert!(page.update(Message::SearchDue(stale)).is_empty(), "an older pause is stale");
    let effects = page.update(Message::SearchDue(latest));
    let search = search_of(&effects).expect("the latest pause searches");
    assert_eq!(search.key.text, "sodium");
    assert_eq!(search.offset, 0);
}

#[test]
fn an_answer_to_an_older_search_is_dropped() {
    let mut page = Discover::new(Source::Modrinth);
    let effects = page.opening();
    let search = search_of(&effects).expect("opening searches").clone();
    page.update(Message::SearchNow);
    let stale = page.update(Message::SearchFinished {
        key: search.key,
        offset: search.offset,
        generation: search.generation,
        result: Ok(SearchPage {
            hits: vec![hit("old")],
            total: 1,
        }),
        elapsed_ms: 5,
    });
    assert!(stale.is_empty());
    assert!(page.results.hits.is_empty(), "the stale page never reaches the screen");
}

#[test]
fn a_recent_first_page_comes_from_the_cache() {
    let mut page = answered(2, 2);
    page.update(Message::ContentSelected(ContentType::ResourcePack));
    let effects = page.update(Message::SearchNow);
    let search = search_of(&effects).expect("a new tab searches").clone();
    page.update(Message::SearchFinished {
        key: search.key,
        offset: search.offset,
        generation: search.generation,
        result: Ok(SearchPage {
            hits: vec![hit("pack")],
            total: 1,
        }),
        elapsed_ms: 9,
    });
    let back = page.update(Message::ContentSelected(ContentType::Modpack));
    assert!(search_of(&back).is_none(), "the first modpack page is still cached");
    assert!(page.results.cached);
    assert_eq!(page.results.hits.len(), 2);
}

#[test]
fn scrolling_near_the_end_asks_for_the_next_page() {
    let mut page = answered(20, 45);
    let far = page.update(Message::Scrolled { remaining: 2000.0 });
    assert!(far.is_empty(), "far from the end, nothing is asked for");
    let near = page.update(Message::Scrolled { remaining: 120.0 });
    let search = search_of(&near).expect("near the end, the next page is asked for");
    assert_eq!(search.offset, 20);
}

#[test]
fn a_platform_that_reports_no_total_has_more_only_while_pages_are_full() {
    let full = Results {
        hits: vec![hit("a"); PAGE_SIZE as usize],
        last_page: PAGE_SIZE as usize,
        ..Results::default()
    };
    assert!(full.has_more());
    let short = Results {
        hits: vec![hit("a"); 3],
        last_page: 3,
        ..Results::default()
    };
    assert!(!short.has_more());
}

#[test]
fn the_filters_follow_the_facets_the_platform_offers() {
    let mut page = answered(0, 0);
    page.update(Message::FacetsLoaded {
        source: Source::Modrinth,
        content: ContentType::Modpack,
        result: Ok(Facets {
            categories: vec![Facet {
                id: "magic".to_string(),
                name: "Magic".to_string(),
            }],
            loaders: vec!["fabric".to_string()],
            game_versions: vec!["1.20.1".to_string()],
        }),
    });
    let effects = page.update(Message::CategorySelected(1));
    assert_eq!(page.category.as_deref(), Some("magic"));
    assert!(search_of(&effects).is_some());
    page.update(Message::CategorySelected(0));
    assert_eq!(page.category, None, "index 0 is any category");
    page.update(Message::LoaderSelected(1));
    assert_eq!(page.loader.as_deref(), Some("fabric"));
}

/// Answers the search `effects` ask for, with `hits` hits out of `total`.
fn answer(page: &mut Discover, effects: &[Effect], hits: usize, total: u64) {
    let search = search_of(effects).expect("a search was asked for").clone();
    let page_hits = (0..hits).map(|index| hit(&index.to_string())).collect();
    page.update(Message::SearchFinished {
        key: search.key,
        offset: search.offset,
        generation: search.generation,
        result: Ok(SearchPage { hits: page_hits, total }),
        elapsed_ms: 12,
    });
}

#[test]
fn a_new_search_starts_the_results_back_at_the_top() {
    let mut page = answered(3, 3);
    page.update(Message::QueryChanged("sodium".to_string()));
    let effects = page.update(Message::SearchDue(page.generation));
    assert!(effects.iter().any(|effect| matches!(effect, Effect::ScrollToTop)));
    assert!(search_of(&effects).is_some());
}

#[test]
fn a_page_the_cache_holds_is_shown_without_asking_the_platform_again() {
    let mut page = answered(20, 40);
    let effects = page.update(Message::Scrolled { remaining: 0.0 });
    answer(&mut page, &effects, 20, 40);
    page.update(Message::ContentSelected(ContentType::ResourcePack));
    let back = page.update(Message::ContentSelected(ContentType::Modpack));
    assert!(search_of(&back).is_none(), "the first page is still cached");
    assert_eq!(page.results.hits.len(), 20);
    let more = page.update(Message::Scrolled { remaining: 0.0 });
    assert!(search_of(&more).is_none(), "the second page is cached too");
    assert_eq!(page.results.hits.len(), 40);
    assert!(page.results.cached);
}
