//! The downloads queue. Each install the New Instance dialog starts is a job here: it waits for a
//! running slot, runs, can be paused and resumed, and ends as done or failed. A mod added to an instance
//! from Discover is a job too. It runs at once, cannot be paused or cancelled, and ends as done or failed.
//! An export is a job as well: it runs at once, outside the install slots, and can be cancelled.
//! The Downloads popover beside the sidebar lists the jobs, and the library cards and instance pages show
//! the installs still pending. This module is the state; `actions.rs` starts and stops the installs, and
//! `panel.rs` draws the popover.

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
    /// An export reported its progress.
    ExportProgress { job: u64, progress: ExportProgress },
    /// An export ended, with the path of its package or the error that stopped it.
    ExportFinished { job: u64, result: Result<PathBuf, String> },
    /// Opens the folder that holds the package a finished export wrote.
    RevealExport(u64),
}

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use chrono::{DateTime, Local};
use iced::task::Handle;

use crate::app::database::DownloadTotals;
use crate::app::export::job::{ExportJob, ExportProgress};
use crate::app::install::{IconSource, InstallProgress, InstallRequest};
use crate::app::pages::discover::Source;
use crate::app::pages::library::instance::{Instance, InstanceIcon, InstanceState};

/// How many installs run at once. The rest wait in the queue. Mods are not counted against it.
pub const RUNNING_LIMIT: usize = 2;

/// Where a job is in its life.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobState {
    /// Waiting for a running slot.
    Queued,
    /// Installing now, or adding a mod now.
    Running,
    /// Stopped with its folder kept, to resume later.
    Paused,
    /// Finished. The instance exists, or the mod is in its folder.
    Done,
    /// Stopped by an error. An install's folder is gone.
    Failed(String),
}

/// What a job does: create an instance, or put a mod into one.
pub enum JobKind {
    /// Creates an instance from its request. Boxed, so the job's small mod variant does not take the install's space.
    Install(Box<InstallRequest>),
    /// Adds a mod to an instance, or swaps the mod's jar for another release.
    Mod(ModJob),
    /// Writes an instance out as a package. It runs outside the install slots, like a mod, and can be cancelled.
    Export(ExportJob),
    /// Fetches a Minecraft version's client jar and profile again. It runs outside the install slots, and has no card.
    Reinstall(ReinstallJob),
}

/// A Minecraft version being fetched again, as the popover shows it.
#[derive(Debug, Clone)]
pub struct ReinstallJob {
    /// The instance the reinstall was asked from, which the popover names.
    pub name: String,
    /// The Minecraft version whose files are fetched again.
    pub mc_version: String,
}

/// A mod going into an instance, as the popover shows it.
#[derive(Debug, Clone)]
pub struct ModJob {
    /// The mod's name.
    pub name: String,
    /// The platform the mod comes from.
    pub source: Source,
    /// The name of the instance the mod goes into.
    pub instance: String,
    /// Whether the job swaps the mod's jar for another release, rather than adding the mod.
    pub replacing: bool,
}

/// One job in the queue: an install or a mod, and how far it has got.
pub struct Job {
    pub id: u64,
    pub kind: JobKind,
    pub state: JobState,
    /// The latest report of the running install. Mods report none.
    pub progress: Option<InstallProgress>,
    /// The most bytes the job's downloads have moved, which is what the totals count.
    pub bytes: u64,
    /// Set while the job is paused, so the install keeps its folder when it stops.
    pub keep_folder: Arc<AtomicBool>,
    /// Stops the running install.
    pub abort: Option<Handle>,
    /// When the job was queued. The library sorts pending cards by it.
    pub queued_at: DateTime<Local>,
}

impl Job {
    /// The install the job runs, when it is an install.
    pub fn install(&self) -> Option<&InstallRequest> {
        match &self.kind {
            JobKind::Install(request) => Some(request.as_ref()),
            JobKind::Mod(_) | JobKind::Export(_) | JobKind::Reinstall(_) => None,
        }
    }

    /// The job's name: the instance's name for an install or an export, the mod's for a mod.
    pub fn name(&self) -> &str {
        match &self.kind {
            JobKind::Install(request) => &request.name,
            JobKind::Mod(mod_job) => &mod_job.name,
            JobKind::Export(export) => &export.name,
            JobKind::Reinstall(reinstall) => &reinstall.name,
        }
    }

    /// The instance the job will create. The library card and the instance page show it while the install runs.
    /// `None` for a mod, which creates no instance.
    pub fn instance(&self) -> Option<Instance> {
        let request = self.install()?;
        Some(Instance {
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
            icon: icon_of(request),
            directory_name: request.directory_name.clone(),
            java_component: request.java_component.clone(),
            version_id: None,
            source_url: request.source_url.clone(),
            source_platform: request.source_platform.clone(),
            source_project_id: request.source_project_id.clone(),
            settings: request.settings.clone(),
        })
    }

    /// Whether the job has ended, one way or the other.
    fn ended(&self) -> bool {
        matches!(self.state, JobState::Done | JobState::Failed(_))
    }
}

/// The icon an instance gets from its install request. An uploaded icon is not on disk yet, so it shows the
/// catalogue's fallback until the install writes it.
pub fn icon_of(request: &InstallRequest) -> InstanceIcon {
    match &request.icon {
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
        self.push(JobKind::Install(Box::new(request)), JobState::Queued)
    }

    /// Starts a mod going into an instance and returns its job id. The mod runs at once, outside the install slots.
    pub fn enqueue_mod(&mut self, mod_job: ModJob) -> u64 {
        self.push(JobKind::Mod(mod_job), JobState::Running)
    }

    /// Starts a reinstall and returns its job id. It runs at once, outside the install slots.
    pub fn enqueue_reinstall(&mut self, reinstall: ReinstallJob) -> u64 {
        self.push(JobKind::Reinstall(reinstall), JobState::Running)
    }

    /// Starts an export and returns its job id. It runs at once, outside the install slots.
    pub fn enqueue_export(&mut self, export: ExportJob) -> u64 {
        self.push(JobKind::Export(export), JobState::Running)
    }

    /// Records a report from a running export.
    pub fn record_export(&mut self, id: u64, progress: ExportProgress) {
        if let Some(JobKind::Export(export)) = self.job_mut(id).map(|job| &mut job.kind) {
            export.progress = Some(progress);
        }
    }

    /// The package job `id` writes, when the job is an export.
    pub fn export_target(&self, id: u64) -> Option<PathBuf> {
        match &self.job(id)?.kind {
            JobKind::Export(export) => Some(export.target.clone()),
            _ => None,
        }
    }

    /// Whether job `id` is an export.
    pub fn is_export(&self, id: u64) -> bool {
        self.job(id).is_some_and(|job| matches!(job.kind, JobKind::Export(_)))
    }

    fn push(&mut self, kind: JobKind, state: JobState) -> u64 {
        self.next_id += 1;
        let id = self.next_id;
        self.jobs.push(Job {
            id,
            kind,
            state,
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

    /// How many jobs are running now, installs and mods alike.
    pub fn running(&self) -> usize {
        self.jobs.iter().filter(|job| job.state == JobState::Running).count()
    }

    /// How many installs are running now. Only installs take the slots that [`RUNNING_LIMIT`] allows.
    pub fn installs_running(&self) -> usize {
        self.jobs
            .iter()
            .filter(|job| job.install().is_some() && job.state == JobState::Running)
            .count()
    }

    /// The next queued install to start, when a running slot is free.
    pub fn next_start(&self) -> Option<u64> {
        if self.installs_running() >= RUNNING_LIMIT {
            return None;
        }
        self.jobs
            .iter()
            .find(|job| job.install().is_some() && job.state == JobState::Queued)
            .map(|job| job.id)
    }

    /// The keep flag the install of job `id` shares, so a pause can set it.
    pub fn keep_flag(&self, id: u64) -> Option<Arc<AtomicBool>> {
        self.job(id).map(|job| job.keep_folder.clone())
    }

    /// The request the install of job `id` runs.
    pub fn request(&self, id: u64) -> Option<&InstallRequest> {
        self.job(id).and_then(|job| job.install())
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

    /// Pauses job `id`. A running install is stopped with its folder kept, and the handle that stops it is returned for
    /// the caller to abort. A mod cannot be paused, so it is left running.
    pub fn pause(&mut self, id: u64) -> Option<Handle> {
        let job = self.job_mut(id).filter(|job| job.install().is_some())?;
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

    /// Pauses every install that has not ended, and returns the handles of the installs that were running.
    pub fn pause_all(&mut self) -> Vec<Handle> {
        let ids: Vec<u64> = self
            .jobs
            .iter()
            .filter(|job| job.install().is_some() && matches!(job.state, JobState::Queued | JobState::Running))
            .map(|job| job.id)
            .collect();
        ids.into_iter().filter_map(|id| self.pause(id)).collect()
    }

    /// Whether any job is paused.
    pub fn any_paused(&self) -> bool {
        self.jobs.iter().any(|job| job.state == JobState::Paused)
    }

    /// Whether any install is queued or running, so pause-all has something to pause.
    pub fn has_pausable(&self) -> bool {
        self.jobs
            .iter()
            .any(|job| job.install().is_some() && matches!(job.state, JobState::Queued | JobState::Running))
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

    /// Removes install or export `id`. A running install is stopped, and its folder goes with the drop of its sipper. A
    /// paused job has no sipper, so its folder is returned for the caller to remove. An export is stopped at its next
    /// file, and removes its own partial package. A mod cannot be cancelled, so it stays.
    pub fn cancel(&mut self, id: u64) -> Option<PathBuf> {
        let index = self
            .jobs
            .iter()
            .position(|job| job.id == id && matches!(job.kind, JobKind::Install(_) | JobKind::Export(_)))?;
        let job = self.jobs.remove(index);
        if let JobKind::Export(export) = &job.kind {
            export.cancel.store(true, Ordering::SeqCst);
            return None;
        }
        if let Some(abort) = &job.abort {
            abort.abort();
        }
        match job.state {
            JobState::Paused => job.install().map(|request| request.paths.instance(&request.directory_name)),
            _ => None,
        }
    }

    /// Ends job `id`: done, or failed with the error that stopped it.
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
        self.jobs
            .iter()
            .find(|job| !job.ended() && job.install().is_some_and(|request| request.id == id))
            .map(|job| job.id)
    }

    /// The directory names the installs that have not ended will create, so a new install avoids them.
    pub fn reserved_directories(&self) -> Vec<String> {
        self.jobs
            .iter()
            .filter(|job| !job.ended())
            .filter_map(|job| job.install().map(|request| request.directory_name.clone()))
            .collect()
    }

    /// The installs that have not ended, as the library shows them. A mod creates no instance, so it is not listed.
    pub fn pending(&self) -> Vec<Pending> {
        self.jobs
            .iter()
            .filter(|job| !job.ended())
            .filter_map(|job| {
                Some(Pending {
                    instance: job.instance()?,
                    state: job.state.clone(),
                    progress: job.progress.clone(),
                })
            })
            .collect()
    }

    /// The running installs added up, for the popover's header. Mods report no progress, so they add nothing.
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
