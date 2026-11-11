use super::*;

/// Keeps every web address and rejects the rest.
fn resolve(href: &str) -> Option<String> {
    href.starts_with("https://").then(|| href.to_owned())
}

/// A page with pictures before, inside and after two spoilers, one picture shown twice.
const PAGE: &str = "<p><img src=\"https://i.test/1.png\"></p>\
    <div class=\"spoiler\"><img src=\"https://i.test/2.png\"><ul><li><img src=\"https://i.test/3.png\"></li></ul></div>\
    <table><tr><td><img src=\"https://i.test/4.png\"></td></tr></table>\
    <blockquote><img src=\"https://i.test/1.png\"></blockquote>\
    <details><summary>More</summary><img src=\"https://i.test/5.png\"></details>";

#[test]
fn pictures_list_in_reading_order_without_the_closed_spoilers() {
    let document = parse(PAGE, resolve);
    assert_eq!(document.pictures(&HashSet::new()), vec!["https://i.test/1.png", "https://i.test/4.png"]);
}

#[test]
fn an_open_spoiler_adds_its_pictures_where_it_sits() {
    let document = parse(PAGE, resolve);
    assert_eq!(
        document.pictures(&HashSet::from([0])),
        vec![
            "https://i.test/1.png",
            "https://i.test/2.png",
            "https://i.test/3.png",
            "https://i.test/4.png"
        ]
    );
    assert_eq!(
        document.pictures(&HashSet::from([1])),
        vec!["https://i.test/1.png", "https://i.test/4.png", "https://i.test/5.png"]
    );
}

#[test]
fn a_page_with_no_drawable_content_is_empty() {
    assert!(parse("<p>&nbsp;</p><!-- nothing --><script>x()</script>", resolve).is_empty());
    assert!(!parse("<p>x</p>", resolve).is_empty());
}
