//! Tests for `launch/output.rs`, kept out of the source file. They are compiled as the `tests` child module of `output`.

use chrono::{DateTime, Local, Utc};

use super::*;

/// The local `HH:MM:SS` a time in milliseconds since the epoch shows as.
fn local(millis: i64) -> String {
    DateTime::<Utc>::from_timestamp_millis(millis)
        .expect("a valid timestamp")
        .with_timezone(&Local)
        .format("%H:%M:%S")
        .to_string()
}

/// One row, as the Logs tab holds it.
fn row(time: &str, level: LogLevel, message: &str) -> LogLine {
    LogLine {
        time: time.to_string(),
        level,
        message: message.to_string(),
    }
}

/// Feeds every line through one reader, as the game's output arrives, and returns the rows they complete.
fn read(lines: &[&str]) -> Vec<LogLine> {
    let mut reader = OutputReader::default();
    lines.iter().flat_map(|line| reader.push(line.to_string())).collect()
}

/// A warning from the shader loader, as the game printed it, over the three lines an event takes.
const WARN_EVENT: [&str; 3] = [
    r#"<log4j:Event logger="net.minecraft.class_5944" timestamp="1794415973585" level="WARN" thread="Render thread">"#,
    r#"  <log4j:Message><![CDATA[Shader position_tex_blur_horizontal could not find uniform named ColorModulator in the specified shader program.]]></log4j:Message>"#,
    "</log4j:Event>",
];

#[test]
fn a_closed_event_is_one_row_with_its_time_and_level() {
    assert_eq!(
        read(&WARN_EVENT),
        [row(
            &local(1794415973585),
            LogLevel::Warn,
            "Shader position_tex_blur_horizontal could not find uniform named ColorModulator in the specified shader program.",
        )]
    );
}

#[test]
fn nothing_is_shown_until_the_event_closes() {
    let mut reader = OutputReader::default();
    assert!(reader.push(WARN_EVENT[0].to_string()).is_empty());
    assert!(reader.push(WARN_EVENT[1].to_string()).is_empty());
    assert_eq!(reader.push(WARN_EVENT[2].to_string()).len(), 1);
}

#[test]
fn events_read_in_the_order_they_arrive() {
    let rows = read(&[
        WARN_EVENT[0],
        WARN_EVENT[1],
        WARN_EVENT[2],
        r#"<log4j:Event logger="net.minecraft.class_1059" timestamp="1794415973598" level="INFO" thread="Render thread">"#,
        r#"  <log4j:Message><![CDATA[Created: 512x256x0 minecraft:textures/atlas/paintings.png-atlas]]></log4j:Message>"#,
        "</log4j:Event>",
        r#"<log4j:Event logger="ModernFix" timestamp="1794415976654" level="WARN" thread="Render thread">"#,
        r#"  <log4j:Message><![CDATA[Game took 14.576 seconds to start]]></log4j:Message>"#,
        "</log4j:Event>",
    ]);
    let levels: Vec<LogLevel> = rows.iter().map(|row| row.level).collect();
    let messages: Vec<&str> = rows.iter().map(|row| row.message.as_str()).collect();
    assert_eq!(levels, [LogLevel::Warn, LogLevel::Info, LogLevel::Warn]);
    assert_eq!(
        messages,
        [
            "Shader position_tex_blur_horizontal could not find uniform named ColorModulator in the specified shader program.",
            "Created: 512x256x0 minecraft:textures/atlas/paintings.png-atlas",
            "Game took 14.576 seconds to start",
        ]
    );
    assert_eq!(rows[1].time, local(1794415973598), "each event shows its own time");
}

#[test]
fn a_message_over_several_lines_continues_under_its_first_line() {
    let rows = read(&[
        r#"<log4j:Event logger="a" timestamp="1794415973585" level="ERROR" thread="main">"#,
        "  <log4j:Message><![CDATA[Could not read the config:",
        "  line 3 is not a number",
        "]]></log4j:Message>",
        "</log4j:Event>",
    ]);
    assert_eq!(
        rows,
        [
            row(&local(1794415973585), LogLevel::Error, "Could not read the config:"),
            row("", LogLevel::Error, "  line 3 is not a number"),
        ]
    );
}

#[test]
fn a_stack_trace_continues_under_the_event_level() {
    let rows = read(&[
        r#"<log4j:Event logger="a" timestamp="1794415973585" level="ERROR" thread="main">"#,
        "  <log4j:Message><![CDATA[Crash]]></log4j:Message>",
        "  <log4j:Throwable><![CDATA[java.lang.IllegalStateException: boom",
        "\tat com.example.Foo.bar(Foo.java:1)",
        "]]></log4j:Throwable>",
        "</log4j:Event>",
    ]);
    assert_eq!(
        rows,
        [
            row(&local(1794415973585), LogLevel::Error, "Crash"),
            row("", LogLevel::Error, "java.lang.IllegalStateException: boom"),
            row("", LogLevel::Error, "\tat com.example.Foo.bar(Foo.java:1)"),
        ]
    );
}

#[test]
fn an_escaped_cdata_end_in_a_message_is_restored() {
    let rows = read(&[
        r#"<log4j:Event logger="a" timestamp="1794415973585" level="INFO" thread="main"><log4j:Message><![CDATA[a]]]]><![CDATA[>b]]></log4j:Message></log4j:Event>"#,
    ]);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].message, "a]]>b");
}

#[test]
fn a_closing_tag_inside_a_message_does_not_end_the_event() {
    let rows = read(&[
        r#"<log4j:Event logger="a" timestamp="1794415973585" level="INFO" thread="main">"#,
        "  <log4j:Message><![CDATA[Quoted:",
        "</log4j:Event>",
        "end]]></log4j:Message>",
        "</log4j:Event>",
    ]);
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].message, "Quoted:");
    assert_eq!(rows[1].message, "</log4j:Event>");
    assert_eq!(rows[2].message, "end");
}

#[test]
fn an_event_on_one_line_is_read() {
    let rows = read(&[
        r#"<log4j:Event logger="a" timestamp="1794415973585" level="DEBUG" thread="main"><log4j:Message><![CDATA[one line]]></log4j:Message></log4j:Event>"#,
    ]);
    assert_eq!(rows, [row(&local(1794415973585), LogLevel::Debug, "one line")]);
}

#[test]
fn a_new_event_ends_an_open_one_and_its_lines_are_shown_as_text() {
    let rows = read(&[
        WARN_EVENT[0],
        WARN_EVENT[1],
        r#"<log4j:Event logger="a" timestamp="1794415973598" level="INFO" thread="main">"#,
        r#"  <log4j:Message><![CDATA[second]]></log4j:Message>"#,
        "</log4j:Event>",
    ]);
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].message, WARN_EVENT[0]);
    assert_eq!(rows[1].message, WARN_EVENT[1]);
    assert_eq!(rows[2], row(&local(1794415973598), LogLevel::Info, "second"));
}

#[test]
fn an_event_whose_opening_tag_never_ends_is_shown_as_text() {
    let line = r#"<log4j:Event logger="a></log4j:Event>"#;
    let rows = read(&[line]);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].message, line);
}

#[test]
fn an_event_that_never_closes_is_shown_as_text_once_it_runs_long() {
    let mut reader = OutputReader::default();
    assert!(reader.push(WARN_EVENT[0].to_string()).is_empty());
    let mut rows = Vec::new();
    for index in 0..MAX_EVENT_LINES {
        rows.extend(reader.push(format!("  filler {index}")));
    }
    assert_eq!(rows.len(), MAX_EVENT_LINES + 1, "every line comes back, once");
    assert_eq!(rows[0].message, WARN_EVENT[0]);
}

#[test]
fn plain_lines_keep_their_own_prefix_or_continue_the_entry_above() {
    let rows = read(&["[12:00:00] [main/WARN]: pattern line", "\tat a.b(C.java:1)", "plain"]);
    assert_eq!(
        rows,
        [
            row("12:00:00", LogLevel::Warn, "pattern line"),
            row("", LogLevel::Warn, "\tat a.b(C.java:1)"),
            row("", LogLevel::Warn, "plain"),
        ]
    );
}

#[test]
fn a_line_before_any_entry_is_shown_at_the_time_it_arrived() {
    let rows = read(&["JVM warning: something"]);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].level, LogLevel::Info);
    assert_eq!(rows[0].time.len(), 8, "the time it arrived, as HH:MM:SS");
}

#[test]
fn blank_lines_are_skipped() {
    assert!(read(&["", "   "]).is_empty());
}

#[test]
fn log4j_levels_map_to_the_console_levels() {
    let cases = [
        ("TRACE", LogLevel::Debug),
        ("DEBUG", LogLevel::Debug),
        ("INFO", LogLevel::Info),
        ("WARN", LogLevel::Warn),
        ("ERROR", LogLevel::Error),
        ("FATAL", LogLevel::Error),
        ("NOTICE", LogLevel::Info),
    ];
    for (tag, expected) in cases {
        let event = format!(
            r#"<log4j:Event logger="a" timestamp="1794415973585" level="{tag}" thread="main"><log4j:Message><![CDATA[x]]></log4j:Message></log4j:Event>"#
        );
        assert_eq!(read(&[event.as_str()])[0].level, expected, "level {tag}");
    }
}

#[test]
fn escaped_text_outside_a_cdata_section_is_unescaped_once() {
    let rows = read(&[
        r#"<log4j:Event logger="a" timestamp="1794415973585" level="INFO" thread="main"><log4j:Message>a &lt;b&gt; &amp;lt; &quot;c&quot;</log4j:Message></log4j:Event>"#,
    ]);
    assert_eq!(rows[0].message, r#"a <b> &lt; "c""#);
}

#[test]
fn an_event_without_a_timestamp_is_shown_at_the_time_it_arrived() {
    let rows = read(&[r#"<log4j:Event logger="a" level="WARN" thread="main"><log4j:Message><![CDATA[x]]></log4j:Message></log4j:Event>"#]);
    assert_eq!(rows[0].level, LogLevel::Warn);
    assert_eq!(rows[0].time.len(), 8, "the time it arrived, as HH:MM:SS");
}
