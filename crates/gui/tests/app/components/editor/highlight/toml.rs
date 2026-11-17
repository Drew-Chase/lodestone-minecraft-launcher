use super::super::tests::{pieces, read, token_of};
use super::*;
use crate::app::components::editor::language::Language::Toml;

#[test]
fn a_table_header_takes_its_brackets_and_a_comment_may_follow() {
    assert_eq!(pieces(Toml, "[general]"), [("[general]".to_string(), Token::Section)]);
    assert_eq!(pieces(Toml, "  [[mods]]"), [("[[mods]]".to_string(), Token::Section)]);
    let line = pieces(Toml, "[servers.alpha] # the first");
    assert_eq!(token_of(&line, "[servers.alpha]"), Some(Token::Section));
    assert_eq!(token_of(&line, "# the first"), Some(Token::Comment));
}

#[test]
fn an_array_value_is_not_a_table_header() {
    let line = pieces(Toml, "list = [1, 2]");
    assert_eq!(token_of(&line, "list"), Some(Token::Key));
    assert_eq!(token_of(&line, "["), Some(Token::Punctuation));
    assert_eq!(token_of(&line, "2"), Some(Token::Number));
    let continued = pieces(Toml, "  [1, 2],");
    assert_eq!(token_of(&continued, "["), Some(Token::Punctuation), "a row of a nested array");
}

#[test]
fn keys_and_the_kinds_of_value_have_their_colours() {
    let line = pieces(Toml, r#"name = "lithium" # mod"#);
    assert_eq!(token_of(&line, "name"), Some(Token::Key));
    assert_eq!(token_of(&line, "="), Some(Token::Punctuation));
    assert_eq!(token_of(&line, r#""lithium""#), Some(Token::Text));
    assert_eq!(token_of(&line, "# mod"), Some(Token::Comment));
    assert_eq!(token_of(&pieces(Toml, "enabled = true"), "true"), Some(Token::Constant));
    assert_eq!(token_of(&pieces(Toml, "threads = 4"), "4"), Some(Token::Number));
    assert_eq!(
        token_of(&pieces(Toml, "when = 2026-01-02T03:04:05Z"), "2026-01-02T03:04:05Z"),
        Some(Token::Number)
    );
    assert_eq!(token_of(&pieces(Toml, "a.b.c = 1"), "a.b.c"), Some(Token::Key), "a dotted key");
    assert_eq!(token_of(&pieces(Toml, r#""quoted key" = 1"#), r#""quoted key""#), Some(Token::Key));
}

#[test]
fn a_literal_string_keeps_its_backslashes() {
    let line = pieces(Toml, r"path = 'C:\mods\' # windows");
    assert_eq!(
        token_of(&line, r"'C:\mods\'"),
        Some(Token::Text),
        "the backslash does not hide the closing quote"
    );
    assert_eq!(token_of(&line, "# windows"), Some(Token::Comment));
}

#[test]
fn a_hash_inside_a_string_is_not_a_comment() {
    let line = pieces(Toml, r##"colour = "#ff00aa""##);
    assert_eq!(token_of(&line, r##""#ff00aa""##), Some(Token::Text));
    assert_eq!(line.len(), 3);
}

#[test]
fn a_multi_line_string_runs_until_its_closing_quotes() {
    let (first, carry) = read(Toml, Carry::None, r#"text = """first"#);
    assert_eq!(carry, Carry::Text(b'"'));
    assert_eq!(token_of(&first, r#""""first"#), Some(Token::Text));
    let (middle, carry) = read(Toml, carry, "key = 1 # not a comment");
    assert_eq!((middle.len(), carry), (1, Carry::Text(b'"')), "the whole line is string");
    let (last, carry) = read(Toml, carry, r#"end""" # done"#);
    assert_eq!(carry, Carry::None);
    assert_eq!(token_of(&last, r#"end""""#), Some(Token::Text));
    assert_eq!(token_of(&last, "# done"), Some(Token::Comment));
    assert_eq!(read(Toml, Carry::None, r#"one = """all on one line""""#).1, Carry::None);
    assert_eq!(read(Toml, Carry::None, "lit = '''open").1, Carry::Text(b'\''));
}
