//! The writer thread: lines reach the files in order, and a full queue drops and counts them.

use super::*;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lodestone-logging-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("the scratch directory");
    dir
}

#[test]
fn the_writer_keeps_lines_in_order_and_flushes_on_request() {
    let dir = scratch("order");
    let logs = RollingLogs::open(dir.clone()).expect("the log files open");
    let (queue, commands) = mpsc::sync_channel(QUEUE_CAPACITY);
    let writer = thread::spawn(move || run_writer(commands, logs));

    let mut sink = LogWriter::new(&queue, target::DEBUG);
    for line in 0..50 {
        sink.write_all(format!("line {line}\n").as_bytes()).expect("the write queues");
    }
    wait_for_queue(&queue, Duration::from_secs(5));

    let text = fs::read_to_string(dir.join("debug.log")).expect("the debug log reads");
    let expected: String = (0..50).map(|line| format!("line {line}\n")).collect();
    assert_eq!(text, expected);

    drop(sink);
    drop(queue);
    writer.join().expect("the writer exits once every sender is gone");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn an_error_line_is_on_disk_once_its_write_returns() {
    let dir = scratch("error");
    let logs = RollingLogs::open(dir.clone()).expect("the log files open");
    let (queue, commands) = mpsc::sync_channel(QUEUE_CAPACITY);
    let writer = thread::spawn(move || run_writer(commands, logs));

    let mut sink = LogWriter::new(&queue, target::ERROR);
    sink.write_all(b"the launcher failed\n").expect("the write queues");
    // No flush was asked for: the error path waits for the writer itself.
    let text = fs::read_to_string(dir.join("error.log")).expect("the error log reads");
    assert_eq!(text, "the launcher failed\n");

    drop(sink);
    drop(queue);
    writer.join().expect("the writer exits once every sender is gone");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_full_queue_drops_and_counts_lines() {
    // Nothing drains this queue, so it is full after two lines.
    let (queue, _held) = mpsc::sync_channel(2);
    let mut sink = LogWriter::new(&queue, target::DEBUG);
    let before = dropped_lines();
    for _ in 0..5 {
        sink.write_all(b"line\n").expect("the write never fails");
    }
    assert_eq!(dropped_lines() - before, 3);
}
