use super::super::tests::{pieces, read, token_of};
use super::*;
use crate::app::components::editor::language::Language::Log;

#[test]
fn an_entry_has_a_faint_time_and_its_level_in_the_levels_colour() {
    let line = pieces(Log, "[12:00:01] [main/INFO]: Loading the world");
    assert_eq!(line[0], ("[12:00:01] ".to_string(), Token::Faint));
    assert_eq!(line[1], ("[main/INFO]".to_string(), Token::Info));
    assert_eq!(line.len(), 2, "the message of an info line keeps the plain ink");
}

#[test]
fn a_warning_and_an_error_colour_their_message_too() {
    let warn = pieces(Log, "[12:00:02] [Render thread/WARN]: Slow frame");
    assert_eq!(token_of(&warn, "[Render thread/WARN]"), Some(Token::Warning));
    assert_eq!(token_of(&warn, ": Slow frame"), Some(Token::Warning));
    let error = pieces(Log, "[12:00:03] [main/ERROR]: Failed to load");
    assert_eq!(token_of(&error, ": Failed to load"), Some(Token::Error));
    assert_eq!(token_of(&pieces(Log, "[main/FATAL]: gone"), "[main/FATAL]"), Some(Token::Error));
    assert_eq!(
        token_of(&pieces(Log, "[12:00:04] [main/DEBUG]: detail"), "[main/DEBUG]"),
        Some(Token::Debug)
    );
}

#[test]
fn other_log_shapes_are_read_by_their_level_word() {
    let launcher = pieces(Log, "2026-01-02T03:04:05Z  WARN lodestone: slow disk");
    assert_eq!(token_of(&launcher, "WARN"), Some(Token::Warning));
    let bracketed = pieces(Log, "[INFO] Starting");
    assert_eq!(token_of(&bracketed, "[INFO]"), Some(Token::Info));
}

#[test]
fn a_level_word_inside_a_longer_word_or_deep_in_the_message_is_not_the_level() {
    let line = pieces(Log, "[12:00:01] [main/INFO]: ERROR handler registered");
    assert_eq!(
        token_of(&line, "[main/INFO]"),
        Some(Token::Info),
        "the first level on the line is the entry's"
    );
    assert!(pieces(Log, "INFORMATION about nothing").is_empty());
}

#[test]
fn the_lines_under_an_error_share_its_colour_and_stack_frames_are_faint() {
    let (_, carry) = read(Log, Carry::None, "[12:00:03] [main/ERROR]: Failed to load");
    assert_eq!(carry, Carry::Level(Level::Error));
    let (cause, carry) = read(Log, carry, "java.lang.IllegalStateException: Bad state");
    assert_eq!(cause[0].1, Token::Error);
    let (frame, carry) = read(Log, carry, "\tat net.minecraft.Foo.bar(Foo.java:12)");
    assert_eq!(frame, [("at net.minecraft.Foo.bar(Foo.java:12)".to_string(), Token::Faint)]);
    let (more, carry) = read(Log, carry, "\t... 12 more");
    assert_eq!(more[0].1, Token::Faint);
    let (note, _) = read(Log, carry, "  a note with no level of its own");
    assert_eq!(note[0].1, Token::Error, "it belongs to the error above");
}

#[test]
fn a_line_under_an_info_entry_is_plain_unless_it_names_an_exception() {
    let carry = Carry::Level(Level::Info);
    assert!(read(Log, carry, "  continued").0.is_empty());
    assert_eq!(read(Log, carry, "Caused by: java.io.IOException: gone").0[0].1, Token::Error);
    assert_eq!(read(Log, carry, "").1, carry, "an empty line keeps the entry's level");
}

#[test]
fn an_exception_line_is_a_dotted_class_that_ends_as_one() {
    assert!(is_exception("java.lang.NullPointerException"));
    assert!(is_exception("java.lang.OutOfMemoryError: Java heap space"));
    assert!(is_exception("Exception in thread \"main\" java.lang.Error"));
    assert!(!is_exception("Exception handling is fine"));
    assert!(!is_exception("NoError here"));
}
