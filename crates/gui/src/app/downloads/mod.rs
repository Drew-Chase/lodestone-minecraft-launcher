//! The downloads queue. Each install the New Instance dialog starts is a job here: it waits for a
//! running slot, runs, can be paused and resumed, and ends as done or failed. The Downloads popover
//! beside the sidebar lists the jobs, and the library cards and instance pages show the ones still
//! pending. This module is the state; `actions.rs` starts and stops the installs, and `panel.rs`
//! draws the popover.

mod actions;
mod panel;

pub(crate) use actions::sweep_partial_installs;
pub use panel::{progress_detail, trigger};

/// The queue's messages: the installs' reports and results, and the popover's controls.
#[derive(Debug, Clone)]
pub enum Message {
    /// A running install reported its progress.
    Progress { job: u64, progress: InstallProgress },
    /// An install ended, with its new instance or the error that stopped it.
    Finished { job: u64, result: Result<Box<Instance>, String> },
    /// The database counted the downloads again.
    TotalsLoaded(Result<DownloadTotals, String>),
    /// The trigger was pressed: the popover opens or shuts.
    TogglePanel,
    /// A press landed outside the popover, or Escape was pressed.
    DismissPanel,
    /// The settings button: opens the Settings page.
    OpenSettings,
    /// Pauses a job.
    Pause(u64),
    /// Resumes a paused job.
    Resume(u64),
    /// Asks before a job is cancelled.
    Cancel(u64),
    /// The answer to the cancel question.
    CancelAnswered { job: u64, confirmed: bool },
    /// Pauses every job that has not ended, or resumes them all when none is left to pause.
    TogglePauseAll,
    /// Drops the jobs that have ended.
    ClearCompleted,
}

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use chrono::{DateTime, Local};
use iced::task::Handle;

use crate::app::database::DownloadTotals;
use crate::app::install::{IconSource, InstallProgress, InstallRequest};
use crate::app::pages::library::instance::{Instance, InstanceIcon, InstanceState};

/// How many installs run at once. The rest wait in the queue.
pub const RUNNING_LIMIT: usize = 2;

/// Where a job is in its life.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobState {
    /// Waiting for a running slot.
    Queued,
    /// Installing now.
    Running,
    /// Stopped with its folder kept, to resume later.
    Paused,
    /// Finished, and its instance exists.
    Done,
    /// Stopped by an error. Its folder is gone.
    Failed(String),
}

/// One install: what it will create, and how far it has got.
pub struct Job {
    pub id: u64,
    pub request: InstallRequest,
    pub state: JobState,
    /// The latest report of the running install.
    pub progress: Option<InstallProgress>,
    /// The most bytes the install's downloads have moved, which is what the totals count.
    pub bytes: u64,
    /// Set while the job is paused, so the install keeps its folder when it stops.
    pub keep_folder: Arc<AtomicBool>,
    /// Stops the running install.
    pub abort: Option<Handle>,
    /// When the job was queued. The library sorts pending cards by it.
    pub queued_at: DateTime<Local>,
}

impl Job {
    /// The icon the job's instance will have. An uploaded icon is not on disk yet, so it shows the
    /// catalogue's fallback until the install writes it.
    pub fn icon(&self) -> InstanceIcon {
        match &self.request.icon {
            IconSource::Composed { background, symbol } => InstanceIcon::Composed {
                background: background.clone(),
                symbol: symbol.clone(),
            },
            IconSource::Uploaded(_) => InstanceIcon::Composed {
                background: String::new(),
                symbol: String::new(),
            },
        }
    }

    /// The instance the job will create. The library card and the instance page show it while the
    /// install runs.
    pub fn instance(&self) -> Instance {
        let request = &self.request;
        Instance {
            id: request.id.clone(),
            name: request.name.clone(),
            mc_version: request.mc_version.clone(),
            loader: request.loader,
            loader_version: request.loader_version.clone(),
            playtime_seconds: 0,
            mod_count: 0,
            last_played: None,
            installed_at: self.queued_at,
            pinned: false,
            state: InstanceState::Idle,
            biome: request.biome,
            art_seed: request.art_seed,
            art_path: None,
            icon: self.icon(),
            directory_name: request.directory_name.clone(),
            java_component: request.java_component.clone(),
            version_id: None,
            source_url: request.source_url.clone(),
        }
    }

    /// Whether the job has ended, one way or the other.
    fn ended(&self) -> bool {
        matches!(self.state, JobState::Done | JobState::Failed(_))
    }
}

/// An install that has not ended, as the library shows it: its instance and where it has got to.
#[derive(Debug, Clone)]
pub struct Pending {
    pub instance: Instance,
    pub state: JobState,
    pub progress: Option<InstallProgress>,
}

/// The running installs added up: the share of their files on disk, their download rate, and the
/// bytes they still have to move.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Summary {
    /// The mean of the running installs' fractions, when any of them knows its fraction.
    pub fraction: Option<f32>,
    pub per_second: u64,
    pub remaining_bytes: u64,
}

impl Summary {
    /// Seconds left at the current rate, when the rate is known.
    pub fn eta_seconds(&self) -> Option<u64> {
        (self.per_second > 0).then(|| self.remaining_bytes / self.per_second)
    }
}

/// The queue and the popover's state.
#[derive(Default)]
pub struct Downloads {
    jobs: Vec<Job>,
    next_id: u64,
    open: bool,
    totals: DownloadTotals,
}

impl Downloads {
    /// Queues an install and returns its job id.
    pub fn enqueue(&mut self, request: InstallRequest) -> u64 {
        self.next_id += 1;
        let id = self.next_id;
        self.jobs.push(Job {
            id,
            request,
            state: JobState::Queued,
            progress: None,
            bytes: 0,
            keep_folder: Arc::new(AtomicBool::new(false)),
            abort: None,
            queued_at: Local::now(),
        });
        id
    }

    pub fn jobs(&self) -> &[Job] {
        &self.jobs
    }

    pub fn job(&self, id: u64) -> Option<&Job> {
        self.jobs.iter().find(|job| job.id == id)
    }

    fn job_mut(&mut self, id: u64) -> Option<&mut Job> {
        self.jobs.iter_mut().find(|job| job.id == id)
    }

    /// Whether the popover is open.
    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn set_open(&mut self, open: bool) {
        self.open = open;
    }

    /// The bytes downloaded today and this week, as the database last counted them.
    pub fn totals(&self) -> DownloadTotals {
        self.totals
    }

    pub fn set_totals(&mut self, totals: DownloadTotals) {
        self.totals = totals;
    }

    /// How many jobs have not ended: the count the sidebar's download icon shows.
    pub fn badge(&self) -> usize {
        self.jobs.iter().filter(|job| !job.ended()).count()
    }

    /// How many installs are running now.
    pub fn running(&self) -> usize {
        self.jobs.iter().filter(|job| job.state == JobState::Running).count()
    }

    /// The next queued job to start, when a running slot is free.
    pub fn next_start(&self) -> Option<u64> {
        if self.running() >= RUNNING_LIMIT {
            return None;
        }
        self.jobs.iter().find(|job| job.state == JobState::Queued).map(|job| job.id)
    }

    /// The keep flag the install of job `id` shares, so a pause can set it.
    pub fn keep_flag(&self, id: u64) -> Option<Arc<AtomicBool>> {
        self.job(id).map(|job| job.keep_folder.clone())
    }

    /// The request the install of job `id` runs.
    pub fn request(&self, id: u64) -> Option<&InstallRequest> {
        self.job(id).map(|job| &job.request)
    }

    /// Marks job `id` as running, with the handle that stops its install.
    pub fn start(&mut self, id: u64, abort: Handle) {
        if let Some(job) = self.job_mut(id).filter(|job| job.state == JobState::Queued) {
            job.state = JobState::Running;
            job.abort = Some(abort);
        }
    }

    /// Records a report from a running install.
    pub fn record(&mut self, id: u64, progress: InstallProgress) {
        if let Some(job) = self.job_mut(id).filter(|job| job.state == JobState::Running) {
            job.bytes = job.bytes.max(progress.transfer.done);
            job.progress = Some(progress);
        }
    }

    /// Pauses job `id`. A running install is stopped with its folder kept, and the handle that stops
    /// it is returned for the caller to abort.
    pub fn pause(&mut self, id: u64) -> Option<Handle> {
        let job = self.job_mut(id)?;
        match job.state {
            JobState::Queued => {
                job.state = JobState::Paused;
                None
            }
            JobState::Running => {
                job.keep_folder.store(true, Ordering::SeqCst);
                job.state = JobState::Paused;
                job.abort.take()
            }
            _ => None,
        }
    }

    /// Puts a paused job back in the queue. Its install finds the folder it kept.
    pub fn resume(&mut self, id: u64) {
        if let Some(job) = self.job_mut(id).filter(|job| job.state == JobState::Paused) {
            job.keep_folder.store(false, Ordering::SeqCst);
            job.state = JobState::Queued;
        }
    }

    /// Pauses every job that has not ended, and returns the handles of the installs that were running.
    pub fn pause_all(&mut self) -> Vec<Handle> {
        let ids: Vec<u64> = self
            .jobs
            .iter()
            .filter(|job| matches!(job.state, JobState::Queued | JobState::Running))
            .map(|job| job.id)
            .collect();
        ids.into_iter().filter_map(|id| self.pause(id)).collect()
    }

    /// Whether any job is paused.
    pub fn any_paused(&self) -> bool {
        self.jobs.iter().any(|job| job.state == JobState::Paused)
    }

    /// Whether any job is queued or running, so pause-all has something to pause.
    pub fn has_pausable(&self) -> bool {
        self.jobs.iter().any(|job| matches!(job.state, JobState::Queued | JobState::Running))
    }

    /// Takes the bytes job `id` moved in its current run, so each run's bytes are counted once.
    pub fn take_bytes(&mut self, id: u64) -> u64 {
        self.job_mut(id).map_or(0, |job| std::mem::take(&mut job.bytes))
    }

    /// Takes the bytes every job moved in its current run.
    pub fn take_all_bytes(&mut self) -> u64 {
        self.jobs.iter_mut().map(|job| std::mem::take(&mut job.bytes)).sum()
    }

    /// Resumes every paused job.
    pub fn resume_all(&mut self) {
        let ids: Vec<u64> = self.jobs.iter().filter(|job| job.state == JobState::Paused).map(|job| job.id).collect();
        for id in ids {
            self.resume(id);
        }
    }

    /// Removes job `id`. A running install is stopped, and its folder goes with the drop of its
    /// sipper. A paused job has no sipper, so its folder is returned for the caller to remove.
    pub fn cancel(&mut self, id: u64) -> Option<PathBuf> {
        let index = self.jobs.iter().position(|job| job.id == id)?;
        let job = self.jobs.remove(index);
        if let Some(abort) = &job.abort {
            abort.abort();
        }
        match job.state {
            JobState::Paused => Some(job.request.paths.instance(&job.request.directory_name)),
            _ => None,
        }
    }

    /// Ends job `id`: done, or failed with the error that stopped its install.
    pub fn finish(&mut self, id: u64, outcome: Result<(), String>) {
        if let Some(job) = self.job_mut(id) {
            job.state = match outcome {
                Ok(()) => JobState::Done,
                Err(error) => JobState::Failed(error),
            };
            job.progress = None;
            job.abort = None;
        }
    }

    /// Drops the jobs that have ended.
    pub fn clear_completed(&mut self) {
        self.jobs.retain(|job| !job.ended());
    }

    /// The job whose install will create the instance `id`, while that install has not ended.
    pub fn job_for_instance(&self, id: &str) -> Option<u64> {
        self.jobs.iter().find(|job| !job.ended() && job.request.id == id).map(|job| job.id)
    }

    /// The directory names the jobs that have not ended will create, so a new install avoids them.
    pub fn reserved_directories(&self) -> Vec<String> {
        self.jobs
            .iter()
            .filter(|job| !job.ended())
            .map(|job| job.request.directory_name.clone())
            .collect()
    }

    /// The installs that have not ended, as the library shows them.
    pub fn pending(&self) -> Vec<Pending> {
        self.jobs
            .iter()
            .filter(|job| !job.ended())
            .map(|job| Pending {
                instance: job.instance(),
                state: job.state.clone(),
                progress: job.progress.clone(),
            })
            .collect()
    }

    /// The running installs added up, for the popover's header.
    pub fn summary(&self) -> Summary {
        let running: Vec<&InstallProgress> = self
            .jobs
            .iter()
            .filter(|job| job.state == JobState::Running)
            .filter_map(|job| job.progress.as_ref())
            .collect();
        let fractions: Vec<f32> = running.iter().filter_map(|progress| progress.fraction).collect();
        let fraction = (!fractions.is_empty()).then(|| fractions.iter().sum::<f32>() / fractions.len() as f32);
        Summary {
            fraction,
            per_second: running.iter().map(|progress| progress.transfer.per_second).sum(),
            remaining_bytes: running
                .iter()
                .map(|progress| progress.transfer.total.saturating_sub(progress.transfer.done))
                .sum(),
        }
    }
}

#[cfg(test)]
#[path = "../../../tests/app/downloads/mod.rs"]
mod tests;
