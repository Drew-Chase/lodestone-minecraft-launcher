use super::*;
use chrono::TimeZone;

/// A moment the feed is read at, for the tests that do not look at it.
fn at() -> DateTime<Local> {
    Local.timestamp_opt(1_700_000_000, 0).single().expect("a valid local time")
}

/// A news document: one post the types describe, and one in a category they do not name, which is skipped.
const NEWS: &str = r#"{"version": 1, "entries": [
    {"title": "Discover Planet Earth III DLC", "category": "Minecraft for Windows", "date": "2024-01-16",
     "text": "Step into a wondrous world.",
     "playPageImage": {"title": "play", "url": "/images/a.jpeg"},
     "newsPageImage": {"title": "news", "url": "/images/b.jpeg", "dimensions": {"width": 772, "height": 350}},
     "readMoreLink": "https://www.minecraft.net/article/planet-earth-iii-perspectives",
     "newsType": ["News page", "Bedrock"], "id": "2OKkEL0h71H7dyzTilA8ah"},
    {"title": "A post in a category the types do not name", "category": "Minecraft Education", "date": "2024-01-01",
     "text": "x",
     "playPageImage": {"title": "play", "url": "/images/c.jpeg"},
     "newsPageImage": {"title": "news", "url": "/images/d.jpeg"},
     "readMoreLink": "https://www.minecraft.net/article/education",
     "newsType": ["News page"], "id": "education"}
]}"#;

/// A patch-notes document: one note, and one with no `contentPath`, which the types require, so it is skipped.
const PATCHES: &str = r#"{"version": 1, "entries": [
    {"title": "Minecraft Snapshot 24w03b", "type": "snapshot", "version": "24w03b",
     "image": {"url": "/images/6PHW.jpeg", "title": "Minecraft Snapshot 24w03b"},
     "body": "<p>We are releasing Snapshot 24w03b.</p>",
     "contentPath": "javaPatchNotes/6PHW.json", "id": "6PHW"},
    {"title": "A note with no content path", "type": "snapshot", "version": "x",
     "image": {"url": "/images/x.jpeg", "title": "x"}, "body": "<p>x</p>", "id": "no-path"}
]}"#;

#[test]
fn a_feed_reads_the_entries_it_describes_and_counts_the_rest() {
    let feed = Feed::read(NEWS.as_bytes(), PATCHES.as_bytes(), at()).expect("the feed reads");
    assert_eq!(feed.posts.len(), 1);
    assert_eq!(feed.posts[0].title, "Discover Planet Earth III DLC");
    assert_eq!(feed.patches.len(), 1);
    assert_eq!(feed.patches[0].version, "24w03b");
    assert_eq!(feed.skipped, 2);
    assert_eq!(feed.fetched_at, at());
}

#[test]
fn a_document_without_its_entries_or_that_is_not_json_is_an_error() {
    assert!(Feed::read(b"{}", PATCHES.as_bytes(), at()).is_err());
    assert!(Feed::read(b"not json", PATCHES.as_bytes(), at()).is_err());
    assert!(Feed::read(NEWS.as_bytes(), b"{\"entries\": 3}", at()).is_err());
}

#[test]
fn a_picture_path_joins_onto_the_content_site_and_a_full_address_stays() {
    assert_eq!(absolute("/images/a.jpeg"), "https://launchercontent.mojang.com/images/a.jpeg");
    assert_eq!(absolute("images/a.jpeg"), "https://launchercontent.mojang.com/images/a.jpeg");
    assert_eq!(absolute("https://example.com/a.png"), "https://example.com/a.png");
}
