//! Spawn / log events / exit / kill behavior via the stub (DESIGN.md §9).

mod common;

use std::time::Duration;

use common::{STUB_STARTED_LINE, STUB_STDERR_LINE, StubJava, builder_for};
use lodestone_launch::{LaunchEvent, LogStream};

#[test]
fn stdout_and_stderr_lines_arrive_as_log_events() {
    let stub = StubJava::echo().unwrap();
    let launcher = builder_for(&stub).game_args(["--echo-me"]).build().unwrap();
    let mut process = launcher.launch().unwrap();
    let events = process.take_events();

    let mut stdout_lines = Vec::new();
    let mut stderr_lines = Vec::new();
    while let Ok(event) = events.recv() {
        match event {
            LaunchEvent::Log {
                stream: LogStream::Stdout,
                line,
            } => stdout_lines.push(line),
            LaunchEvent::Log {
                stream: LogStream::Stderr,
                line,
            } => stderr_lines.push(line),
            LaunchEvent::Exited { .. } => {}
        }
    }

    let echoed = stdout_lines
        .iter()
        .any(|line| line.contains("--echo-me") && line.contains("com.stub.Main"));
    assert!(echoed, "expected the echoed argv, got {stdout_lines:?}");
    assert!(
        stderr_lines.iter().any(|line| line.trim_end() == STUB_STDERR_LINE),
        "expected the stub's stderr marker, got {stderr_lines:?}"
    );
}

#[test]
fn exit_produces_exactly_one_exited_event_last() {
    let stub = StubJava::echo().unwrap();
    let launcher = builder_for(&stub).build().unwrap();
    let mut process = launcher.launch().unwrap();
    let events = process.take_events();

    let mut collected = Vec::new();
    while let Ok(event) = events.recv() {
        collected.push(event);
    }

    let exited_count = collected.iter().filter(|event| matches!(event, LaunchEvent::Exited { .. })).count();
    assert_eq!(exited_count, 1);
    assert!(matches!(collected.last(), Some(LaunchEvent::Exited { .. })));
}

#[test]
fn try_wait_and_wait_report_the_exit_status() {
    let stub = StubJava::echo().unwrap();
    let launcher = builder_for(&stub).build().unwrap();
    let mut process = launcher.launch().unwrap();

    let mut exited = false;
    for _ in 0..500 {
        if process.try_wait().unwrap().is_some() {
            exited = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(exited, "the echo stub exits immediately");

    let status = process.wait().unwrap();
    assert!(status.success());
}

#[test]
fn kill_stops_a_long_running_game_and_the_stream_disconnects() {
    let stub = StubJava::sleeper().unwrap();
    let launcher = builder_for(&stub).build().unwrap();
    let mut process = launcher.launch().unwrap();
    let events = process.take_events();

    // Wait for the stub to announce itself so the kill lands on a live game.
    loop {
        match events.recv() {
            Ok(LaunchEvent::Log { line, .. }) if line.contains(STUB_STARTED_LINE) => break,
            Ok(_) => continue,
            Err(_) => panic!("the stub exited before announcing startup"),
        }
    }

    process.kill().expect("killing a live game succeeds");

    // The game's death must still flow through the event stream, exactly once,
    // and then the channel disconnects (loop exit).
    let mut exited_events = 0;
    while let Ok(event) = events.recv() {
        if matches!(event, LaunchEvent::Exited { .. }) {
            exited_events += 1;
        }
    }
    assert_eq!(exited_events, 1);

    let status = process.wait().expect("wait after kill returns the status");
    assert!(!status.success(), "a killed game does not exit cleanly");
}

#[test]
#[should_panic(expected = "already taken")]
fn taking_events_twice_panics() {
    let stub = StubJava::echo().unwrap();
    let launcher = builder_for(&stub).build().unwrap();
    let mut process = launcher.launch().unwrap();
    let _first = process.take_events();
    let _second = process.take_events();
}
