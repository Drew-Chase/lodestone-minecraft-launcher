use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::panic;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender, TrySendError};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use chrono::{Local, Timelike};
use lodestone_launcher::perf;
use tar::Builder;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::{Layer, fmt, prelude::*};
use xz2::stream::{Check, Stream};
use xz2::write::XzEncoder;

/// Lines the writer thread may hold before new ones are dropped and counted.
const QUEUE_CAPACITY: usize = 16_384;

/// How often the writer flushes when no line has arrived.
const FLUSH_INTERVAL: Duration = Duration::from_millis(250);

/// The longest a caller waits for the writer to drain the queue (errors and panics).
const WAIT_LIMIT: Duration = Duration::from_secs(2);

/// Roll a log file once it grows past this many bytes.
const MAX_BYTES: u64 = 1024 * 1024 * 1024; // 1 GB

/// `liblzma` extreme preset flag (`LZMA_PRESET_EXTREME`). OR'd with the
/// numeric preset (9) this yields 7-zip "Ultra" LZMA2 compression.
const LZMA_PRESET_EXTREME: u32 = 1 << 31;

/// Log file names, ordered to match the [`Target`] indices below.
const LOG_FILES: [&str; 4] = ["debug.log", "latest.log", "error.log", "logs.ndjson"];

/// Index into [`RollingLogs`] file/size arrays. Order matches [`LOG_FILES`].
mod target {
    pub const DEBUG: usize = 0;
    pub const LATEST: usize = 1;
    pub const ERROR: usize = 2;
    pub const JSON: usize = 3;
    /// Console output, which goes to stdout rather than a file.
    pub const CONSOLE: usize = 4;
}

/// Shared, mutex-guarded state backing every log layer.
///
/// All four destination files share one lock so that a size-triggered roll
/// archives them together into a single snapshot.
struct RollingLogs {
    dir: PathBuf,
    files: [File; 4],
    sizes: [u64; 4],
}

impl RollingLogs {
    /// Open (truncating) all four log files in `dir`, ready for a fresh launch.
    fn open(dir: PathBuf) -> io::Result<Self> {
        let files = [
            fresh_file(&dir, LOG_FILES[0])?,
            fresh_file(&dir, LOG_FILES[1])?,
            fresh_file(&dir, LOG_FILES[2])?,
            fresh_file(&dir, LOG_FILES[3])?,
        ];
        Ok(Self { dir, files, sizes: [0; 4] })
    }

    /// Append `buf` to one destination file, rolling everything if it crosses
    /// the size threshold afterwards.
    fn write_to(&mut self, target: usize, buf: &[u8]) -> io::Result<()> {
        self.files[target].write_all(buf)?;
        self.sizes[target] += buf.len() as u64;
        if self.sizes[target] >= MAX_BYTES {
            self.roll()?;
        }
        Ok(())
    }

    /// Archive the current log files, then truncate them in place so the same
    /// handles keep working (avoids Windows sharing violations from reopening).
    fn roll(&mut self) -> io::Result<()> {
        for file in &mut self.files {
            file.flush()?;
        }
        archive(&self.dir)?;
        for (file, size) in self.files.iter_mut().zip(self.sizes.iter_mut()) {
            file.set_len(0)?;
            file.seek(SeekFrom::Start(0))?;
            *size = 0;
        }
        Ok(())
    }
}

/// Lines the logging path could not queue because the writer thread was behind. Counted rather than
/// logged, so a flood of output cannot feed itself.
static DROPPED_LINES: AtomicU64 = AtomicU64::new(0);

/// The queue into the writer thread, kept so [`flush_on_panic`] can drain it.
static QUEUE: OnceLock<SyncSender<Command>> = OnceLock::new();

/// One unit of work for the writer thread.
enum Command {
    /// Bytes for one destination: a [`target`] index.
    Write { target: usize, bytes: Vec<u8> },
    /// Flush every file, then signal the sender.
    Flush(SyncSender<()>),
}

/// A `tracing` writer that hands each formatted event to the writer thread. Only that thread touches
/// the files and stdout, so a slow disk or console never stalls the caller.
#[derive(Clone)]
struct LogWriter {
    queue: SyncSender<Command>,
    target: usize,
}

impl LogWriter {
    fn new(queue: &SyncSender<Command>, target: usize) -> Self {
        Self {
            queue: queue.clone(),
            target,
        }
    }
}

impl Write for LogWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let command = Command::Write {
            target: self.target,
            bytes: buf.to_vec(),
        };
        // Errors are rare, so they wait until they are on disk: a later abort cannot lose them. Every
        // other line never waits for room, and a full queue drops it.
        let is_error = self.target == target::ERROR;
        let limit = if is_error { WAIT_LIMIT } else { Duration::ZERO };
        if !send_within(&self.queue, command, limit) {
            DROPPED_LINES.fetch_add(1, Ordering::Relaxed);
        } else if is_error {
            wait_for_queue(&self.queue, WAIT_LIMIT);
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        // The writer thread flushes at least every `FLUSH_INTERVAL`, so there is nothing to wait for.
        Ok(())
    }
}

/// Drains the queue into the files and stdout until every sender is gone. Flushes when asked to, and
/// whenever the queue has been quiet for [`FLUSH_INTERVAL`].
fn run_writer(commands: Receiver<Command>, mut logs: RollingLogs) {
    loop {
        match commands.recv_timeout(FLUSH_INTERVAL) {
            Ok(Command::Write { target, bytes }) if target == target::CONSOLE => {
                let _ = io::stdout().lock().write_all(&bytes);
            }
            Ok(Command::Write { target, bytes }) => {
                let _ = logs.write_to(target, &bytes);
            }
            Ok(Command::Flush(ack)) => {
                flush_files(&mut logs);
                let _ = ack.send(());
            }
            Err(RecvTimeoutError::Timeout) => flush_files(&mut logs),
            Err(RecvTimeoutError::Disconnected) => {
                flush_files(&mut logs);
                return;
            }
        }
    }
}

fn flush_files(logs: &mut RollingLogs) {
    for file in &mut logs.files {
        let _ = file.flush();
    }
}

/// Sends `command`, waiting up to `limit` for room in the queue. Returns false when it gave up, which
/// happens only when the queue stays full, or the writer thread has gone.
fn send_within(queue: &SyncSender<Command>, mut command: Command, limit: Duration) -> bool {
    let deadline = Instant::now() + limit;
    loop {
        match queue.try_send(command) {
            Ok(()) => return true,
            Err(TrySendError::Disconnected(_)) => return false,
            Err(TrySendError::Full(returned)) if Instant::now() < deadline => {
                command = returned;
                thread::sleep(Duration::from_millis(1));
            }
            Err(TrySendError::Full(_)) => return false,
        }
    }
}

/// Blocks until the writer has drained everything queued before this call, or until [`WAIT_LIMIT`]
/// passes. It never blocks indefinitely, so a stuck writer cannot hang a panicking process.
fn wait_for_queue(queue: &SyncSender<Command>, limit: Duration) {
    let (ack, done) = mpsc::sync_channel(1);
    if send_within(queue, Command::Flush(ack), limit) {
        let _ = done.recv_timeout(limit);
    }
}

/// Waits until the writer has written every line queued so far. Call it before `main` returns: the writer
/// thread does not outlive the process, so lines still in the queue would otherwise be lost.
pub fn flush() {
    if let Some(queue) = QUEUE.get() {
        wait_for_queue(queue, WAIT_LIMIT);
    }
}

/// Drains the queue before a panic ends the process. `panic = "abort"` runs no destructors, so without
/// this the lines queued just before a panic could be lost. Call it after `color_eyre::install`, so the
/// hook it wraps is the one that runs.
pub fn flush_on_panic() {
    let previous = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        if let Some(queue) = QUEUE.get() {
            wait_for_queue(queue, WAIT_LIMIT);
        }
        previous(info);
    }));
}

/// Lines dropped because the writer thread was behind.
pub fn dropped_lines() -> u64 {
    DROPPED_LINES.load(Ordering::Relaxed)
}

impl<'a> MakeWriter<'a> for LogWriter {
    type Writer = LogWriter;
    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

/// Initialise tracing with rolling, multi-level file logs plus console output.
///
/// Writes a `logs/` directory next to the running executable containing:
/// - `debug.log`  — every level (TRACE..=ERROR), human readable
/// - `latest.log` — INFO and above
/// - `error.log`  — ERROR only
/// - `logs.ndjson` — same content as `debug.log`, structured as newline-delimited JSON
///
/// Log events from the webview frontend flow through here too: the `log`
/// Tauri command (see `lib.rs`) re-emits them as `tracing` events under the
/// `frontend` target, so they land in the same files as the Rust-side logs.
///
/// Existing logs are archived on every launch, and any file that grows past
/// 50 MB triggers a roll. A roll snapshots all four files into
/// `logs/{YYYY-MM-DD}.{ms-past-midnight}.tar.xz` using LZMA2 "Ultra"
/// compression, then truncates them.
pub fn setup_logging() -> color_eyre::Result<()> {
    let dir = logs_dir()?;

    // Per-launch roll: archive whatever the previous run left behind.
    if has_existing_logs(&dir) {
        archive(&dir)?;
    }

    let logs = RollingLogs::open(dir.clone())?;
    let (queue, commands) = mpsc::sync_channel(QUEUE_CAPACITY);
    thread::Builder::new()
        .name("log-writer".into())
        .spawn(move || run_writer(commands, logs))?;
    let _ = QUEUE.set(queue.clone());

    let debug_layer = fmt::layer()
        .with_ansi(false)
        .with_thread_names(true)
        .with_writer(LogWriter::new(&queue, target::DEBUG))
        .with_filter(LevelFilter::TRACE);

    let latest_layer = fmt::layer()
        .with_ansi(false)
        .with_thread_names(true)
        .with_writer(LogWriter::new(&queue, target::LATEST))
        .with_filter(LevelFilter::INFO);

    let error_layer = fmt::layer()
        .with_ansi(false)
        .with_thread_names(true)
        .with_writer(LogWriter::new(&queue, target::ERROR))
        .with_filter(LevelFilter::ERROR);

    let json_layer = fmt::layer()
        .json()
        .with_thread_names(true)
        .with_writer(LogWriter::new(&queue, target::JSON))
        .with_filter(LevelFilter::TRACE);

    let console_filter = if cfg!(debug_assertions) { LevelFilter::DEBUG } else { LevelFilter::INFO };
    let console_layer = fmt::layer()
        .with_thread_names(true)
        .with_writer(LogWriter::new(&queue, target::CONSOLE))
        .with_filter(console_filter);

    tracing_subscriber::registry()
        .with(debug_layer)
        .with(latest_layer)
        .with(error_layer)
        .with(json_layer)
        .with(console_layer)
        .with(perf::layer())
        .init();

    perf::init(&dir, dropped_lines);
    Ok(())
}

/// `logs/` directory next to the current executable, created if missing.
fn logs_dir() -> io::Result<PathBuf> {
    let exe = std::env::current_exe()?;
    let dir = exe.parent().unwrap_or_else(|| Path::new(".")).join("logs");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// True if any prior log file exists with content worth archiving.
fn has_existing_logs(dir: &Path) -> bool {
    LOG_FILES
        .iter()
        .any(|name| dir.join(name).metadata().map(|m| m.len() > 0).unwrap_or(false))
}

/// Create/truncate a single log file, returning a writable handle at offset 0.
fn fresh_file(dir: &Path, name: &str) -> io::Result<File> {
    OpenOptions::new().create(true).write(true).truncate(true).open(dir.join(name))
}

/// Snapshot the current log files into a timestamped LZMA2 "Ultra" tarball:
/// `{YYYY-MM-DD}.{ms-past-midnight}.tar.xz`.
///
/// This must never block logging (or mod startup) for long. The only work done
/// on the calling thread is copying the current logs into a private
/// `.tmp/{stamp}/` staging directory — cheap, and it lets the live files be
/// truncated and reused immediately. The slow LZMA compression then runs off
/// the hot path on a detached thread, so the application (and further logging)
/// resumes at once.
///
/// The logs are *copied* rather than renamed on purpose: during a size roll the
/// live files are still open and truncated in place by [`RollingLogs::roll`],
/// so moving the handles out from under it would break on Windows (sharing
/// violation) and silently detach the handle on Unix.
fn archive(dir: &Path) -> io::Result<()> {
    let now = Local::now();
    let ms_past_midnight = now.num_seconds_from_midnight() as u64 * 1000 + now.timestamp_subsec_millis() as u64;
    let stamp = format!("{}.{}", now.format("%Y-%m-%d"), ms_past_midnight);

    let staging = dir.join(".tmp").join(&stamp);
    fs::create_dir_all(&staging)?;

    let mut staged: Vec<&str> = Vec::new();
    for name in LOG_FILES {
        let src = dir.join(name);
        match fs::metadata(&src) {
            Ok(meta) if meta.len() > 0 => {
                fs::copy(&src, staging.join(name))?;
                staged.push(name);
            }
            Ok(_) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
    }

    // Nothing worth compressing — drop the empty staging dir and bail.
    if staged.is_empty() {
        let _ = fs::remove_dir(&staging);
        return Ok(());
    }

    let archive_path = dir.join(format!("{stamp}.tar.xz"));
    let dir = dir.to_path_buf();
    let staged: Vec<String> = staged.into_iter().map(str::to_owned).collect();

    thread::spawn(move || {
        if let Err(e) = compress_staged(&archive_path, &staging, &staged) {
            eprintln!("log archival failed: {e}");
        }
        let _ = fs::remove_dir_all(&staging);
        if let Err(e) = cleanup_old_logs_archives(&dir) {
            eprintln!("log archive cleanup failed: {e}");
        }
    });

    Ok(())
}

/// Compress the staged log copies in `staging` into `archive_path` as an LZMA2
/// "Ultra" tarball. Runs on a background thread; see [`archive`].
///
/// Progress is written straight to stdout via [`println!`] as bytes are fed
/// into the compressor, throttled to one line per whole-percent change.
fn compress_staged(archive_path: &Path, staging: &Path, names: &[String]) -> io::Result<()> {
    let label = archive_path.file_name().and_then(|n| n.to_str()).unwrap_or("archive");

    let total: u64 = names
        .iter()
        .map(|name| fs::metadata(staging.join(name)).map(|m| m.len()).unwrap_or(0))
        .sum();

    println!("log archive: compressing {label} ({total} bytes)");

    let output = File::create(archive_path)?;
    let stream = Stream::new_easy_encoder(9 | LZMA_PRESET_EXTREME, Check::Crc64).map_err(io::Error::other)?;
    let mut builder = Builder::new(XzEncoder::new_stream(output, stream));

    let mut processed = 0u64;
    let mut last_percent = u8::MAX; // sentinel so the first read always prints
    for name in names {
        let file = File::open(staging.join(name))?;
        let mut header = tar::Header::new_gnu();
        header.set_metadata(&file.metadata()?);
        let reader = ProgressReader {
            inner: file,
            processed: &mut processed,
            total,
            last_percent: &mut last_percent,
            label,
        };
        builder.append_data(&mut header, name, reader)?;
    }

    builder.into_inner()?.finish()?;
    println!("log archive: compressed {label} — done");
    Ok(())
}

/// Wraps a log file being fed into the tarball, reporting cumulative
/// compression progress to stdout as it is read.
struct ProgressReader<'a> {
    inner: File,
    /// Bytes read so far across *all* staged files (shared across the set).
    processed: &'a mut u64,
    /// Total bytes to compress across all staged files.
    total: u64,
    /// Last whole-percent value printed, used to throttle output.
    last_percent: &'a mut u8,
    label: &'a str,
}

impl Read for ProgressReader<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.inner.read(buf)?;
        *self.processed += n as u64;

        if let Some(percent) = (*self.processed * 100u64).checked_div(self.total) {
            let percent = percent as u8;
            if percent != *self.last_percent {
                *self.last_percent = percent;
                println!("log archive: compressing {} — {percent}%", self.label);
            }
        }
        Ok(n)
    }
}

fn cleanup_old_logs_archives(dir: &Path) -> io::Result<()> {
    let logs_older_than_days = 14u8;
    let cutoff = SystemTime::now()
        .checked_sub(Duration::from_secs(logs_older_than_days as u64 * 24 * 3600))
        .unwrap_or(SystemTime::UNIX_EPOCH);

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("xz") {
            continue;
        }
        if let Ok(modified) = entry.metadata()?.modified()
            && modified < cutoff
        {
            fs::remove_file(&path)?;
        }
    }

    Ok(())
}

#[cfg(test)]
#[path = "../tests/logging.rs"]
mod tests;
