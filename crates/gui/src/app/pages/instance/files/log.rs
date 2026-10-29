//! `logs/latest.log`, read from its end and parsed line by line.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use super::super::logs::{LogLevel, LogLine};

/// How far from the end of the file the read starts. Plenty for a long tail,
/// so a long session never reads the whole log.
const TAIL_BYTES: u64 = 256 * 1024;

/// The last `max_lines` lines of the log at `path`, oldest first. A missing or
/// unreadable log has no lines.
pub fn read_tail(path: &Path, max_lines: usize) -> Vec<LogLine> {
    let Ok(mut file) = File::open(path) else {
        return Vec::new();
    };
    let Ok(length) = file.metadata().map(|metadata| metadata.len()) else {
        return Vec::new();
    };
    let start = length.saturating_sub(TAIL_BYTES);
    if file.seek(SeekFrom::Start(start)).is_err() {
        return Vec::new();
    }
    let mut bytes = Vec::new();
    if file.read_to_end(&mut bytes).is_err() {
        return Vec::new();
    }

    let text = String::from_utf8_lossy(&bytes);
    // Reading began mid-line, so the first line is a fragment.
    let body: &str = if start > 0 {
        text.split_once('\n').map_or("", |(_, rest)| rest)
    } else {
        &text
    };
    let mut lines = parse(body);
    let excess = lines.len().saturating_sub(max_lines);
    lines.drain(..excess);
    lines
}

/// Parses `[HH:MM:SS] [thread/LEVEL]: message` lines. A line without that
/// prefix (a stack trace, say) becomes its own row under the level of the line
/// above it.
fn parse(text: &str) -> Vec<LogLine> {
    let mut lines: Vec<LogLine> = Vec::new();
    for raw in text.lines().filter(|raw| !raw.trim().is_empty()) {
        match parse_line(raw) {
            Some(line) => lines.push(line),
            None => {
                let level = lines.last().map_or(LogLevel::Info, |previous| previous.level);
                lines.push(LogLine {
                    time: String::new(),
                    level,
                    message: raw.to_string(),
                });
            }
        }
    }
    lines
}

/// One log line, or `None` when it has no `[time] [thread/LEVEL]` prefix.
fn parse_line(raw: &str) -> Option<LogLine> {
    let (time, rest) = raw.strip_prefix('[')?.split_once(']')?;
    if time.len() != 8 {
        return None;
    }
    let (tag, rest) = rest.trim_start().strip_prefix('[')?.split_once(']')?;
    let level = level_of(tag.rsplit_once('/')?.1)?;
    let message = rest.strip_prefix(':').unwrap_or(rest).trim_start();
    Some(LogLine {
        time: time.to_string(),
        level,
        message: message.to_string(),
    })
}

/// The level a log4j tag names. Anything unrecognised is not a level.
fn level_of(tag: &str) -> Option<LogLevel> {
    match tag {
        "DEBUG" | "TRACE" => Some(LogLevel::Debug),
        "INFO" => Some(LogLevel::Info),
        "WARN" => Some(LogLevel::Warn),
        "ERROR" | "FATAL" => Some(LogLevel::Error),
        _ => None,
    }
}

#[cfg(test)]
#[path = "../../../../../tests/app/pages/instance/files/log.rs"]
mod tests;
