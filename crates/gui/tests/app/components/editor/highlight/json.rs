use super::super::tests::{pieces, read, token_of};
use super::*;
use crate::app::components::editor::language::Language::Json;

#[test]
fn a_key_is_told_from_a_string_value_by_its_colon() {
    let line = pieces(Json, r#"  "weather": "FANCY","#);
    assert_eq!(token_of(&line, r#""weather""#), Some(Token::Key));
    assert_eq!(token_of(&line, r#""FANCY""#), Some(Token::Text));
    assert_eq!(token_of(&line, ":"), Some(Token::Punctuation));
    assert_eq!(token_of(&line, ","), Some(Token::Punctuation));
}

#[test]
fn numbers_and_constants_are_values() {
    let line = pieces(Json, r#"{"a": -1.5e3, "b": true, "c": null}"#);
    assert_eq!(token_of(&line, "-1.5e3"), Some(Token::Number));
    assert_eq!(token_of(&line, "true"), Some(Token::Constant));
    assert_eq!(token_of(&line, "null"), Some(Token::Constant));
}

#[test]
fn an_escaped_quote_stays_inside_its_string() {
    let line = pieces(Json, r#""say \"hi\"": 1"#);
    assert_eq!(token_of(&line, r#""say \"hi\"""#), Some(Token::Key));
}

#[test]
fn json5_and_nbt_forms_are_coloured_too() {
    let line = pieces(Json, "{id: 'minecraft:stone', Count: 1b} // trailing");
    assert_eq!(token_of(&line, "id"), Some(Token::Key), "a bare key");
    assert_eq!(token_of(&line, "'minecraft:stone'"), Some(Token::Text), "single quotes");
    assert_eq!(token_of(&line, "1b"), Some(Token::Number), "a typed number");
    assert_eq!(token_of(&line, "// trailing"), Some(Token::Comment));
}

#[test]
fn a_block_comment_may_end_on_its_own_line_or_run_on() {
    assert_eq!(read(Json, Carry::None, "/* open").1, Carry::Comment);
    let (line, next) = read(Json, Carry::Comment, "still */ 5");
    assert_eq!(next, Carry::None);
    assert_eq!(line, [("still */".to_string(), Token::Comment), ("5".to_string(), Token::Number)]);
    assert_eq!(
        read(Json, Carry::Comment, "").1,
        Carry::Comment,
        "an empty line inside a comment keeps it open"
    );
    let (line, next) = read(Json, Carry::None, "1 /* in */ 2");
    assert_eq!(next, Carry::None);
    assert_eq!(line.len(), 3);
}

#[test]
fn text_outside_ascii_is_left_whole() {
    let line = pieces(Json, r#""名前": "café ✓""#);
    assert_eq!(token_of(&line, r#""名前""#), Some(Token::Key));
    assert_eq!(token_of(&line, r#""café ✓""#), Some(Token::Text));
}
