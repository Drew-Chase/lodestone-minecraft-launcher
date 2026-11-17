use super::super::tests::{pieces, token_of};
use super::*;
use crate::app::components::editor::language::Language::Ini;

#[test]
fn every_comment_mark_starts_a_comment() {
    for line in ["# hash", "; semicolon", "! bang", "// slashes", "   # indented"] {
        let coloured = pieces(Ini, line);
        assert_eq!(coloured.len(), 1);
        assert_eq!(coloured[0].1, Token::Comment, "{line}");
    }
}

#[test]
fn a_property_has_a_name_a_separator_and_a_value() {
    let line = pieces(Ini, "server-port=25565");
    assert_eq!(
        line,
        [
            ("server-port".to_string(), Token::Key),
            ("=".to_string(), Token::Punctuation),
            ("25565".to_string(), Token::Number),
        ]
    );
    assert_eq!(token_of(&pieces(Ini, "online-mode = false"), "false"), Some(Token::Constant));
    assert_eq!(token_of(&pieces(Ini, "motd=A Minecraft Server"), "A Minecraft Server"), Some(Token::Text));
    assert_eq!(pieces(Ini, "level-seed=").len(), 2, "an empty value has no span");
}

#[test]
fn the_games_options_use_a_colon() {
    let line = pieces(Ini, "renderDistance:12");
    assert_eq!(token_of(&line, "renderDistance"), Some(Token::Key));
    assert_eq!(token_of(&line, "12"), Some(Token::Number));
    let keys = pieces(Ini, "key_key.attack:key.mouse.left");
    assert_eq!(token_of(&keys, "key_key.attack"), Some(Token::Key));
    assert_eq!(token_of(&keys, "key.mouse.left"), Some(Token::Text));
}

#[test]
fn an_equals_sign_wins_over_a_colon_in_the_value() {
    let line = pieces(Ini, "resource-pack=https://example.com/pack.zip");
    assert_eq!(token_of(&line, "resource-pack"), Some(Token::Key));
    assert_eq!(token_of(&line, "https://example.com/pack.zip"), Some(Token::Text));
}

#[test]
fn sections_and_forge_categories_are_headings() {
    assert_eq!(pieces(Ini, "[display]"), [("[display]".to_string(), Token::Section)]);
    assert_eq!(pieces(Ini, "general {"), [("general {".to_string(), Token::Section)]);
    assert_eq!(pieces(Ini, "    \"world gen\" {"), [("\"world gen\" {".to_string(), Token::Section)]);
    assert_eq!(pieces(Ini, "}"), [("}".to_string(), Token::Punctuation)]);
}

#[test]
fn an_old_forge_config_marks_each_name_with_its_type() {
    let line = pieces(Ini, "    B:enableOres=true");
    assert_eq!(token_of(&line, "B:"), Some(Token::Attribute));
    assert_eq!(token_of(&line, "enableOres"), Some(Token::Key));
    assert_eq!(token_of(&line, "true"), Some(Token::Constant));
    let list = pieces(Ini, "S:blocks <");
    assert_eq!(
        list,
        [
            ("S:".to_string(), Token::Attribute),
            ("blocks".to_string(), Token::Key),
            ("<".to_string(), Token::Punctuation),
        ],
        "a list is opened without an equals sign"
    );
    let opened = pieces(Ini, "S:blocks=<");
    assert_eq!(token_of(&opened, "<"), Some(Token::Punctuation));
}

#[test]
fn a_line_with_no_separator_is_plain() {
    assert!(
        pieces(Ini, "minecraft:stone").len() == 3,
        "a colon is a separator when there is no equals sign"
    );
    assert!(pieces(Ini, "just words").is_empty());
    assert!(pieces(Ini, "   ").is_empty());
}
