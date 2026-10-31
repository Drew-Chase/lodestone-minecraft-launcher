//! The download engine the install and the launch share: finding which files are missing,
//! fetching them in parallel, and reporting how far each part of an install has got.

use std::future::Future;

use simple_download_utility::{FileDownloadArguments, MultiDownloadProgress, download_multiple_files_with_client};
use tokio::sync::mpsc;

use crate::app::lang;
use crate::app::launch::libraries::Item;

/// How many files one download fetches at once.
pub const PARALLEL: u16 = 64;

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

impl Transfer {
    /// What a multi-file download reports, in bytes.
    fn of(progress: &MultiDownloadProgress) -> Self {
        Self {
            done: progress.bytes_downloaded as u64,
            total: progress.bytes_to_download as u64,
            per_second: progress.bytes_per_second as u64,
        }
    }
}

/// Items split by whether they are already on disk.
pub struct Missing {
    /// The items to fetch.
    pub missing: Vec<Item>,
    /// How many were already on disk.
    pub present: usize,
}

/// Splits `items` into the ones to fetch and the ones already on disk. A file is on disk when
/// it exists at the size the version expects. Without a size it only has to be non-empty.
pub async fn missing(items: Vec<Item>) -> Result<Missing, String> {
    tokio::task::spawn_blocking(move || {
        let (present, missing): (Vec<Item>, Vec<Item>) = items.into_iter().partition(is_present);
        Missing {
            present: present.len(),
            missing,
        }
    })
    .await
    .map_err(|error| error.to_string())
}

fn is_present(item: &Item) -> bool {
    match std::fs::metadata(&item.path) {
        Ok(meta) => meta.is_file() && item.size.map_or(meta.len() > 0, |size| meta.len() == size),
        Err(_) => false,
    }
}

/// Fetches `items` in parallel. `part` reports after each file lands, counting the `skipped`
/// files that were already on disk out of `total`.
pub async fn fetch(
    client: &reqwest::Client,
    items: Vec<Item>,
    part: Part,
    skipped: usize,
    total: usize,
    reports: &mpsc::Sender<PartReport>,
) -> Result<(), String> {
    if items.is_empty() {
        return Ok(());
    }
    let mut downloads = Vec::with_capacity(items.len());
    for item in items {
        let Some(url) = item.url else {
            return Err(lang::t_with("install.no_download", &[("path", item.path.display().to_string().as_str())]));
        };
        downloads.push(FileDownloadArguments {
            url,
            sha1: item.sha1,
            sender: None,
            path: item.path.to_string_lossy().into_owned(),
        });
    }

    let (sender, mut progress) = mpsc::channel::<MultiDownloadProgress>(64);
    let reports = reports.clone();
    let forward = tokio::spawn(async move {
        while let Some(report) = progress.recv().await {
            let _ = reports
                .send(PartReport {
                    part,
                    done: skipped + report.files_downloaded,
                    total,
                    transfer: Transfer::of(&report),
                })
                .await;
        }
    });
    let result = download_multiple_files_with_client(client.clone(), downloads, PARALLEL, Some(sender)).await;
    let _ = forward.await;
    result.map_err(|error| error.to_string())
}

/// Runs `work` while forwarding its multi-file progress to `reports` as `part`. The runtime
/// installer reports this way, and its files arrive while `work` runs.
pub async fn relay<T>(
    reports: &mpsc::Sender<PartReport>,
    part: Part,
    mut files: mpsc::Receiver<MultiDownloadProgress>,
    work: impl Future<Output = T>,
) -> T {
    tokio::pin!(work);
    let mut open = true;
    loop {
        tokio::select! {
            report = files.recv(), if open => match report {
                Some(report) => {
                    let transfer = Transfer::of(&report);
                    let _ = reports
                        .send(PartReport {
                            part,
                            done: report.files_downloaded,
                            total: report.files_total,
                            transfer,
                        })
                        .await;
                }
                None => open = false,
            },
            output = &mut work => return output,
        }
    }
}

/// Fetches `items` in parallel with no progress report, for the small files no part counts.
pub async fn fetch_quiet(client: &reqwest::Client, items: Vec<Item>) -> Result<(), String> {
    if items.is_empty() {
        return Ok(());
    }
    let mut downloads = Vec::with_capacity(items.len());
    for item in items {
        let Some(url) = item.url else {
            return Err(lang::t_with("install.no_download", &[("path", item.path.display().to_string().as_str())]));
        };
        downloads.push(FileDownloadArguments {
            url,
            sha1: item.sha1,
            sender: None,
            path: item.path.to_string_lossy().into_owned(),
        });
    }
    download_multiple_files_with_client(client.clone(), downloads, PARALLEL, None)
        .await
        .map_err(|error| error.to_string())
}

#[cfg(test)]
#[path = "../../../tests/app/install/download.rs"]
mod tests;
