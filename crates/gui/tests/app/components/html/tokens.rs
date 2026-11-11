use super::*;

/// A start tag with the given attributes.
fn start(name: &str, attributes: &[(&str, &str)]) -> Token<'static> {
    Token::Start {
        name: name.to_owned(),
        attributes: attributes.iter().map(|(key, value)| ((*key).to_owned(), (*value).to_owned())).collect(),
        closed: false,
    }
}

/// Text.
fn text(value: &str) -> Token<'static> {
    Token::Text(Cow::Owned(value.to_owned()))
}

#[test]
fn tags_and_text_read_in_order_with_lowercase_names() {
    assert_eq!(
        tokenize("<P Class=\"x\">Hi</P>"),
        vec![start("p", &[("class", "x")]), text("Hi"), Token::End("p".to_owned())]
    );
}

#[test]
fn a_quoted_value_keeps_its_greater_than_sign() {
    assert_eq!(
        tokenize("<img alt=\"a > b\" src='c.png'>"),
        vec![start("img", &[("alt", "a > b"), ("src", "c.png")])]
    );
}

#[test]
fn bare_and_empty_attributes_read() {
    assert_eq!(
        tokenize("<iframe src=https://x/y allowfullscreen width = 638>"),
        vec![start("iframe", &[("src", "https://x/y"), ("allowfullscreen", ""), ("width", "638")])]
    );
}

#[test]
fn a_repeated_attribute_keeps_its_first_value() {
    assert_eq!(tokenize("<a href=\"one\" HREF=\"two\">"), vec![start("a", &[("href", "one")])]);
}

#[test]
fn attribute_values_and_text_decode_their_references() {
    assert_eq!(
        tokenize("<a title=\"Q&amp;A\">&lt;3</a>"),
        vec![start("a", &[("title", "Q&A")]), text("<3"), Token::End("a".to_owned())]
    );
}

#[test]
fn a_self_closing_tag_says_so() {
    assert_eq!(
        tokenize("<br/><img src=\"a\" />"),
        vec![
            Token::Start {
                name: "br".to_owned(),
                attributes: Vec::new(),
                closed: true,
            },
            Token::Start {
                name: "img".to_owned(),
                attributes: vec![("src".to_owned(), "a".to_owned())],
                closed: true,
            },
        ]
    );
}

#[test]
fn comments_declarations_and_instructions_are_skipped() {
    assert_eq!(
        tokenize("<!DOCTYPE html><?xml x?>a<!-- <p>hidden</p> -->b<!-->c"),
        vec![text("a"), text("b"), text("c")]
    );
}

#[test]
fn the_content_of_script_and_style_is_skipped() {
    assert_eq!(
        tokenize("<style>p > a { color: red }</style>x<SCRIPT>if (a < b) {}</script >y"),
        vec![
            start("style", &[]),
            Token::End("style".to_owned()),
            text("x"),
            start("script", &[]),
            Token::End("script".to_owned()),
            text("y"),
        ]
    );
}

#[test]
fn a_script_that_never_ends_takes_the_rest_of_the_document() {
    assert_eq!(tokenize("<script>alert(1)<p>gone"), vec![start("script", &[])]);
}

#[test]
fn a_less_than_sign_that_opens_no_tag_is_text() {
    assert_eq!(
        tokenize("1 < 2 <3 </ 4"),
        vec![text("1 "), text("<"), text(" 2 "), text("<"), text("3 "), text("<"), text("/ 4")]
    );
}

#[test]
fn a_tag_that_never_closes_is_text() {
    assert_eq!(tokenize("before <img src=\"a.png\""), vec![text("before "), text("<img src=\"a.png\"")]);
}

#[test]
fn a_quote_that_never_closes_runs_to_the_next_greater_than_sign() {
    assert_eq!(
        tokenize("<img alt=\"broken>after"),
        vec![start("img", &[("alt", "broken")]), text("after")]
    );
}
