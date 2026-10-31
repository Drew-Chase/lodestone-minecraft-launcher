//! Tests for `downloads`: the queue's scheduling, pausing, totals and the instances it lists while they
//! install. They live here rather than inline, and reach the private helpers through `super`.

use iced::Task;

use super::*;
use crate::app::install::Stage;
use crate::app::install::download::Transfer;
use crate::app::pages::library::instance::{Biome, Loader};
use crate::app::paths::LauncherPaths;

/// A request for an instance called `name`, with an id derived from the name.
fn request(name: &str) -> InstallRequest {
    InstallRequest {
        paths: LauncherPaths::default(),
        name: name.to_owned(),
        directory_name: name.to_owned(),
        mc_version: "1.20.1".to_owned(),
        loader: Loader::Fabric,
        loader_version: Some("0.14.21".to_owned()),
        java_component: None,
        icon: IconSource::Composed {
            background: "rose".to_owned(),
            symbol: "grass".to_owned(),
        },
        biome: Biome::Forest,
        art_seed: 1,
        banner: None,
        id: format!("id-{name}"),
        source_url: None,
        source_platform: None,
        source_project_id: None,
        modpack: None,
    }
}

/// A handle for a task that never runs, so a job can be marked as running.
fn handle() -> Handle {
    Task::<()>::none().abortable().1
}

/// A report from a running install: its stage, its share of files, and its byte transfer.
fn report(fraction: f32, done: u64, total: u64, per_second: u64) -> InstallProgress {
    InstallProgress {
        stage: Stage::Download,
        fraction: Some(fraction),
        detail: "1/2 files".to_owned(),
        transfer: Transfer { done, total, per_second },
    }
}

#[test]
fn two_installs_run_at_once_and_a_finished_one_frees_its_slot() {
    let mut queue = Downloads::default();
    let first = queue.enqueue(request("First"));
    let second = queue.enqueue(request("Second"));
    let third = queue.enqueue(request("Third"));

    assert_eq!(queue.next_start(), Some(first));
    queue.start(first, handle());
    assert_eq!(queue.next_start(), Some(second));
    queue.start(second, handle());
    assert_eq!(queue.next_start(), None, "a third install waits for a slot");
    assert_eq!(queue.running(), 2);

    queue.finish(first, Ok(()));
    assert_eq!(queue.next_start(), Some(third), "the finished install frees its slot");
}

#[test]
fn pausing_a_running_install_keeps_its_folder_and_frees_its_slot() {
    let mut queue = Downloads::default();
    let id = queue.enqueue(request("Paused"));
    queue.start(id, handle());

    assert!(queue.pause(id).is_some(), "the running install's handle comes back to be aborted");
    assert_eq!(queue.job(id).map(|job| &job.state), Some(&JobState::Paused));
    assert!(queue.keep_flag(id).is_some_and(|keep| keep.load(Ordering::SeqCst)), "its folder is kept");
    assert_eq!(queue.running(), 0);
}

#[test]
fn resuming_queues_the_job_again_and_lets_its_folder_go() {
    let mut queue = Downloads::default();
    let id = queue.enqueue(request("Resumed"));
    queue.start(id, handle());
    queue.pause(id);

    queue.resume(id);
    assert_eq!(queue.job(id).map(|job| &job.state), Some(&JobState::Queued));
    assert!(queue.keep_flag(id).is_some_and(|keep| !keep.load(Ordering::SeqCst)));
    assert_eq!(queue.next_start(), Some(id));
}

#[test]
fn cancelling_a_paused_job_returns_the_folder_to_remove() {
    let mut queue = Downloads::default();
    let id = queue.enqueue(request("Cancelled"));
    queue.start(id, handle());
    queue.pause(id);

    let folder = queue.cancel(id);
    assert_eq!(folder, Some(LauncherPaths::default().instance("Cancelled")));
    assert!(queue.job(id).is_none());
}

#[test]
fn cancelling_a_queued_job_has_no_folder_to_remove() {
    let mut queue = Downloads::default();
    let id = queue.enqueue(request("Waiting"));
    assert_eq!(queue.cancel(id), None);
    assert!(queue.jobs().is_empty());
}

#[test]
fn the_badge_counts_the_installs_that_have_not_ended() {
    let mut queue = Downloads::default();
    let done = queue.enqueue(request("Done"));
    queue.enqueue(request("Queued"));
    let failed = queue.enqueue(request("Failed"));
    queue.finish(done, Ok(()));
    queue.finish(failed, Err("the network dropped".to_owned()));

    assert_eq!(queue.badge(), 1);
}

#[test]
fn clearing_completed_drops_only_the_installs_that_ended() {
    let mut queue = Downloads::default();
    let done = queue.enqueue(request("Done"));
    let waiting = queue.enqueue(request("Waiting"));
    queue.finish(done, Ok(()));

    queue.clear_completed();
    assert_eq!(queue.jobs().iter().map(|job| job.id).collect::<Vec<_>>(), [waiting]);
}

#[test]
fn a_run_keeps_the_most_bytes_it_reported_and_hands_them_over_once() {
    let mut queue = Downloads::default();
    let id = queue.enqueue(request("Bytes"));
    queue.start(id, handle());
    queue.record(id, report(0.2, 100, 1_000, 50));
    queue.record(id, report(0.4, 60, 1_000, 40));

    assert_eq!(queue.take_bytes(id), 100, "the bytes only grow within a run");
    assert_eq!(queue.take_bytes(id), 0, "a second take finds nothing left");
}

#[test]
fn the_summary_adds_the_running_installs_up() {
    let mut queue = Downloads::default();
    let first = queue.enqueue(request("First"));
    let second = queue.enqueue(request("Second"));
    queue.start(first, handle());
    queue.start(second, handle());
    queue.record(first, report(0.5, 400, 1_000, 200));
    queue.record(second, report(0.25, 100, 500, 100));

    let summary = queue.summary();
    assert_eq!(summary.fraction, Some(0.375), "the mean of the two fractions");
    assert_eq!(summary.per_second, 300);
    assert_eq!(summary.remaining_bytes, 600 + 400);
    assert_eq!(summary.eta_seconds(), Some(3));
}

#[test]
fn the_summary_knows_no_rate_before_a_download_reports_one() {
    let queue = Downloads::default();
    let summary = queue.summary();
    assert_eq!(summary.fraction, None);
    assert_eq!(summary.eta_seconds(), None);
}

#[test]
fn the_library_lists_each_install_that_has_not_ended_as_its_instance() {
    let mut queue = Downloads::default();
    let waiting = queue.enqueue(request("Waiting"));
    let done = queue.enqueue(request("Done"));
    queue.finish(done, Ok(()));

    let pending = queue.pending();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].instance.id, "id-Waiting");
    assert_eq!(pending[0].instance.name, "Waiting");
    assert_eq!(pending[0].state, JobState::Queued);
    assert_eq!(queue.job_for_instance("id-Waiting"), Some(waiting));
    assert_eq!(queue.job_for_instance("id-Done"), None, "an install that ended is no longer cancellable");
}

#[test]
fn the_names_of_unfinished_installs_are_reserved() {
    let mut queue = Downloads::default();
    queue.enqueue(request("Reserved"));
    let done = queue.enqueue(request("Released"));
    queue.finish(done, Ok(()));

    assert_eq!(queue.reserved_directories(), ["Reserved"]);
}

#[test]
fn take_all_bytes_sums_every_run_and_resets_them() {
    let mut queue = Downloads::default();
    let first = queue.enqueue(request("First"));
    let second = queue.enqueue(request("Second"));
    queue.start(first, handle());
    queue.start(second, handle());
    queue.record(first, report(0.1, 30, 100, 10));
    queue.record(second, report(0.1, 70, 100, 10));

    assert_eq!(queue.take_all_bytes(), 100);
    assert_eq!(queue.take_all_bytes(), 0);
}
