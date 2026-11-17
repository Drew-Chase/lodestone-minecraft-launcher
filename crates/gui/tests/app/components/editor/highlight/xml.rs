use super::super::tests::{pieces, read, token_of};
use super::*;
use crate::app::components::editor::language::Language::Xml;

#[test]
fn a_tag_has_a_name_attributes_and_quoted_values() {
    let line = pieces(Xml, r#"<Appender name="File" fileName='logs/latest.log'/>"#);
    assert_eq!(token_of(&line, "Appender"), Some(Token::Tag));
    assert_eq!(token_of(&line, "name"), Some(Token::Attribute));
    assert_eq!(token_of(&line, r#""File""#), Some(Token::Text));
    assert_eq!(token_of(&line, "'logs/latest.log'"), Some(Token::Text));
    assert_eq!(token_of(&line, "="), Some(Token::Punctuation));
    assert_eq!(token_of(&line, ">"), Some(Token::Punctuation));
}

#[test]
fn closing_tags_and_declarations_are_tags_and_text_between_tags_is_plain() {
    let line = pieces(Xml, "<level>info</level>");
    assert_eq!(line.iter().filter(|(text, token)| text == "level" && *token == Token::Tag).count(), 2);
    assert_eq!(token_of(&line, "info"), None);
    assert_eq!(token_of(&line, "</"), Some(Token::Punctuation));
    let declaration = pieces(Xml, r#"<?xml version="1.0"?>"#);
    assert_eq!(token_of(&declaration, "xml"), Some(Token::Tag));
    assert_eq!(token_of(&declaration, "version"), Some(Token::Attribute));
}

#[test]
fn a_comment_may_run_over_lines() {
    assert_eq!(pieces(Xml, "<!-- one line -->"), [("<!-- one line -->".to_string(), Token::Comment)]);
    let (_, carry) = read(Xml, Carry::None, "<a/> <!-- open");
    assert_eq!(carry, Carry::Comment);
    let (inside, carry) = read(Xml, carry, "<b attr=\"x\"/>");
    assert_eq!((inside.len(), carry), (1, Carry::Comment));
    let (last, carry) = read(Xml, carry, "done --> <c/>");
    assert_eq!(carry, Carry::None);
    assert_eq!(token_of(&last, "done -->"), Some(Token::Comment));
    assert_eq!(token_of(&last, "c"), Some(Token::Tag));
}

#[test]
fn a_tag_may_put_its_attributes_on_the_lines_below() {
    let (first, carry) = read(Xml, Carry::None, "<RollingFile name=\"a\"");
    assert_eq!(carry, Carry::Tag);
    assert_eq!(token_of(&first, "RollingFile"), Some(Token::Tag));
    let (second, carry) = read(Xml, carry, "    filePattern=\"logs/%d.log.gz\">");
    assert_eq!(carry, Carry::None);
    assert_eq!(token_of(&second, "filePattern"), Some(Token::Attribute));
    assert_eq!(token_of(&second, "\"logs/%d.log.gz\""), Some(Token::Text));
}

#[test]
fn a_greater_than_sign_inside_a_value_does_not_close_the_tag() {
    let line = pieces(Xml, r#"<Pattern value="%d > %m" />"#);
    assert_eq!(token_of(&line, r#""%d > %m""#), Some(Token::Text));
}
