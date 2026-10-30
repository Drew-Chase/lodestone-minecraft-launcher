use std::fs;
use std::path::PathBuf;

use super::*;

/// A pid no process can hold: above the largest one any operating system hands out.
const NO_SUCH_PID: u32 = i32::MAX as u32;

/// A folder under the system temp directory that no other test shares.
fn scratch_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lodestone-pidfile-{:016x}", fastrand::u64(..)));
    fs::create_dir_all(&dir).expect("the scratch folder can be created");
    dir
}

/// A record for this test process, which is certainly running and whose start time the table knows.
fn own_record(instance: &str) -> Record {
    let pid = std::process::id();
    let process_started_at = Processes::new().start_time(pid).expect("the test process is in the process table");
    Record {
        instance: instance.to_owned(),
        pid,
        started_at: 1_700_000_000,
        process_started_at,
    }
}

#[test]
fn a_process_counts_as_the_same_only_within_the_start_tolerance() {
    assert!(same_process(100, 100));
    assert!(same_process(100, 102));
    assert!(!same_process(100, 103));
    assert!(!same_process(100, 0), "a process without a reported start time is not the game");
}

#[test]
fn a_record_is_written_and_read_back_while_its_game_runs() {
    let dir = scratch_dir();
    let record = own_record("aether");
    write_in(&dir, &record).expect("the record is written");

    assert_eq!(load_live_in(&dir, &mut Processes::new()), vec![record]);
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn the_file_of_a_game_that_has_gone_is_deleted() {
    let dir = scratch_dir();
    let mut record = own_record("aether");
    record.pid = NO_SUCH_PID;
    write_in(&dir, &record).expect("the record is written");

    assert!(load_live_in(&dir, &mut Processes::new()).is_empty());
    assert!(!file_in(&dir, NO_SUCH_PID).exists());
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_reused_process_id_is_not_taken_for_the_game() {
    let dir = scratch_dir();
    let mut record = own_record("aether");
    record.process_started_at += 3_600;
    write_in(&dir, &record).expect("the record is written");

    assert!(load_live_in(&dir, &mut Processes::new()).is_empty());
    assert!(!file_in(&dir, std::process::id()).exists());
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_malformed_pid_file_is_deleted() {
    let dir = scratch_dir();
    fs::write(file_in(&dir, 4242), b"not json").expect("the file is written");

    assert!(load_live_in(&dir, &mut Processes::new()).is_empty());
    assert!(!file_in(&dir, 4242).exists());
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn files_that_are_not_pid_files_are_left_alone() {
    let dir = scratch_dir();
    fs::write(dir.join("notes.txt"), b"keep").expect("the file is written");

    load_live_in(&dir, &mut Processes::new());
    assert!(dir.join("notes.txt").exists());
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn removing_a_record_twice_is_not_an_error() {
    let dir = scratch_dir();
    write_in(&dir, &own_record("aether")).expect("the record is written");

    remove_in(&dir, std::process::id());
    remove_in(&dir, std::process::id());
    assert!(!file_in(&dir, std::process::id()).exists());
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn kill_refuses_a_record_whose_process_is_not_the_game() {
    let mut record = own_record("aether");
    record.process_started_at += 3_600;

    assert!(!Processes::new().kill(&record), "a process that only shares the id is never killed");
}

#[cfg(unix)]
#[test]
fn a_spawned_game_is_found_again_through_its_record_and_killed_through_it() {
    let dir = scratch_dir();
    let mut child = std::process::Command::new("sleep").arg("60").spawn().expect("sleep can be started");
    let pid = child.id();
    let process_started_at = Processes::new().start_time(pid).expect("the child is in the process table");
    let record = Record {
        instance: "aether".to_owned(),
        pid,
        started_at: 1_700_000_000,
        process_started_at,
    };
    write_in(&dir, &record).expect("the record is written");

    assert_eq!(load_live_in(&dir, &mut Processes::new()), vec![record.clone()]);
    assert!(Processes::new().kill(&record), "the record names the running child");
    assert!(!child.wait().expect("the killed child is reaped").success());
    assert!(load_live_in(&dir, &mut Processes::new()).is_empty());
    assert!(!file_in(&dir, pid).exists());
    fs::remove_dir_all(&dir).ok();
}
