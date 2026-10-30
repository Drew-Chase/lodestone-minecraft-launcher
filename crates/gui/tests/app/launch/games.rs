use iced::Task;

use super::*;

/// The abort handle of a task that does nothing, for a preparation the test does not run.
fn handle() -> iced::task::Handle {
    let (_task, handle) = Task::<()>::none().abortable();
    handle
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
    games.kill("aether");
    assert_eq!(games.status("aether"), LaunchStatus::Idle);
    assert!(
        !games.finish_preparing("aether", serial),
        "an aborted preparation has nothing left to finish"
    );
}

#[test]
fn an_instance_with_no_game_has_no_console_and_no_latest() {
    let games = Games::default();
    assert!(games.console("aether").is_none());
    assert!(games.latest_running().is_none());
    assert_eq!(games.status("aether"), LaunchStatus::Idle);
}

#[test]
fn a_restart_mark_is_taken_once() {
    let mut games = Games::default();
    games.mark_restart("aether");
    assert!(games.take_restart("aether"));
    assert!(!games.take_restart("aether"));
}

#[test]
fn nothing_is_marked_as_stopped_without_a_game() {
    let mut games = Games::default();
    games.kill("aether");
    assert!(!games.take_killed("aether"), "only a running game can be stopped by the user");
}
