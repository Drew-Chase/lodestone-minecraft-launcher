use iced::advanced::text::Highlighter as _;

use super::*;

/// The coloured pieces of `line` in `language`, as text, and what the line leaves open.
pub(crate) fn read(language: Language, carry: Carry, line: &str) -> (Vec<(String, Token)>, Carry) {
    let mut spans = Spans::new();
    let next = lex(language, carry, line, &mut spans);
    let mut last = 0;
    for (range, _) in &spans {
        assert!(range.start >= last, "the spans of {line:?} overlap or are out of order: {spans:?}");
        assert!(
            range.start < range.end && range.end <= line.len(),
            "a span of {line:?} is empty or past the end: {spans:?}"
        );
        assert!(
            line.is_char_boundary(range.start) && line.is_char_boundary(range.end),
            "a span of {line:?} splits a character"
        );
        last = range.end;
    }
    (spans.into_iter().map(|(range, token)| (line[range].to_string(), token)).collect(), next)
}

/// The coloured pieces of a line that follows nothing left open.
pub(crate) fn pieces(language: Language, line: &str) -> Vec<(String, Token)> {
    read(language, Carry::None, line).0
}

/// The token `text` was given in `pieces`.
pub(crate) fn token_of(pieces: &[(String, Token)], text: &str) -> Option<Token> {
    pieces.iter().find(|(piece, _)| piece == text).map(|(_, token)| *token)
}

fn settings(language: Language) -> Settings {
    Settings { language, revision: 0 }
}

/// Feeds `lines` to the highlighter from the line it says it is at, as iced does.
fn colour(highlighter: &mut Highlighter, lines: &[&str]) -> Vec<Vec<Token>> {
    let from = highlighter.current_line();
    lines[from..]
        .iter()
        .map(|line| highlighter.highlight_line(line).map(|(_, token)| token).collect())
        .collect()
}

#[test]
fn a_block_comment_colours_the_lines_it_runs_over() {
    let lines = ["{", "  /* a comment", "  \"still\": 1", "  */ \"key\": 2", "}"];
    let mut highlighter = Highlighter::new(&settings(Language::Json));
    let tokens = colour(&mut highlighter, &lines);
    assert_eq!(tokens[1], [Token::Comment]);
    assert_eq!(tokens[2], [Token::Comment], "the line inside the comment is all comment");
    assert_eq!(tokens[3], [Token::Comment, Token::Key, Token::Punctuation, Token::Number]);
    assert_eq!(highlighter.current_line(), 5);
}

#[test]
fn an_edit_colours_again_from_the_changed_line_with_what_that_line_was_handed() {
    let mut lines = vec!["{", "  /* a comment", "  \"inside\": 1", "  */", "}"];
    let mut highlighter = Highlighter::new(&settings(Language::Json));
    let _ = colour(&mut highlighter, &lines);

    // The line inside the comment changes. It is still handed an open comment, so it stays a comment.
    lines[2] = "  \"edited\": 2";
    highlighter.change_line(2);
    assert_eq!(highlighter.current_line(), 2);
    let tokens = colour(&mut highlighter, &lines);
    assert_eq!(tokens[0], [Token::Comment]);

    // The comment's opening line loses its `/*`, so the lines after it are code again.
    lines[1] = "  \"a\": 0,";
    highlighter.change_line(1);
    let tokens = colour(&mut highlighter, &lines);
    assert_eq!(tokens[1], [Token::Key, Token::Punctuation, Token::Number]);
}

#[test]
fn a_change_below_the_line_reached_does_not_move_the_highlighter_forward() {
    let mut highlighter = Highlighter::new(&settings(Language::Toml));
    let _ = colour(&mut highlighter, &["a = 1"]);
    highlighter.change_line(9);
    assert_eq!(highlighter.current_line(), 1);
}

#[test]
fn new_settings_start_the_colouring_over() {
    let mut highlighter = Highlighter::new(&settings(Language::Json));
    let _ = colour(&mut highlighter, &["{", "}"]);
    highlighter.update(&Settings {
        language: Language::Toml,
        revision: 1,
    });
    assert_eq!(highlighter.current_line(), 0);
    let tokens = colour(&mut highlighter, &["[table]"]);
    assert_eq!(tokens[0], [Token::Section], "the new language is used");
}

#[test]
fn plain_text_has_no_colours() {
    assert!(pieces(Language::Plain, "{ \"a\": 1 } # nothing").is_empty());
}

#[test]
fn numbers_are_told_from_words() {
    for number in [
        "0",
        "-12",
        "+3.5",
        "1e9",
        "0xFF",
        "1_000",
        ".5",
        "1b",
        "0.5f",
        "2026-01-02T03:04:05Z",
        "10:30:00",
        "inf",
        "-nan",
    ] {
        assert!(is_number(number), "{number} is a number");
    }
    for word in ["", "-", "abc", "1abc", "0x", "e9", "true", "v1.2"] {
        assert!(!is_number(word), "{word} is not a number");
    }
}

#[test]
fn a_string_ends_at_its_quote_and_an_escaped_quote_does_not_end_it() {
    assert_eq!(string_end(br#""a\"b" rest"#, 0, b'"'), (6, true));
    assert_eq!(string_end(br#""open"#, 0, b'"'), (5, false));
    assert_eq!(
        string_end(br#""ends in a slash\"#, 0, b'"'),
        (17, false),
        "a trailing backslash does not read past the line"
    );
    assert_eq!(string_rest(b"rest` after", 0, b'`'), (5, true));
}

#[test]
fn every_token_has_a_colour() {
    let theme = Theme::Dark;
    for token in [
        Token::Comment,
        Token::Key,
        Token::Text,
        Token::Number,
        Token::Constant,
        Token::Keyword,
        Token::Punctuation,
        Token::Section,
        Token::Tag,
        Token::Attribute,
        Token::Error,
        Token::Warning,
        Token::Info,
        Token::Debug,
        Token::Faint,
    ] {
        assert!(format(&token, &theme).color.is_some());
    }
    assert_ne!(
        format(&Token::Key, &theme).color,
        format(&Token::Text, &theme).color,
        "a key and its value differ"
    );
}
