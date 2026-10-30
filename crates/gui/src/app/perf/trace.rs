//! The Chrome trace: timed scopes and iced events on one timeline, written as the JSON that Perfetto
//! (https://ui.perfetto.dev) opens directly. Events queue for a writer thread, so recording never waits
//! on the disk. An event that finds the queue full is dropped and counted.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender, TrySendError};
use std::thread;
use std::time::{Duration, Instant};

/// Events the queue holds before new ones are dropped.
const QUEUE_CAPACITY: usize = 65_536;

/// Events written before the trace stops, which keeps a long session's file near 100 MB.
const MAX_EVENTS: u64 = 1_000_000;

/// How often the writer flushes while events are quiet.
const FLUSH_INTERVAL: Duration = Duration::from_millis(250);

/// One record on the timeline.
#[derive(Debug, PartialEq, Eq)]
enum Event {
    /// A scope that began `start_us` after the trace started and lasted `dur_us`.
    Span {
        name: &'static str,
        start_us: u64,
        dur_us: u64,
        tid: u64,
    },
    /// A moment, with a number to show with it.
    Instant { name: &'static str, at_us: u64, tid: u64, value: u64 },
}

/// The queue into the writer thread, and the instant the timeline starts from.
struct Trace {
    queue: SyncSender<Event>,
    epoch: Instant,
}

static TRACE: OnceLock<Trace> = OnceLock::new();
static DROPPED: AtomicU64 = AtomicU64::new(0);
static NEXT_THREAD: AtomicU64 = AtomicU64::new(1);

thread_local! {
    /// A small id for this thread, so each thread gets its own row on the timeline.
    static THREAD: u64 = NEXT_THREAD.fetch_add(1, Ordering::Relaxed);
}

/// Opens `trace-<stamp>.json` in `logs_dir` and starts the writer thread.
pub fn start(logs_dir: &Path) {
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let file = match File::create(logs_dir.join(format!("trace-{stamp}.json"))) {
        Ok(file) => file,
        Err(error) => {
            eprintln!("perf trace unavailable: {error}");
            return;
        }
    };
    let (queue, events) = mpsc::sync_channel(QUEUE_CAPACITY);
    let spawned = thread::Builder::new()
        .name("perf-trace".into())
        .spawn(move || write_events(events, BufWriter::new(file)));
    if let Err(error) = spawned {
        eprintln!("perf trace unavailable: {error}");
        return;
    }
    let _ = TRACE.set(Trace {
        queue,
        epoch: Instant::now(),
    });
}

/// Records a scope that began at `start` and lasted `elapsed`. Does nothing before [`start`].
pub fn span(name: &'static str, start: Instant, elapsed: Duration) {
    let Some(trace) = TRACE.get() else { return };
    send(
        trace,
        Event::Span {
            name,
            start_us: micros(start.saturating_duration_since(trace.epoch)),
            dur_us: micros(elapsed),
            tid: THREAD.with(|tid| *tid),
        },
    );
}

/// Records a moment, with a number to show with it. Does nothing before [`start`].
pub fn instant(name: &'static str, value: u64) {
    let Some(trace) = TRACE.get() else { return };
    send(
        trace,
        Event::Instant {
            name,
            at_us: micros(Instant::now().saturating_duration_since(trace.epoch)),
            tid: THREAD.with(|tid| *tid),
            value,
        },
    );
}

/// Trace events dropped because the queue was full.
pub fn dropped() -> u64 {
    DROPPED.load(Ordering::Relaxed)
}

fn send(trace: &Trace, event: Event) {
    if let Err(TrySendError::Full(_)) = trace.queue.try_send(event) {
        DROPPED.fetch_add(1, Ordering::Relaxed);
    }
}

fn micros(duration: Duration) -> u64 {
    u64::try_from(duration.as_micros()).unwrap_or(u64::MAX)
}

/// Writes the events as one JSON array, and closes it once every sender is gone. A trace cut short
/// by a crash is still an array Perfetto reads, since it does not need the closing bracket.
fn write_events(events: Receiver<Event>, mut out: BufWriter<File>) {
    let _ = out.write_all(b"[\n");
    let mut written = 0u64;
    loop {
        match events.recv_timeout(FLUSH_INTERVAL) {
            Ok(event) => {
                if written == MAX_EVENTS {
                    continue;
                }
                if written > 0 {
                    let _ = out.write_all(b",\n");
                }
                let _ = out.write_all(encode(&event).as_bytes());
                written += 1;
            }
            Err(RecvTimeoutError::Timeout) => {
                let _ = out.flush();
            }
            Err(RecvTimeoutError::Disconnected) => {
                let _ = out.write_all(b"\n]\n");
                let _ = out.flush();
                return;
            }
        }
    }
}

/// One event as a Chrome trace event object.
fn encode(event: &Event) -> String {
    match event {
        Event::Span { name, start_us, dur_us, tid } => format!(
            r#"{{"name":{},"cat":"perf","ph":"X","ts":{start_us},"dur":{dur_us},"pid":1,"tid":{tid}}}"#,
            json_string(name)
        ),
        Event::Instant { name, at_us, tid, value } => format!(
            r#"{{"name":{},"cat":"iced","ph":"i","s":"t","ts":{at_us},"pid":1,"tid":{tid},"args":{{"value":{value}}}}}"#,
            json_string(name)
        ),
    }
}

fn json_string(text: &str) -> String {
    serde_json::to_string(text).unwrap_or_else(|_| "\"\"".to_owned())
}

#[cfg(test)]
#[path = "../../../tests/app/perf/trace.rs"]
mod tests;
