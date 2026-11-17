use super::super::tests::{pieces, token_of};
use super::*;
use crate::app::components::editor::language::Language::Function;

#[test]
fn a_comment_line_is_a_comment() {
    assert_eq!(pieces(Function, "# Runs every tick"), [("# Runs every tick".to_string(), Token::Comment)]);
}

#[test]
fn the_command_and_the_words_that_chain_it_are_keywords() {
    let line = pieces(Function, "execute as @a at @s run say hi");
    assert_eq!(token_of(&line, "execute"), Some(Token::Keyword));
    for word in ["as", "at", "run"] {
        assert_eq!(token_of(&line, word), Some(Token::Keyword), "{word}");
    }
    assert_eq!(
        token_of(&line, "say"),
        None,
        "a command after `run` is not marked, since any word may follow"
    );
    assert_eq!(line.iter().filter(|(text, _)| text == "@a" || text == "@s").count(), 2);
    assert_eq!(token_of(&line, "@a"), Some(Token::Constant));
}

#[test]
fn coordinates_and_numbers_are_numbers() {
    let line = pieces(Function, "tp @s ~ ~1.5 ^-2 10 0.5f");
    for number in ["~", "~1.5", "^-2", "10", "0.5f"] {
        assert_eq!(token_of(&line, number), Some(Token::Number), "{number}");
    }
}

#[test]
fn namespaced_ids_and_block_tags_are_marked() {
    let line = pieces(Function, "give @p minecraft:diamond_sword{Damage:5} 1");
    assert_eq!(token_of(&line, "minecraft:diamond_sword"), Some(Token::Tag));
    assert_eq!(token_of(&line, "{"), Some(Token::Punctuation));
    let tagged = pieces(Function, "execute if block ~ ~ ~ #minecraft:logs run function mypack:tick/main");
    assert_eq!(token_of(&tagged, "#minecraft:logs"), Some(Token::Tag));
    assert_eq!(token_of(&tagged, "mypack:tick/main"), Some(Token::Tag));
}

#[test]
fn selector_arguments_and_strings_are_coloured() {
    let line = pieces(Function, r#"tellraw @a[tag=!done,limit=1] {"text":"hi there"}"#);
    assert_eq!(token_of(&line, "["), Some(Token::Punctuation));
    assert_eq!(token_of(&line, "1"), Some(Token::Number));
    assert_eq!(token_of(&line, r#""hi there""#), Some(Token::Text));
}

#[test]
fn a_macro_line_marks_its_dollar_and_still_has_a_command() {
    let line = pieces(Function, "$say $(name)");
    assert_eq!(line[0], ("$".to_string(), Token::Punctuation));
    assert_eq!(token_of(&line, "say"), Some(Token::Keyword));
}
