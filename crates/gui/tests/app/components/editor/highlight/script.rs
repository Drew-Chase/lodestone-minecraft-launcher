use super::super::tests::{pieces, read, token_of};
use super::*;
use crate::app::components::editor::language::Language::Script;

#[test]
fn keywords_strings_and_numbers_have_their_colours() {
    let line = pieces(Script, "const count = 42; // how many");
    assert_eq!(token_of(&line, "const"), Some(Token::Keyword));
    assert_eq!(token_of(&line, "42"), Some(Token::Number));
    assert_eq!(token_of(&line, ";"), Some(Token::Punctuation));
    assert_eq!(token_of(&line, "// how many"), Some(Token::Comment));
    assert_eq!(token_of(&line, "count"), None, "a name is plain");
    let call = pieces(Script, "event.shapeless('minecraft:stick', [\"oak\", true, null])");
    assert_eq!(token_of(&call, "'minecraft:stick'"), Some(Token::Text));
    assert_eq!(token_of(&call, "\"oak\""), Some(Token::Text));
    assert_eq!(token_of(&call, "true"), Some(Token::Constant));
    assert_eq!(token_of(&call, "null"), Some(Token::Constant));
}

#[test]
fn a_keyword_inside_a_longer_name_or_after_a_dot_path_is_left_alone() {
    let line = pieces(Script, "let format = this.forEach(item)");
    assert_eq!(token_of(&line, "let"), Some(Token::Keyword));
    assert_eq!(token_of(&line, "this"), Some(Token::Keyword));
    assert_eq!(token_of(&line, "format"), None, "`for` inside `format`");
    assert_eq!(token_of(&line, "for"), None);
    assert_eq!(token_of(&pieces(Script, "x = 1.5"), "1.5"), Some(Token::Number), "a number keeps its dot");
}

#[test]
fn a_block_comment_runs_over_lines() {
    let (_, carry) = read(Script, Carry::None, "let a = 1 /* start");
    assert_eq!(carry, Carry::Comment);
    let (middle, carry) = read(Script, carry, " * function not a keyword");
    assert_eq!((middle.len(), carry), (1, Carry::Comment));
    let (last, carry) = read(Script, carry, " */ return a");
    assert_eq!(carry, Carry::None);
    assert_eq!(token_of(&last, "return"), Some(Token::Keyword));
}

#[test]
fn a_template_string_runs_over_lines_and_an_ordinary_one_does_not() {
    let (first, carry) = read(Script, Carry::None, "const text = `line one");
    assert_eq!(carry, Carry::Text(b'`'));
    assert_eq!(token_of(&first, "`line one"), Some(Token::Text));
    let (middle, carry) = read(Script, carry, "const still = text");
    assert_eq!(middle, [("const still = text".to_string(), Token::Text)]);
    let (last, carry) = read(Script, carry, "end` + 1");
    assert_eq!(carry, Carry::None);
    assert_eq!(token_of(&last, "end`"), Some(Token::Text));
    assert_eq!(token_of(&last, "1"), Some(Token::Number));
    assert_eq!(
        read(Script, Carry::None, "let s = 'unclosed").1,
        Carry::None,
        "a quote left open ends with its line"
    );
    assert_eq!(
        read(Script, Carry::Text(b'`'), "").1,
        Carry::Text(b'`'),
        "an empty line stays in the template"
    );
}

#[test]
fn zenscript_bracket_handlers_are_marked_and_comparisons_are_not() {
    let line = pieces(
        Script,
        "craftingTable.addShapeless(\"a\", <item:minecraft:stick> * 4, [<tag:items:forge:rods>]);",
    );
    assert_eq!(token_of(&line, "<item:minecraft:stick>"), Some(Token::Tag));
    assert_eq!(token_of(&line, "<tag:items:forge:rods>"), Some(Token::Tag));
    let compare = pieces(Script, "if (a < b && c > d) {");
    assert!(compare.iter().all(|(_, token)| *token != Token::Tag));
}

#[test]
fn a_directive_at_the_start_of_a_line_is_a_keyword() {
    assert_eq!(pieces(Script, "#priority 10")[0], ("#priority".to_string(), Token::Keyword));
    assert_eq!(pieces(Script, "#version 150")[0], ("#version".to_string(), Token::Keyword));
    assert_eq!(token_of(&pieces(Script, "uniform sampler2D tex;"), "sampler2D"), Some(Token::Keyword));
}
