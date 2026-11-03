use super::*;
use chrono::TimeZone;

/// A moment `seconds` after a fixed point, for the feeds' fetch times.
fn at(seconds: i64) -> DateTime<Local> {
    Local.timestamp_opt(1_700_000_000 + seconds, 0).single().expect("a valid local time")
}

/// A feed of three posts across two categories, and two patch notes, fetched at `fetched`.
fn feed(fetched: DateTime<Local>) -> Feed {
    let news = r#"{"entries": [
        {"title": "Old java", "category": "Minecraft: Java Edition", "date": "2024-01-01", "text": "a",
         "playPageImage": {"title": "p", "url": "/a.jpg"}, "newsPageImage": {"title": "n", "url": "/an.jpg"},
         "readMoreLink": "https://www.minecraft.net/a", "newsType": ["Java"], "id": "a"},
        {"title": "New bedrock", "category": "Minecraft for Windows", "date": "2024-03-01", "text": "b",
         "playPageImage": {"title": "p", "url": "/b.jpg"}, "newsPageImage": {"title": "n", "url": "/bn.jpg"},
         "readMoreLink": "https://www.minecraft.net/b", "newsType": ["Bedrock"], "id": "b"},
        {"title": "New java", "category": "Minecraft: Java Edition", "date": "2024-02-01", "text": "c",
         "playPageImage": {"title": "p", "url": "/c.jpg"}, "newsPageImage": {"title": "n", "url": "/cn.jpg"},
         "readMoreLink": "https://www.minecraft.net/c", "newsType": ["Java"], "id": "c"}
    ]}"#;
    let patches = r#"{"entries": [
        {"title": "Snapshot 1", "type": "snapshot", "version": "1", "image": {"url": "/p1.jpg", "title": "p"},
         "body": "<p>One</p>", "contentPath": "x", "id": "p1"},
        {"title": "Snapshot 2", "type": "snapshot", "version": "2", "image": {"url": "/p2.jpg", "title": "p"},
         "body": "<p>Two</p>", "contentPath": "y", "id": "p2"}
    ]}"#;
    Feed::read(news.as_bytes(), patches.as_bytes(), fetched).expect("the fixture reads")
}

/// The URL a fixture picture path joins to.
fn site(path: &str) -> String {
    format!("https://launchercontent.mojang.com{path}")
}

#[test]
fn a_new_page_has_no_feed_and_its_feed_is_stale() {
    let state = State::default();
    assert!(!state.has_feed());
    assert!(state.is_stale(at(0)));
}

#[test]
fn a_fetch_under_way_is_not_started_again_until_it_fails_or_lands() {
    let mut state = State::default();
    assert!(state.begin_fetch());
    assert!(!state.begin_fetch());
    state.fetched(Err("offline".to_owned()));
    assert!(state.begin_fetch());
}

#[test]
fn a_feed_goes_stale_after_half_an_hour() {
    let mut state = State::default();
    state.fetched(Ok(feed(at(0))));
    assert!(!state.is_stale(at(29 * 60)));
    assert!(state.is_stale(at(31 * 60)));
}

#[test]
fn a_failed_fetch_keeps_the_feed_already_on_hand() {
    let mut state = State::default();
    state.fetched(Ok(feed(at(0))));
    state.fetched(Err("offline".to_owned()));
    assert!(state.has_feed());
    assert_eq!(state.error.as_deref(), Some("offline"));
}

#[test]
fn a_cached_feed_never_replaces_one_already_fetched() {
    let mut state = State::default();
    state.fetched(Ok(feed(at(100))));
    state.cached(Some(feed(at(0))));
    assert_eq!(state.feed.as_ref().map(|feed| feed.fetched_at), Some(at(100)));
}

#[test]
fn posts_list_newest_first_and_the_category_filter_narrows_them() {
    let mut state = State::default();
    state.fetched(Ok(feed(at(0))));
    let feed = state.feed.clone().expect("a feed is on hand");
    assert_eq!(state.filtered(&feed), vec![1, 2, 0]);
    state.update(Message::Category(Some(Category::MinecraftJavaEdition)));
    assert_eq!(state.filtered(&feed), vec![2, 0]);
}

#[test]
fn the_featured_post_uses_its_wide_picture_and_each_picture_is_asked_for_once() {
    let mut state = State::default();
    state.fetched(Ok(feed(at(0))));
    let asked = state.take_missing_pictures();
    assert_eq!(asked, vec![site("/bn.jpg"), site("/c.jpg"), site("/a.jpg")]);
    assert!(state.take_missing_pictures().is_empty(), "a picture is asked for once");
}

#[test]
fn the_patch_notes_ask_for_their_own_pictures_on_their_tab() {
    let mut state = State::default();
    state.fetched(Ok(feed(at(0))));
    state.update(Message::Tab(Tab::Patches));
    assert_eq!(state.take_missing_pictures(), vec![site("/p1.jpg"), site("/p2.jpg")]);
}

#[test]
fn show_more_widens_the_list_and_a_new_filter_starts_it_again() {
    let mut state = State::default();
    assert_eq!(state.limit(), PAGE_SIZE);
    state.update(Message::ShowMore);
    assert_eq!(state.limit(), 2 * PAGE_SIZE);
    state.update(Message::Category(None));
    assert_eq!(state.limit(), PAGE_SIZE);
}

#[test]
fn reading_a_patch_note_opens_the_reader_until_the_tab_changes() {
    let mut state = State::default();
    state.fetched(Ok(feed(at(0))));
    state.update(Message::ReadPatch(1));
    assert_eq!(state.reading.as_ref().map(|reading| reading.index), Some(1));
    state.update(Message::Tab(Tab::Patches));
    assert!(state.reading.is_none());
}

#[test]
fn a_post_opens_its_minecraft_net_article() {
    let mut state = State::default();
    state.fetched(Ok(feed(at(0))));
    assert_eq!(state.post_link(1).as_deref(), Some("https://www.minecraft.net/b"));
    assert_eq!(state.post_link(9), None);
}
