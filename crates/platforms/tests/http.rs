use super::*;

#[test]
fn query_appending_uses_the_right_separator() {
    assert_eq!(with_query("https://x.test/a", "q", "abc"), "https://x.test/a?q=abc");
    assert_eq!(with_query("https://x.test/a?b=1", "q", "abc"), "https://x.test/a?b=1&q=abc");
}

#[test]
fn query_values_are_percent_encoded() {
    assert_eq!(with_query("https://x.test/a", "q", "space here"), "https://x.test/a?q=space%20here");
    assert_eq!(with_query("https://x.test/a", "q", "a&b=c"), "https://x.test/a?q=a%26b%3Dc");
    assert_eq!(with_query("https://x.test/a", "q", "hülle"), "https://x.test/a?q=h%C3%BClle");
}

#[test]
fn an_answer_is_kept_until_it_is_one_ttl_old() {
    let start = Instant::now();
    let mut cache = AnswerCache::new(Duration::from_secs(600), 8);
    cache.put("https://x.test/a".to_string(), "{}".to_string(), start);
    assert_eq!(cache.get("https://x.test/a", start + Duration::from_secs(599)), Some("{}"));
    assert_eq!(cache.get("https://x.test/a", start + Duration::from_secs(600)), None);
}

#[test]
fn the_oldest_answer_is_dropped_when_the_cache_is_full() {
    let start = Instant::now();
    let at = |seconds: u64| start + Duration::from_secs(seconds);
    let mut cache = AnswerCache::new(Duration::from_secs(600), 2);
    cache.put("a".to_string(), "1".to_string(), at(0));
    cache.put("b".to_string(), "2".to_string(), at(1));
    cache.put("c".to_string(), "3".to_string(), at(2));
    assert_eq!(cache.get("a", at(3)), None);
    assert_eq!(cache.get("b", at(3)), Some("2"));
    assert_eq!(cache.get("c", at(3)), Some("3"));
}

#[test]
fn a_url_answered_again_takes_the_new_answer_without_evicting_another() {
    let start = Instant::now();
    let at = |seconds: u64| start + Duration::from_secs(seconds);
    let mut cache = AnswerCache::new(Duration::from_secs(600), 2);
    cache.put("a".to_string(), "1".to_string(), at(0));
    cache.put("b".to_string(), "2".to_string(), at(1));
    cache.put("a".to_string(), "3".to_string(), at(2));
    assert_eq!(cache.get("a", at(3)), Some("3"));
    assert_eq!(cache.get("b", at(3)), Some("2"));
}
