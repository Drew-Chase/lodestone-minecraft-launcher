//! Tests for `pages/instance/logs.rs`, kept out of the source file. They are
//! compiled as the `tests` child module of `logs`, so they can read the buffer
//! and the follow flag directly.

use super::*;

fn line(message: &str) -> LogLine {
    LogLine {
        time: "12:00:00".to_string(),
        level: LogLevel::Info,
        message: message.to_string(),
    }
}

#[test]
fn the_buffer_keeps_the_newest_lines_in_order() {
    let mut buffer = LogBuffer::default();
    for index in 0..MAX_LINES + 10 {
        buffer.push(line(&index.to_string()));
    }

    assert_eq!(buffer.iter().count(), MAX_LINES);
    assert_eq!(
        buffer.iter().next().map(|line| line.message.as_str()),
        Some("10"),
        "the oldest ten are dropped"
    );
    assert_eq!(buffer.iter().last().map(|line| line.message.clone()), Some((MAX_LINES + 9).to_string()));
}

#[test]
fn from_lines_keeps_the_order_given() {
    let buffer = LogBuffer::from_lines([line("a"), line("b")]);
    let messages: Vec<_> = buffer.iter().map(|line| line.message.as_str()).collect();
    assert_eq!(messages, ["a", "b"]);
}

#[test]
fn levels_have_bracketed_tags() {
    assert_eq!(LogLevel::Info.tag(), "[INFO]");
    assert_eq!(LogLevel::Warn.tag(), "[WARN]");
    assert_eq!(LogLevel::Error.tag(), "[ERROR]");
    assert_eq!(LogLevel::Debug.tag(), "[DEBUG]");
}

#[test]
fn an_empty_log_has_no_lines_yet() {
    assert!(LogBuffer::default().iter().next().is_none());
    assert!(LogBuffer::from_lines([line("a")]).iter().next().is_some());
}

#[test]
fn the_live_pill_toggles_following_and_export_changes_nothing() {
    let mut state = State::default();
    assert!(state.following(), "a fresh log follows new lines");

    state.set_lines(vec![line("a"), line("b")]);
    state.update(Message::ToggleFollow);
    assert!(!state.following());

    let before = state.buffer.iter().count();
    state.update(Message::Export);
    assert_eq!(state.buffer.iter().count(), before);
}
