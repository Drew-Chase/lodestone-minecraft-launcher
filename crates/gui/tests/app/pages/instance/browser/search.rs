use super::super::Message as Browser;
use super::*;
use crate::app::folder_listing::{Entries, read_rows};
use std::fs;
use std::path::{Path, PathBuf};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lodestone-browser-search-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("the scratch folder can be made");
    dir
}

fn put(root: &Path, relative: &str, contents: &[u8]) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("a file has a parent")).expect("the folders can be made");
    fs::write(path, contents).expect("the file can be written");
}

/// The paths a walk found, sorted, so the order of the file system does not matter.
fn found(results: &Results) -> Vec<String> {
    let mut paths: Vec<String> = results.rows.iter().map(|row| row.path.clone()).collect();
    paths.sort();
    paths
}

fn search(root: &Path, folder: &str, subfolders: bool, contents: bool, needle: &str) -> Results {
    walk(root, folder, subfolders, contents, needle, &AtomicBool::new(false))
}

#[test]
fn a_name_search_stays_in_the_folder_unless_subfolders_are_asked_for() {
    let root = scratch("names");
    put(&root, "notes.txt", b"");
    put(&root, "sub/notes-2.txt", b"");
    put(&root, "sub/deeper/notes-3.md", b"");
    assert_eq!(found(&search(&root, "", false, false, "notes")), ["notes.txt"]);
    assert_eq!(
        found(&search(&root, "", true, false, "notes")),
        ["notes.txt", "sub/deeper/notes-3.md", "sub/notes-2.txt"]
    );
    assert_eq!(
        found(&search(&root, "sub", false, false, "notes")),
        ["sub/notes-2.txt"],
        "a folder on screen searches from itself"
    );
}

#[test]
fn a_folder_whose_name_matches_is_a_match_and_says_it_is_a_folder() {
    let root = scratch("folders");
    fs::create_dir_all(root.join("config-old")).expect("the folder can be made");
    let results = search(&root, "", false, false, "config");
    assert_eq!(results.rows.len(), 1);
    assert!(results.rows[0].is_folder);
    assert_eq!(results.rows[0].path, "config-old");
}

#[test]
fn a_content_search_reads_text_and_skips_binary_files() {
    let root = scratch("contents");
    put(&root, "readme.txt", b"Hello World, from the config");
    put(&root, "blob.dat", b"\0Hello World hidden in binary");
    assert_eq!(found(&search(&root, "", false, true, "hello world")), ["readme.txt"]);
    assert!(
        search(&root, "", false, false, "hello world").rows.is_empty(),
        "without contents, only names are searched"
    );
}

#[test]
fn a_file_over_the_content_limit_is_matched_by_its_name_alone() {
    let root = scratch("large");
    let mut big = vec![b'a'; CONTENT_LIMIT as usize + 1];
    big.extend_from_slice(b"needle");
    put(&root, "big.log", &big);
    assert!(search(&root, "", false, true, "needle").rows.is_empty());
    assert_eq!(found(&search(&root, "", false, true, "big")), ["big.log"]);
}

#[test]
fn links_are_listed_but_not_followed() {
    let root = scratch("links");
    put(&root, "sub/inner.txt", b"");
    #[cfg(unix)]
    std::os::unix::fs::symlink(root.join("sub"), root.join("link")).expect("the link can be made");
    #[cfg(unix)]
    assert_eq!(
        found(&search(&root, "", true, false, "inner")),
        ["sub/inner.txt"],
        "the link's folder is not walked twice"
    );
}

#[test]
fn a_search_stops_at_its_limit_and_says_so() {
    let root = scratch("limit");
    for index in 0..(LIMIT + 5) {
        put(&root, &format!("match-{index}.txt"), b"");
    }
    let results = search(&root, "", false, false, "match");
    assert_eq!(results.rows.len(), LIMIT);
    assert!(results.truncated);
}

#[test]
fn a_cancelled_walk_returns_nothing_more() {
    let root = scratch("cancel");
    put(&root, "notes.txt", b"");
    let results = walk(&root, "", false, false, "notes", &AtomicBool::new(true));
    assert!(results.rows.is_empty());
    assert!(!results.truncated);
}

#[test]
fn a_new_query_cancels_the_walk_in_flight_and_renews_its_count() {
    let mut search = Search::default();
    let before = search.request;
    let flag = search.walk_flag();
    let renewed = search.renew();
    assert_eq!(renewed, before + 1);
    assert!(flag.load(Ordering::Relaxed), "the walk started before the new query is cancelled");
    assert!(!search.walk_flag().load(Ordering::Relaxed), "the new query's walk is not cancelled");
}

#[test]
fn a_query_shows_its_matches_and_clearing_it_brings_the_folder_back() {
    let root = scratch("state");
    put(&root, "a.txt", b"");
    put(&root, "b.txt", b"");
    let mut state = State::new(root.clone(), false);
    let rows = read_rows(&root, "", Entries::All).expect("the folder reads");
    let _ = state.update(Browser::Listed {
        root: root.clone(),
        request: state.request,
        folder: String::new(),
        result: Ok(rows),
    });
    assert_eq!(state.rows.len(), 2);

    let _ = state.search_message(Message::Query("a".to_string()));
    assert!(
        state.search.shown && !state.search.ready,
        "the list says it is searching until the walk lands"
    );

    let request = state.search.request;
    let matches = search(&root, "", false, false, "a");
    let _ = state.search_message(Message::Found {
        request,
        rows: matches.rows,
        truncated: false,
    });
    assert!(state.search.ready);
    assert_eq!(state.rows.len(), 1, "the list shows the match");

    let _ = state.search_message(Message::Clear);
    assert!(!state.search.shown && !state.search.active());
    assert_eq!(state.rows.len(), 2, "the folder's own rows come back");
}

#[test]
fn an_answer_for_a_query_that_has_been_replaced_is_dropped() {
    let root = scratch("stale");
    put(&root, "a.txt", b"");
    let mut state = State::new(root.clone(), false);
    let _ = state.search_message(Message::Query("a".to_string()));
    let stale = state.search.request;
    let _ = state.search_message(Message::Query("ab".to_string()));
    let _ = state.search_message(Message::Found {
        request: stale,
        rows: Vec::new(),
        truncated: false,
    });
    assert!(!state.search.ready, "the answer for the older query does not stand for the newer one");
}
