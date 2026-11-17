use super::super::tests::{pieces, read, token_of};
use super::*;
use crate::app::components::editor::language::Language::Yaml;

#[test]
fn a_key_ends_at_its_colon_and_its_value_has_its_own_colour() {
    let line = pieces(Yaml, "  view-distance: 10 # chunks");
    assert_eq!(token_of(&line, "view-distance"), Some(Token::Key));
    assert_eq!(token_of(&line, ":"), Some(Token::Punctuation));
    assert_eq!(token_of(&line, "10"), Some(Token::Number));
    assert_eq!(token_of(&line, "# chunks"), Some(Token::Comment));
    assert_eq!(token_of(&pieces(Yaml, "enabled: true"), "true"), Some(Token::Constant));
    assert_eq!(token_of(&pieces(Yaml, "motd: \"A server\""), "\"A server\""), Some(Token::Text));
    assert_eq!(
        pieces(Yaml, "settings:"),
        [("settings".to_string(), Token::Key), (":".to_string(), Token::Punctuation)]
    );
}

#[test]
fn a_colon_inside_a_value_does_not_make_a_key() {
    let line = pieces(Yaml, "url: http://example.com:8080/path");
    assert_eq!(token_of(&line, "url"), Some(Token::Key));
    assert_eq!(line.len(), 2, "the address is plain text");
    assert!(pieces(Yaml, "- 10:30").iter().all(|(_, token)| *token != Token::Key));
}

#[test]
fn a_plain_value_of_several_words_is_not_a_number() {
    let line = pieces(Yaml, "version: 1.20.1 release");
    assert_eq!(line.len(), 2, "only the key and the colon are coloured");
}

#[test]
fn list_dashes_and_document_marks_are_punctuation() {
    let line = pieces(Yaml, "  - name: alpha");
    assert_eq!(line[0], ("-".to_string(), Token::Punctuation));
    assert_eq!(token_of(&line, "name"), Some(Token::Key));
    assert_eq!(pieces(Yaml, "---"), [("---".to_string(), Token::Punctuation)]);
    assert_eq!(token_of(&pieces(Yaml, "- -5"), "-5"), Some(Token::Number), "a negative number in a list");
}

#[test]
fn a_hash_needs_a_space_before_it_to_start_a_comment() {
    assert_eq!(pieces(Yaml, "# whole line"), [("# whole line".to_string(), Token::Comment)]);
    let line = pieces(Yaml, "colour: a#b");
    assert!(line.iter().all(|(_, token)| *token != Token::Comment));
}

#[test]
fn a_block_string_takes_the_lines_indented_under_its_key() {
    let (first, carry) = read(Yaml, Carry::None, "  description: |");
    assert_eq!(carry, Carry::Block(2));
    assert_eq!(token_of(&first, "|"), Some(Token::Punctuation));
    let (inside, carry) = read(Yaml, carry, "    key: not a key # nor a comment");
    assert_eq!(inside, [("key: not a key # nor a comment".to_string(), Token::Text)]);
    assert_eq!(read(Yaml, carry, "").1, carry, "an empty line stays in the block");
    let (after, carry) = read(Yaml, carry, "  next: 1");
    assert_eq!(carry, Carry::None);
    assert_eq!(token_of(&after, "next"), Some(Token::Key));
    assert_eq!(read(Yaml, Carry::None, "text: >-").1, Carry::Block(0));
    assert_eq!(
        read(Yaml, Carry::None, "math: a > b").1,
        Carry::None,
        "a greater-than sign in a value opens nothing"
    );
}

#[test]
fn anchors_tags_and_flow_brackets_are_marked() {
    let line = pieces(Yaml, "base: &defaults {a: 1, b: [x, y]}");
    assert_eq!(token_of(&line, "&defaults"), Some(Token::Tag));
    assert_eq!(token_of(&line, "{"), Some(Token::Punctuation));
    assert_eq!(token_of(&pieces(Yaml, "copy: *defaults"), "*defaults"), Some(Token::Tag));
    assert_eq!(token_of(&pieces(Yaml, "when: !!timestamp 2026"), "!!timestamp"), Some(Token::Tag));
}
