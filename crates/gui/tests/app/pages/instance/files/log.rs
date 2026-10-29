//! Tests for `pages/instance/files/log.rs`, kept out of the source file. They are
//! compiled as the `tests` child module of `log`.

use std::fs;
use std::path::{Path, PathBuf};

use super::*;

/// A fresh folder under the system temp directory, named for the test.
fn scratch(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("lodestone-log-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).expect("scratch folder");
    path
}

/// A log with every shape the parser meets: plain levels, a thread with
/// a second tag, a stack trace, an unknown level and a bare line.
const LOG: &str = "[18:42:03] [main/INFO]: Loading Fabric Loader 0.15.7
[18:42:04] [Render thread/WARN]: Mod `create` registered legacy recipe handler
java.lang.IllegalStateException: boom
\tat com.example.Foo.bar(Foo.java:1)
[18:42:05] [Render thread/ERROR] [minecraft/Crash]: Failed to load
[18:42:06] [main/DEBUG]: tick profiler armed
not a log line
";

#[test]
fn lines_parse_into_time_level_and_message() {
    let lines = parse(LOG);
    let parsed: Vec<(&str, LogLevel, &str)> = lines.iter().map(|line| (line.time.as_str(), line.level, line.message.as_str())).collect();
    assert_eq!(
        parsed,
        [
            ("18:42:03", LogLevel::Info, "Loading Fabric Loader 0.15.7"),
            ("18:42:04", LogLevel::Warn, "Mod `create` registered legacy recipe handler"),
            ("", LogLevel::Warn, "java.lang.IllegalStateException: boom"),
            ("", LogLevel::Warn, "\tat com.example.Foo.bar(Foo.java:1)"),
            ("18:42:05", LogLevel::Error, "[minecraft/Crash]: Failed to load"),
            ("18:42:06", LogLevel::Debug, "tick profiler armed"),
            ("", LogLevel::Debug, "not a log line"),
        ],
        "a line without the prefix continues the level of the line above it"
    );
}

#[test]
fn an_unknown_level_is_not_a_log_line() {
    let lines = parse("[18:42:03] [main/NOTICE]: something\n");
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].time, "", "the line is kept, but without a time");
    assert_eq!(lines[0].level, LogLevel::Info, "with no line above it, the level is Info");
}

#[test]
fn blank_lines_are_skipped() {
    let lines = parse("\n\n[18:42:03] [main/INFO]: a\n\n");
    assert_eq!(lines.len(), 1);
}

#[test]
fn the_tail_keeps_the_newest_lines_and_starts_on_a_whole_line() {
    let root = scratch("tail");
    let mut text = String::new();
    for index in 0..20_000 {
        text.push_str(&format!("[12:00:00] [main/INFO]: line {index}\n"));
    }
    let path = root.join("latest.log");
    fs::write(&path, &text).expect("log writes");

    // The file is far larger than the read, so the read begins mid-line.
    let lines = read_tail(&path, 10);
    assert_eq!(lines.len(), 10);
    assert!(
        lines.iter().all(|line| line.time == "12:00:00"),
        "the fragment the read begins with is dropped"
    );
    assert_eq!(lines.first().map(|line| line.message.as_str()), Some("line 19990"));
    assert_eq!(lines.last().map(|line| line.message.as_str()), Some("line 19999"));
    fs::remove_dir_all(&root).ok();
}

#[test]
fn a_missing_log_has_no_lines() {
    assert!(read_tail(Path::new("/definitely/not/a/latest.log"), 10).is_empty());
}
