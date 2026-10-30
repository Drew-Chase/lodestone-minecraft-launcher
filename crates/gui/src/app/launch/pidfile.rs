//! The pid files that let the launcher find its games again after a restart. Each game it starts gets
//! a small JSON file under the config directory, naming the instance, the game's process id and when
//! the launch happened. A later launcher reads them back, attaches to the games still running, and
//! deletes the files of the games that have gone.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sysinfo::{Pid, Process, ProcessRefreshKind, ProcessesToUpdate, System};
use tracing::warn;

/// How far apart, in seconds, the recorded and the observed start time of one process may be. The
/// process table reports whole seconds, so two reads of one process can fall either side of a boundary.
const START_TOLERANCE: u64 = 2;

/// One game's pid file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record {
    /// The id of the instance the game belongs to.
    pub instance: String,
    /// The game's process id.
    pub pid: u32,
    /// When the launch happened, in seconds since the Unix epoch.
    pub started_at: i64,
    /// When the operating system says the game's process started, in seconds since the Unix epoch.
    pub process_started_at: u64,
}

/// The folder the pid files live in: `<config dir>/lodestone/running`.
pub fn dir() -> Option<PathBuf> {
    let mut path = dirs::config_dir()?;
    path.push("lodestone");
    path.push("running");
    Some(path)
}

/// Records a game the launcher has just started. The process start time is read from the process
/// table now, so a later check can tell this process from another that reuses its id.
pub fn record_started(instance: &str, pid: u32, started_at: i64) -> Result<(), String> {
    let dir = dir().ok_or("the config directory is unknown")?;
    let process_started_at = Processes::new().start_time(pid).ok_or_else(|| format!("process {pid} is not running"))?;
    let record = Record {
        instance: instance.to_owned(),
        pid,
        started_at,
        process_started_at,
    };
    write_in(&dir, &record).map_err(|error| error.to_string())
}

/// Deletes the pid file of the game with process id `pid`, when it has one.
pub fn remove(pid: u32) {
    if let Some(dir) = dir() {
        remove_in(&dir, pid);
    }
}

/// Reads the pid files and returns the records of the games still running. Files that do not parse,
/// and files of games that have gone, are deleted.
pub fn load_live() -> Vec<Record> {
    match dir() {
        Some(dir) => load_live_in(&dir, &mut Processes::new()),
        None => Vec::new(),
    }
}

/// Kills the game a record names, when that process is still the game. Returns whether a kill was sent.
pub fn kill(record: &Record) -> bool {
    Processes::new().kill(record)
}

/// The file the game with process id `pid` keeps in `dir`. A pid is unique among the processes that
/// are running, so two runs of one instance never share a file.
fn file_in(dir: &Path, pid: u32) -> PathBuf {
    dir.join(format!("{pid}.pid"))
}

/// Writes `record` into `dir`, replacing the file its process had.
pub fn write_in(dir: &Path, record: &Record) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    let json = serde_json::to_vec_pretty(record).map_err(io::Error::other)?;
    fs::write(file_in(dir, record.pid), json)
}

/// Deletes the pid file of the game with process id `pid` in `dir`. A file that is already gone is
/// not an error.
pub fn remove_in(dir: &Path, pid: u32) {
    delete(&file_in(dir, pid));
}

/// Reads every pid file in `dir` and returns the records of the games still running. A file that
/// does not parse, or whose game has gone, is deleted. Other files in `dir` are left alone.
pub fn load_live_in(dir: &Path, processes: &mut Processes) -> Vec<Record> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut live = Vec::new();
    for path in entries.flatten().map(|entry| entry.path()) {
        if path.extension().and_then(|extension| extension.to_str()) != Some("pid") {
            continue;
        }
        let record = fs::read(&path).ok().and_then(|bytes| serde_json::from_slice::<Record>(&bytes).ok());
        match record {
            Some(record) if processes.is_game(&record) => live.push(record),
            _ => delete(&path),
        }
    }
    live
}

/// Deletes one file. A file that is already gone is not an error, and any other failure is logged.
fn delete(path: &Path) {
    if let Err(error) = fs::remove_file(path)
        && error.kind() != io::ErrorKind::NotFound
    {
        warn!(%error, path = %path.display(), "could not delete a game's pid file");
    }
}

/// Whether a process observed now is the one a record names: its start time is within the tolerance
/// of the recorded one, and the process table reported a start time at all.
pub fn same_process(recorded: u64, observed: u64) -> bool {
    observed != 0 && recorded.abs_diff(observed) <= START_TOLERANCE
}

/// The operating system's process table, asked about the games the launcher started.
#[derive(Default)]
pub struct Processes {
    system: System,
}

impl Processes {
    pub fn new() -> Self {
        Self::default()
    }

    /// The start time of process `pid` in seconds since the Unix epoch, or `None` when no such process exists.
    pub fn start_time(&mut self, pid: u32) -> Option<u64> {
        let pid = Pid::from_u32(pid);
        self.system
            .refresh_processes_specifics(ProcessesToUpdate::Some(&[pid]), true, ProcessRefreshKind::nothing());
        self.system.process(pid).map(Process::start_time)
    }

    /// Whether the process a record names is still the game it started.
    pub fn is_game(&mut self, record: &Record) -> bool {
        self.start_time(record.pid)
            .is_some_and(|observed| same_process(record.process_started_at, observed))
    }

    /// Kills the process a record names, when that process is still the game it started. Returns
    /// whether a kill was sent.
    pub fn kill(&mut self, record: &Record) -> bool {
        if !self.is_game(record) {
            return false;
        }
        self.system.process(Pid::from_u32(record.pid)).is_some_and(Process::kill)
    }
}

#[cfg(test)]
#[path = "../../../tests/app/launch/pidfile.rs"]
mod tests;
