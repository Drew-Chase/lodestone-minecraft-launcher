use super::super::tests::{pieces, token_of};
use super::*;
use crate::app::components::editor::language::Language::Crash;

#[test]
fn the_banner_and_the_part_headings_are_sections() {
    assert_eq!(pieces(Crash, "---- Minecraft Crash Report ----")[0].1, Token::Section);
    assert_eq!(pieces(Crash, "-- System Details --")[0].1, Token::Section);
    assert_eq!(pieces(Crash, "-- Head --")[0].1, Token::Section);
}

#[test]
fn the_witty_comment_is_a_comment() {
    assert_eq!(
        pieces(Crash, "// Why did you do that?"),
        [("// Why did you do that?".to_string(), Token::Comment)]
    );
}

#[test]
fn details_have_their_name_marked() {
    let line = pieces(Crash, "Time: 2026-01-02 03:04:05");
    assert_eq!(line, [("Time".to_string(), Token::Key), (":".to_string(), Token::Punctuation)]);
    let indented = pieces(Crash, "\tMinecraft Version: 1.20.1");
    assert_eq!(token_of(&indented, "Minecraft Version"), Some(Token::Key));
    assert_eq!(token_of(&pieces(Crash, "Stacktrace:"), "Stacktrace"), Some(Token::Key));
    assert_eq!(
        token_of(
            &pieces(Crash, "\tJava VM Version: OpenJDK 64-Bit Server VM (mixed mode), Eclipse"),
            "Java VM Version"
        ),
        Some(Token::Key)
    );
}

#[test]
fn the_thrown_exception_is_an_error() {
    assert_eq!(
        pieces(Crash, "java.lang.NullPointerException: Cannot invoke \"Foo.bar()\"")[0].1,
        Token::Error
    );
    assert_eq!(pieces(Crash, "Caused by: java.lang.IllegalArgumentException: bad")[0].1, Token::Error);
}

#[test]
fn a_stack_frame_is_faint_with_its_file_and_line_marked() {
    let line = pieces(Crash, "\tat net.minecraft.client.Minecraft.run(Minecraft.java:718) ~[client.jar:?]");
    assert_eq!(
        line,
        [
            ("at net.minecraft.client.Minecraft.run(".to_string(), Token::Faint),
            ("Minecraft.java:718".to_string(), Token::Tag),
            (") ~[client.jar:?]".to_string(), Token::Faint),
        ],
        "the jar the frame came from stays faint"
    );
    let simple = pieces(Crash, "\tat net.minecraft.Foo.bar(Foo.java:12)");
    assert_eq!(token_of(&simple, "Foo.java:12"), Some(Token::Tag));
    assert_eq!(token_of(&simple, "at net.minecraft.Foo.bar("), Some(Token::Faint));
    assert_eq!(pieces(Crash, "\t... 5 more")[0].1, Token::Faint);
}

#[test]
fn prose_and_table_rows_are_plain() {
    assert!(
        pieces(
            Crash,
            "A detailed walkthrough of the error, its code path and all known details is as follows:"
        )
        .is_empty()
    );
    assert!(pieces(Crash, "\t\t| State | ID | Version |").is_empty());
    assert!(pieces(Crash, "").is_empty());
}
