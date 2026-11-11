//! The game's standard output, read into the rows the Logs tab shows.
//!
//! The game can write its log to standard output in log4j's XML layout: each event is a `<log4j:Event>` element over
//! several lines, and its message sits in a CDATA section. The Logs tab shows the rows latest.log holds instead,
//! `[HH:MM:SS] [thread/LEVEL]: message`, with the lines of a message or stack trace under them. [`OutputReader`] turns
//! each event into those rows when its closing tag arrives, so the console reads the same while the game runs as it
//! does once the game has exited.

use chrono::{DateTime, Local, Utc};

use crate::app::pages::instance::logs::{LogLevel, LogLine};

/// The most lines an event may run to. An event still open past this never closed, so its lines are shown as text.
const MAX_EVENT_LINES: usize = 1000;

const EVENT_OPEN: &str = "<log4j:Event";
const EVENT_CLOSE: &str = "</log4j:Event>";
const CDATA_OPEN: &str = "<![CDATA[";
const CDATA_CLOSE: &str = "]]>";
/// The entities [`unescape`] decodes.
const ENTITIES: [(&str, char); 5] = [("&amp;", '&'), ("&lt;", '<'), ("&gt;", '>'), ("&quot;", '"'), ("&apos;", '\'')];

/// Reads one game's standard output a line at a time, into the rows the Logs tab shows.
#[derive(Debug, Default)]
pub struct OutputReader {
    /// The event whose closing tag has not arrived yet.
    open: Option<Open>,
    /// The level of the last entry. A line with no prefix of its own continues that entry, as it does in latest.log.
    level: Option<LogLevel>,
}

impl OutputReader {
    /// Reads the next line of the game's output and returns the rows it completes. While an event is open this returns
    /// nothing: an event's rows come with the line that closes it.
    pub fn push(&mut self, line: String) -> Vec<LogLine> {
        if let Some(mut open) = self.open.take() {
            if open.in_cdata || !starts_event(&line) {
                open.add(line);
                if open.is_closed() {
                    return self.closed(open.lines);
                }
                if open.lines.len() < MAX_EVENT_LINES {
                    self.open = Some(open);
                    return Vec::new();
                }
                // Longer than any event, so it never closed: its lines are text.
                return self.plain_lines(open.lines);
            }
            // A new event starts, so the open one never closed and its lines are text.
            let mut rows = self.plain_lines(open.lines);
            rows.extend(self.start(line));
            return rows;
        }
        if starts_event(&line) {
            return self.start(line);
        }
        if line.trim().is_empty() {
            return Vec::new();
        }
        vec![self.plain(line)]
    }

    /// Starts an event with its first line, which is the whole event when it also closes there.
    fn start(&mut self, line: String) -> Vec<LogLine> {
        let mut open = Open::default();
        open.add(line);
        if open.is_closed() {
            return self.closed(open.lines);
        }
        self.open = Some(open);
        Vec::new()
    }

    /// The rows of a complete event. Lines that do not parse as one are shown as text.
    fn closed(&mut self, lines: Vec<String>) -> Vec<LogLine> {
        match Event::parse(&lines.join("\n")) {
            Some(event) => self.event_rows(event),
            None => self.plain_lines(lines),
        }
    }

    /// The rows of a parsed event. The first line of its message carries the time and level, and the rest of the message
    /// and any stack trace continue under it.
    fn event_rows(&mut self, event: Event) -> Vec<LogLine> {
        let level = event.level.unwrap_or(LogLevel::Info);
        self.level = Some(level);
        let time = event.timestamp.and_then(clock).unwrap_or_else(receipt_time);
        let mut message = event.message.lines();
        let mut rows = vec![LogLine {
            time,
            level,
            message: message.next().unwrap_or_default().to_string(),
        }];
        rows.extend(
            message
                .chain(event.throwable.lines())
                .filter(|text| !text.trim().is_empty())
                .map(|text| LogLine {
                    time: String::new(),
                    level,
                    message: text.to_string(),
                }),
        );
        rows
    }

    /// The row of a line that is not part of an event. A line with a prefix of its own starts an entry. Any other line
    /// continues the last entry, or, before the first entry, is shown at the time it arrived.
    fn plain(&mut self, line: String) -> LogLine {
        if let Some(row) = LogLine::from_prefixed(&line) {
            self.level = Some(row.level);
            return row;
        }
        match self.level {
            Some(level) => LogLine {
                time: String::new(),
                level,
                message: line,
            },
            None => LogLine {
                time: receipt_time(),
                level: LogLevel::Info,
                message: line,
            },
        }
    }

    /// The rows of lines read as text. Blank lines are dropped, as latest.log drops them.
    fn plain_lines(&mut self, lines: Vec<String>) -> Vec<LogLine> {
        lines
            .into_iter()
            .filter(|line| !line.trim().is_empty())
            .map(|line| self.plain(line))
            .collect()
    }
}

/// An event whose lines are still arriving.
#[derive(Debug, Default)]
struct Open {
    lines: Vec<String>,
    /// Whether a CDATA section is open after the last line. An event closes only outside one, so a message that quotes
    /// its own closing tag does not end early.
    in_cdata: bool,
}

impl Open {
    fn add(&mut self, line: String) {
        self.in_cdata = cdata_open_after(self.in_cdata, &line);
        self.lines.push(line);
    }

    /// Whether the last line closes the event, outside any CDATA section.
    fn is_closed(&self) -> bool {
        !self.in_cdata && self.lines.last().is_some_and(|last| last.trim_end().ends_with(EVENT_CLOSE))
    }
}

/// The parts of a complete event that its rows show.
struct Event {
    /// Milliseconds since the epoch, when the event has one.
    timestamp: Option<i64>,
    level: Option<LogLevel>,
    message: String,
    throwable: String,
}

impl Event {
    /// Reads a complete event from its lines joined with `\n`. `None` when its opening tag never ends.
    fn parse(text: &str) -> Option<Event> {
        let rest = text.trim_start().strip_prefix(EVENT_OPEN)?;
        let end = tag_end(rest)?;
        let pairs = attributes(&rest[..end]);
        let attribute = |wanted: &str| pairs.iter().find(|(name, _)| *name == wanted).map(|(_, value)| unescape(value));
        let body = &rest[end + 1..];
        Some(Event {
            timestamp: attribute("timestamp").and_then(|value| value.trim().parse().ok()),
            level: attribute("level").and_then(|value| LogLevel::from_tag(&value)),
            message: element(body, "log4j:Message").unwrap_or_default(),
            throwable: element(body, "log4j:Throwable").unwrap_or_default(),
        })
    }
}

/// Whether a CDATA section is open after `line`, given whether one was open before it. Inside a section only its end
/// counts, so text that looks like a start is part of the message.
fn cdata_open_after(mut open: bool, line: &str) -> bool {
    let mut rest = line;
    loop {
        if open {
            let Some(end) = rest.find(CDATA_CLOSE) else {
                return true;
            };
            open = false;
            rest = &rest[end + CDATA_CLOSE.len()..];
        } else {
            let Some(start) = rest.find(CDATA_OPEN) else {
                return false;
            };
            open = true;
            rest = &rest[start + CDATA_OPEN.len()..];
        }
    }
}

/// Whether a line opens an event.
fn starts_event(line: &str) -> bool {
    line.trim_start().starts_with(EVENT_OPEN)
}

/// The index of the `>` that closes a tag, skipping any inside a quoted attribute value.
fn tag_end(text: &str) -> Option<usize> {
    let mut quoted = false;
    for (index, byte) in text.bytes().enumerate() {
        match byte {
            b'"' => quoted = !quoted,
            b'>' if !quoted => return Some(index),
            _ => {}
        }
    }
    None
}

/// The `name="value"` pairs of a tag's attributes, with the values still escaped.
fn attributes(tag: &str) -> Vec<(&str, &str)> {
    let mut pairs = Vec::new();
    let mut rest = tag;
    while let Some(equals) = rest.find("=\"") {
        let name = rest[..equals].rsplit(char::is_whitespace).next().unwrap_or_default();
        let value = &rest[equals + 2..];
        let Some(close) = value.find('"') else {
            break;
        };
        pairs.push((name, &value[..close]));
        rest = &value[close + 1..];
    }
    pairs
}

/// The text of the first `<name>` element in `text`, or `None` when it is absent or never closes. A self-closing
/// element has no text.
fn element(text: &str, name: &str) -> Option<String> {
    let open = format!("<{name}");
    let after = &text[text.find(&open)? + open.len()..];
    let end = tag_end(after)?;
    if after[..end].ends_with('/') {
        return Some(String::new());
    }
    let content = &after[end + 1..];
    let close = format!("</{name}>");
    Some(text_of(&content[..content.find(&close)?]))
}

/// The text an element holds: its CDATA sections joined, or, without any, its content unescaped.
fn text_of(content: &str) -> String {
    if !content.contains(CDATA_OPEN) {
        return unescape(content);
    }
    let mut text = String::new();
    let mut rest = content;
    while let Some(start) = rest.find(CDATA_OPEN) {
        let section = &rest[start + CDATA_OPEN.len()..];
        let Some(end) = section.find(CDATA_CLOSE) else {
            break;
        };
        text.push_str(&section[..end]);
        rest = &section[end + CDATA_CLOSE.len()..];
    }
    text
}

/// Decodes the entities in `text`, each once: `&amp;lt;` becomes `&lt;`, not `<`.
fn unescape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        match ENTITIES.iter().find(|(entity, _)| rest.starts_with(*entity)) {
            Some((entity, decoded)) => {
                out.push(*decoded);
                rest = &rest[entity.len()..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// The local `HH:MM:SS` of a time in milliseconds since the epoch, or `None` when the time is out of range.
fn clock(millis: i64) -> Option<String> {
    let at = DateTime::<Utc>::from_timestamp_millis(millis)?;
    Some(at.with_timezone(&Local).format("%H:%M:%S").to_string())
}

/// The local `HH:MM:SS` now, for a line whose event has no usable time.
fn receipt_time() -> String {
    Local::now().format("%H:%M:%S").to_string()
}

#[cfg(test)]
#[path = "../../../tests/app/launch/output.rs"]
mod tests;
