use super::*;

#[test]
fn versions_compare_by_their_numbers() {
    assert_eq!(natural_cmp("47.10.0", "47.9.0"), Ordering::Greater);
    assert_eq!(natural_cmp("0.16.10", "0.9.0"), Ordering::Greater);
    assert_eq!(natural_cmp("1.21.4", "1.21.4"), Ordering::Equal);
    assert_eq!(natural_cmp("1.20.1-47.3.0", "1.20.1-47.2.9"), Ordering::Greater);
    assert_eq!(natural_cmp("21.1", "21.1.5"), Ordering::Less);
}

#[test]
fn sorting_newest_first_puts_the_highest_build_on_top() {
    let mut builds = vec!["0.9.0", "0.16.10", "0.16.9", "0.15.11"];
    builds.sort_by(|a, b| natural_cmp(b, a));
    assert_eq!(builds, ["0.16.10", "0.16.9", "0.15.11", "0.9.0"]);
}
