//! The download engine the install and the launch share: finding which files are missing, fetching them in parallel through
//! the launcher's checked downloader, and reporting how far each part of an install has got.

use std::num::NonZeroUsize;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use lodestone_net::download::{Target, sha1_of_file};
use tokio::sync::{Semaphore, mpsc};
use tokio::task::JoinSet;

use crate::app::lang;
use crate::app::launch::libraries::Item;
use crate::app::preferences::DEFAULT_CONCURRENT_DOWNLOADS;

/// The default number of files a download fetches at once.
pub const PARALLEL: u16 = DEFAULT_CONCURRENT_DOWNLOADS;

/// How many files a download fetches at once. The Settings page sets it, and each download reads it when it starts.
static CONCURRENCY: AtomicU16 = AtomicU16::new(PARALLEL);

/// How often a running download reports its progress.
const REPORT_EVERY: Duration = Duration::from_millis(150);

/// Sets how many files a download fetches at once. Zero is raised to one, so a download can always make progress.
pub fn set_concurrency(count: u16) {
    CONCURRENCY.store(count.max(1), Ordering::Relaxed);
}

/// How many files a download fetches at once.
pub fn concurrency() -> u16 {
    CONCURRENCY.load(Ordering::Relaxed)
}

/// The parts of an install that run side by side. Each reports its own count of files, and
/// the install adds them up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    Java,
    Client,
    Libraries,
    Assets,
    /// The mod files a modpack lists.
    Mods,
}

/// One part's progress: files on disk of the files it needs, and the bytes its downloads have moved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PartReport {
    pub part: Part,
    pub done: usize,
    pub total: usize,
    pub transfer: Transfer,
}

impl PartReport {
    /// A count with no transfer behind it, for the parts that do not download.
    pub fn counted(part: Part, done: usize, total: usize) -> Self {
        Self {
            part,
            done,
            total,
            transfer: Transfer::default(),
        }
    }
}

/// The bytes a download has moved: how many of how many, and the rate over its latest stretch.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Transfer {
    pub done: u64,
    pub total: u64,
    pub per_second: u64,
}

/// Items split by whether they are already on disk.
pub struct Missing {
    /// The items to fetch.
    pub missing: Vec<Item>,
    /// How many were already on disk.
    pub present: usize,
}

/// Splits `items` into the ones to fetch and the ones already on disk. A file is on disk when it exists at the size the
/// version expects and, when the version names one, hashes to its SHA-1. Without a size it only has to be non-empty.
///
/// Hashing runs on the blocking pool, spread across the cores, so a large asset index does not hold one thread.
pub async fn missing(items: Vec<Item>) -> Result<Missing, String> {
    tokio::task::spawn_blocking(move || {
        let on_disk = verdicts(&items);
        let mut split = Missing {
            missing: Vec::new(),
            present: 0,
        };
        for (item, here) in items.into_iter().zip(on_disk) {
            if here {
                split.present += 1;
            } else {
                split.missing.push(item);
            }
        }
        split
    })
    .await
    .map_err(|error| error.to_string())
}

/// Whether each item is on disk as its version expects, in order. The items are checked in chunks on scoped threads.
fn verdicts(items: &[Item]) -> Vec<bool> {
    let threads = std::thread::available_parallelism().map_or(4, NonZeroUsize::get).clamp(1, 8);
    let chunk = items.len().div_ceil(threads).max(1);
    std::thread::scope(|scope| {
        let handles: Vec<_> = items
            .chunks(chunk)
            .map(|part| (part.len(), scope.spawn(move || part.iter().map(is_present).collect::<Vec<bool>>())))
            .collect();
        handles
            .into_iter()
            .flat_map(|(len, handle)| handle.join().unwrap_or_else(|_| vec![false; len]))
            .collect()
    })
}

/// Whether `item` is on disk as the version expects it: at its size, and hashing to its SHA-1 when it has one.
fn is_present(item: &Item) -> bool {
    let Ok(meta) = std::fs::metadata(&item.path) else {
        return false;
    };
    if !meta.is_file() {
        return false;
    }
    let sized = match item.size {
        Some(size) => meta.len() == size,
        None => meta.len() > 0,
    };
    sized
        && item
            .sha1
            .as_deref()
            .is_none_or(|expected| sha1_of_file(&item.path).is_ok_and(|actual| actual.eq_ignore_ascii_case(expected)))
}

/// [`missing`] when `verify` is set. When it is not, the files are taken as present without being looked at, so
/// nothing is fetched and a launch starts without checking the disk.
pub async fn missing_when(items: Vec<Item>, verify: bool) -> Result<Missing, String> {
    if verify {
        return missing(items).await;
    }
    Ok(Missing {
        present: items.len(),
        missing: Vec::new(),
    })
}

/// Fetches `items` in parallel, `concurrency()` files at a time. `part` reports every [`REPORT_EVERY`] while the files run,
/// counting the `skipped` files already on disk out of `total`. The first failure stops the files not yet started, and
/// removes the part files of the ones in flight.
pub async fn fetch(
    client: &reqwest::Client,
    items: Vec<Item>,
    part: Part,
    skipped: usize,
    total: usize,
    reports: &mpsc::Sender<PartReport>,
) -> Result<(), String> {
    let targets = targets(items)?;
    if targets.is_empty() {
        return Ok(());
    }
    let meter = Arc::new(Meter::default());
    meter
        .bytes_total
        .store(targets.iter().map(|target| target.size.unwrap_or(0)).sum(), Ordering::Relaxed);
    let gauge = Gauge { part, skipped, total };
    let reporter = tokio::spawn(report(Arc::clone(&meter), reports.clone(), gauge));
    let result = run(client, targets, Arc::clone(&meter)).await;
    meter.finished.store(true, Ordering::Release);
    let _ = reporter.await;
    result
}

/// Fetches `items` in parallel with no progress report, for the small files no part counts.
pub async fn fetch_quiet(client: &reqwest::Client, items: Vec<Item>) -> Result<(), String> {
    run(client, targets(items)?, Arc::new(Meter::default())).await
}

/// The downloads for `items`. An item with no url cannot be fetched, and says which file it was.
fn targets(items: Vec<Item>) -> Result<Vec<Target>, String> {
    items
        .into_iter()
        .map(|item| {
            let Some(url) = item.url else {
                return Err(lang::t_with("install.no_download", &[("path", item.path.display().to_string().as_str())]));
            };
            Ok(Target {
                url,
                path: item.path,
                sha1: item.sha1,
                size: item.size,
            })
        })
        .collect()
}

/// The counts the files of one download share. Each chunk adds to two counters, and the reporter reads them, so the cost of
/// a report does not depend on how many files there are.
#[derive(Default)]
struct Meter {
    files: AtomicUsize,
    bytes_done: AtomicU64,
    bytes_total: AtomicU64,
    failed: AtomicBool,
    finished: AtomicBool,
}

/// What the reporter adds to the counts: the part, the files on disk before the download, and the part's file count.
struct Gauge {
    part: Part,
    skipped: usize,
    total: usize,
}

/// Runs every target with at most `concurrency()` in flight, and returns the first failure.
async fn run(client: &reqwest::Client, targets: Vec<Target>, meter: Arc<Meter>) -> Result<(), String> {
    if targets.is_empty() {
        return Ok(());
    }
    let slots = Arc::new(Semaphore::new(usize::from(concurrency())));
    let mut tasks = JoinSet::new();
    for target in targets {
        let slot = Arc::clone(&slots).acquire_owned().await.map_err(|error| error.to_string())?;
        let (client, meter) = (client.clone(), Arc::clone(&meter));
        tasks.spawn(async move {
            let _slot = slot;
            fetch_one(&client, &target, &meter).await
        });
    }
    let mut first = None;
    while let Some(joined) = tasks.join_next().await {
        let outcome = joined.map_err(|error| error.to_string()).and_then(|result| result);
        if let Err(error) = outcome {
            first.get_or_insert(error);
        }
    }
    first.map_or(Ok(()), Err)
}

/// Fetches one target, adding its bytes to the meter as they arrive. A target that starts after another has failed does nothing.
async fn fetch_one(client: &reqwest::Client, target: &Target, meter: &Meter) -> Result<(), String> {
    if meter.failed.load(Ordering::Acquire) {
        return Ok(());
    }
    // The bytes this file has counted so far, and whether its total is already in the meter (a file with a size named
    // in the item has been counted up front).
    let mut seen = 0u64;
    let mut announced = target.size.is_some();
    let result = lodestone_net::download::fetch(client, target, |received, total| {
        meter.bytes_done.fetch_add(received - seen, Ordering::Relaxed);
        seen = received;
        if !announced && let Some(total) = total {
            meter.bytes_total.fetch_add(total, Ordering::Relaxed);
            announced = true;
        }
    })
    .await;
    match result {
        Ok(written) => {
            if !announced {
                meter.bytes_total.fetch_add(written, Ordering::Relaxed);
            }
            meter.files.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
        Err(error) => {
            meter.failed.store(true, Ordering::Release);
            Err(error.to_string())
        }
    }
}

/// Sends the part's progress every [`REPORT_EVERY`] until the download finishes, then once more with the final counts.
/// The rate is the bytes moved since the previous report, over the time between the two.
async fn report(meter: Arc<Meter>, reports: mpsc::Sender<PartReport>, gauge: Gauge) {
    let mut last = (Instant::now(), 0u64);
    loop {
        tokio::time::sleep(REPORT_EVERY).await;
        if meter.finished.load(Ordering::Acquire) {
            break;
        }
        let (report, moved_at) = snapshot(&meter, &gauge, last);
        last = moved_at;
        let _ = reports.try_send(report);
    }
    let (report, _) = snapshot(&meter, &gauge, last);
    let _ = reports.send(report).await;
}

/// The part's report now, and the moment and byte count it was taken at, for the next rate.
fn snapshot(meter: &Meter, gauge: &Gauge, (since, moved): (Instant, u64)) -> (PartReport, (Instant, u64)) {
    let now = Instant::now();
    let done = meter.bytes_done.load(Ordering::Relaxed);
    let elapsed = now.duration_since(since).as_secs_f64();
    let per_second = if elapsed > 0.0 {
        (done.saturating_sub(moved) as f64 / elapsed) as u64
    } else {
        0
    };
    let report = PartReport {
        part: gauge.part,
        done: gauge.skipped + meter.files.load(Ordering::Relaxed),
        total: gauge.total,
        transfer: Transfer {
            done,
            total: meter.bytes_total.load(Ordering::Relaxed),
            per_second,
        },
    };
    (report, (now, done))
}

#[cfg(test)]
#[path = "../../../tests/app/install/download.rs"]
mod tests;
