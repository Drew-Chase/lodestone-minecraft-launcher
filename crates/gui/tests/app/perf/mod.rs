use super::*;

use std::fs;
use std::time::Duration;

/// Starts the reporter and the trace in a scratch directory, records one scope, and checks that both
/// files show it. The reporter prints once a second, so this waits past the first report.
#[test]
fn the_reporter_and_the_trace_write_into_the_logs_directory() {
    let dir = std::env::temp_dir().join(format!("lodestone-perf-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("the scratch directory");

    init(&dir, || 7);
    {
        let _scope = scope("view.root");
        count("art.hit", 1);
    }
    std::thread::sleep(Duration::from_millis(1_500));

    let report = fs::read_to_string(dir.join("perf.log")).expect("the report file exists");
    assert!(report.contains("dropped 7 log lines"), "{report}");
    assert!(report.contains("view.root"), "{report}");

    let trace = fs::read_dir(&dir)
        .expect("the directory reads")
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .find(|path| path.file_name().is_some_and(|name| name.to_string_lossy().starts_with("trace-")))
        .expect("the trace file exists");
    let events = fs::read_to_string(trace).expect("the trace reads");
    assert!(events.contains("\"view.root\""), "{events}");
}
