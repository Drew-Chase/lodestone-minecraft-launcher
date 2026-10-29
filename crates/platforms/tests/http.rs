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
