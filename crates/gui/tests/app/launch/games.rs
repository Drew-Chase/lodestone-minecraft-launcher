use iced::Task;

use super::*;

/// The abort handle of a task that does nothing, for a preparation the test does not run.
fn handle() -> iced::task::Handle {
    let (_task, handle) = Task::<()>::none().abortable();
    handle
}

/// A record for a run some earlier launcher started. Its pid is one no process holds, so a test never
/// kills anything.
fn record(instance: &str) -> Record {
    Record {
        instance: instance.to_owned(),
        pid: u32::MAX - 1,
        started_at: 1_700_000_000,
        process_started_at: 1_700_000_000,
    }
}

/// The moment a run started `minutes` ago.
fn started_ago(minutes: i64) -> DateTime<Local> {
    Local::now() - chrono::TimeDelta::minutes(minutes)
}

#[test]
fn serials_never_repeat() {
    let mut games = Games::default();
    let first = games.next_serial();
    assert_ne!(first, games.next_serial());
}

#[test]
fn a_preparation_reports_its_progress_until_it_finishes() {
    let mut games = Games::default();
    let serial = games.next_serial();
    games.begin("aether", serial, handle());
    assert_eq!(games.status("aether"), LaunchStatus::Preparing(None));
    assert!(games.is_active("aether"));
    assert!(games.is_preparing("aether"));
    assert!(!games.is_running("aether"));

    games.set_progress("aether", serial, Some(0.5));
    assert_eq!(games.status("aether"), LaunchStatus::Preparing(Some(0.5)));

    assert!(!games.finish_preparing("aether", serial + 1), "a result from another launch is stale");
    assert!(games.finish_preparing("aether", serial));
    assert_eq!(games.status("aether"), LaunchStatus::Idle);
    assert!(!games.is_active("aether"));
}

#[test]
fn stopping_a_preparation_ends_it() {
    let mut games = Games::default();
    let serial = games.next_serial();
    games.begin("aether", serial, handle());
    assert!(games.kill_instance("aether").is_empty());
    assert_eq!(games.status("aether"), LaunchStatus::Idle);
    assert!(
        !games.finish_preparing("aether", serial),
        "an aborted preparation has nothing left to finish"
    );
}

#[test]
fn an_instance_with_no_run_has_no_runs_and_no_newest_run() {
    let games = Games::default();
    assert!(games.runs().is_empty());
    assert!(games.newest_run("aether").is_none());
    assert_eq!(games.status("aether"), LaunchStatus::Idle);
}

#[test]
fn an_attached_run_has_no_console_and_makes_its_instance_run() {
    let mut games = Games::default();
    let serial = games.attach(record("aether"), started_ago(5));
    assert!(games.is_running("aether"));
    assert!(games.is_active("aether"));
    assert_eq!(games.status("aether"), LaunchStatus::Running);
    assert!(
        games.console(serial).is_none(),
        "an attached run's output went with the launcher that started it"
    );
}

#[test]
fn runs_are_listed_oldest_first_and_each_instance_has_a_newest_run() {
    let mut games = Games::default();
    let borealis = games.attach(record("borealis"), started_ago(5));
    let aether = games.attach(record("aether"), started_ago(10));
    assert_eq!(games.runs(), vec![(aether, "aether"), (borealis, "borealis")]);
    assert_eq!(games.newest_run("borealis"), Some(borealis));
    assert_eq!(games.newest_run("aether"), Some(aether));
}

#[test]
fn an_instance_can_run_twice_and_stays_running_until_its_last_run_ends() {
    let mut games = Games::default();
    let first = games.attach(record("aether"), started_ago(10));
    let second = games.attach(record("aether"), started_ago(5));
    assert_eq!(games.newest_run("aether"), Some(second));

    assert!(games.take_running(first).is_some());
    assert!(games.is_running("aether"), "the second run still goes");
    assert!(games.take_running(second).is_some());
    assert!(!games.is_running("aether"));
}

#[test]
fn killing_an_instance_stops_every_run_of_it_and_leaves_the_others() {
    let mut games = Games::default();
    let first = games.attach(record("aether"), started_ago(10));
    let second = games.attach(record("aether"), started_ago(5));
    let borealis = games.attach(record("borealis"), started_ago(3));

    let records = games.kill_instance("aether");
    assert_eq!(records, vec![record("aether"), record("aether")]);
    assert!(games.take_killed(first));
    assert!(games.take_killed(second));
    assert!(!games.take_killed(borealis), "the other instance was not stopped");
    assert!(games.is_running("borealis"));
}

#[test]
fn killing_one_run_leaves_the_others_running() {
    let mut games = Games::default();
    let first = games.attach(record("aether"), started_ago(10));
    let second = games.attach(record("aether"), started_ago(5));

    assert_eq!(games.kill_run(first), Some(record("aether")));
    assert!(games.take_killed(first));
    assert!(games.is_running("aether"));
    assert_eq!(games.instance_of(second), Some("aether"));
}

#[test]
fn restarting_an_instance_marks_every_run_of_it_once() {
    let mut games = Games::default();
    let first = games.attach(record("aether"), started_ago(10));
    let second = games.attach(record("aether"), started_ago(5));
    let borealis = games.attach(record("borealis"), started_ago(3));

    games.mark_restart("aether");
    assert!(games.take_restart(first));
    assert!(games.take_restart(second));
    assert!(!games.take_restart(first), "a mark is taken once");
    assert!(!games.take_restart(borealis), "the other instance was not marked");
}

#[test]
fn stopping_a_run_that_is_not_there_changes_nothing() {
    let mut games = Games::default();
    assert_eq!(games.kill_run(999), None);
    assert!(!games.take_killed(999));
}

#[test]
fn a_launch_waiting_for_a_session_shows_as_preparing_until_it_is_stopped() {
    let mut games = Games::default();
    games.wait_for_session("aether", 3, None);
    assert_eq!(games.status("aether"), LaunchStatus::Preparing(None));
    assert!(games.is_active("aether"), "a second press must not start the same launch");
    assert!(games.is_preparing_with("aether", 3));

    games.kill_instance("aether");
    assert!(!games.is_preparing_with("aether", 3), "a stopped wait is not started later");
    assert_eq!(games.status("aether"), LaunchStatus::Idle);
}

#[test]
fn one_session_refresh_runs_at_a_time_and_its_waiters_come_back_with_it() {
    let mut games = Games::default();
    games.wait_for_session("aether", 1, None);
    assert!(games.start_refresh("token-a".to_string()));
    games.wait_for_session("borealis", 2, None);
    assert!(
        !games.start_refresh("token-b".to_string()),
        "a second launch shares the refresh in flight"
    );

    let (token, waiting) = games.finish_refresh();
    assert_eq!(token.as_deref(), Some("token-a"));
    assert_eq!(waiting, vec![("aether".to_string(), 1, None), ("borealis".to_string(), 2, None)]);
    assert!(
        games.start_refresh("token-c".to_string()),
        "the next refresh starts once this one has ended"
    );
}

#[test]
fn a_launch_waiting_for_a_session_keeps_the_world_it_quick_plays_into() {
    let mut games = Games::default();
    games.wait_for_session("aether", 5, Some("Aether Hub".to_string()));
    let (_, waiting) = games.finish_refresh();
    assert_eq!(waiting, vec![("aether".to_string(), 5, Some("Aether Hub".to_string()))]);
}

#[test]
fn ending_a_refresh_that_never_ran_hands_back_nothing() {
    let mut games = Games::default();
    assert_eq!(games.finish_refresh(), (None, Vec::new()));
}

#[test]
fn each_runs_panel_opens_and_shuts_on_its_own() {
    let mut open = RunsOpen::default();
    assert!(!open.is_open(RunsPlace::Sidebar));
    open.toggle(RunsPlace::Sidebar);
    assert!(open.is_open(RunsPlace::Sidebar));
    assert!(!open.is_open(RunsPlace::Titlebar), "the title bar's panel is separate");
    open.toggle(RunsPlace::Sidebar);
    assert!(!open.is_open(RunsPlace::Sidebar));

    open.toggle(RunsPlace::Titlebar);
    open.dismiss(RunsPlace::Titlebar);
    assert!(!open.is_open(RunsPlace::Titlebar));
}
