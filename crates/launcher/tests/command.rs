//! Argument ordering, classpath separator, and `${...}` expansion, asserted
//! end-to-end through the per-OS stub (DESIGN.md §9).
//!
//! The pure, no-process assertions for `src/command.rs` live in
//! `#[cfg(test)] mod tests` inside that module, because it is `pub(crate)` and
//! therefore invisible to integration tests.

mod common;

use common::{StubJava, builder_for};
use lodestone_launch::{LaunchEvent, Launcher, LogStream};

/// The platform classpath separator.
const SEPARATOR: char = if cfg!(windows) { ';' } else { ':' };

/// Launches `launcher`, pumps its event stream to disconnection, and returns
/// (stdout lines, stderr lines, exit status).
fn run(launcher: &Launcher) -> (Vec<String>, Vec<String>, std::process::ExitStatus) {
    let mut process = launcher.launch().unwrap();
    let events = process.take_events();

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let mut status = None;
    while let Ok(event) = events.recv() {
        match event {
            LaunchEvent::Log {
                stream: LogStream::Stdout,
                line,
            } => stdout.push(line),
            LaunchEvent::Log {
                stream: LogStream::Stderr,
                line,
            } => stderr.push(line),
            LaunchEvent::Exited { status: collected } => status = Some(collected),
        }
    }
    let status = status.expect("Exited is emitted exactly once before disconnection");
    // `wait` is instant now, and confirms the handle's own status path agrees.
    assert_eq!(process.wait().unwrap(), status);
    (stdout, stderr, status)
}

/// Windows `echo %*` reproduces the quotes std put around arguments; strip them
/// so assertions see argument content rather than cmd's quoting.
fn normalize(line: &str) -> String {
    line.replace('"', "")
}

#[test]
fn argument_order_follows_the_contract() {
    let stub = StubJava::echo().unwrap();
    let launcher = builder_for(&stub)
        .jvm_args(["--jvm-marker"])
        .game_args(["--game-marker"])
        .build()
        .unwrap();

    let (stdout, _stderr, status) = run(&launcher);
    assert!(status.success());
    let line = normalize(&stdout[0]);

    let natives = format!("-Djava.library.path={}", stub.dir.join("natives").display());
    let classpath = format!("{}{SEPARATOR}{}", stub.dir.join("one.jar").display(), stub.dir.join("two.jar").display(),);
    let expected = ["--jvm-marker", &natives, "-cp", &classpath, "com.stub.Main", "--game-marker"].join(" ");
    assert_eq!(line, expected);
}

#[test]
fn environmental_placeholders_expand_to_configured_values() {
    let stub = StubJava::echo().unwrap();
    let launcher = builder_for(&stub)
        .game_args([
            "${classpath}",
            "${classpath_separator}",
            "${library_directory}",
            "${natives_directory}",
            "${assets_root}",
            "${launcher_name}",
            "${launcher_version}",
        ])
        .build()
        .unwrap();

    let (stdout, _stderr, _status) = run(&launcher);
    let line = normalize(&stdout[0]);

    let classpath = format!("{}{SEPARATOR}{}", stub.dir.join("one.jar").display(), stub.dir.join("two.jar").display());
    let expected = [
        format!("-Djava.library.path={}", stub.dir.join("natives").display()),
        "-cp".to_string(),
        classpath.clone(),
        "com.stub.Main".to_string(),
        classpath,
        SEPARATOR.to_string(),
        stub.dir.join("libraries").display().to_string(),
        stub.dir.join("natives").display().to_string(),
        stub.dir.join("assets").display().to_string(),
        "lodestone-test".to_string(),
        "0.0.0-test".to_string(),
    ]
    .join(" ");
    assert_eq!(line, expected);
}

#[test]
fn unknown_unset_and_substring_placeholders_pass_through_untouched() {
    let stub = StubJava::echo().unwrap();
    // Built directly (not via `builder_for`) so no natives or assets directory
    // is configured: those placeholders are known but unset, and must behave
    // like unknown tokens.
    let launcher = Launcher::builder(&stub.java_path, &stub.working_directory)
        .library_directory(stub.dir.join("libraries"))
        .main_class("com.stub.Main")
        .classpath([stub.dir.join("one.jar")])
        .game_args([
            "${not_a_placeholder}",
            "${natives_directory}",
            "${assets_root}",
            "prefix-${launcher_name}",
        ])
        .build()
        .unwrap();

    let (stdout, _stderr, _status) = run(&launcher);
    let line = normalize(&stdout[0]);

    let expected = [
        "-cp".to_string(),
        stub.dir.join("one.jar").display().to_string(),
        "com.stub.Main".to_string(),
        "${not_a_placeholder}".to_string(),
        "${natives_directory}".to_string(),
        "${assets_root}".to_string(),
        "prefix-${launcher_name}".to_string(),
    ]
    .join(" ");
    assert_eq!(line, expected);
}
