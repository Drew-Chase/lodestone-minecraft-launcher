//! Progress and cancellation for a write. A writer checks for cancellation before each file and reports
//! after each one, so a cancel takes effect at the next file boundary.

use std::sync::atomic::{AtomicBool, Ordering};

use crate::error::{Error, Result};

/// How far a write has got, in files and in bytes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Progress {
    /// Files written so far.
    pub files: usize,
    /// Files the write will write.
    pub total_files: usize,
    /// Bytes of source files read so far.
    pub bytes: u64,
    /// Bytes of source files the write will read.
    pub total_bytes: u64,
}

/// Cancellation and progress for one write.
pub struct Control<'a> {
    cancel: &'a AtomicBool,
    report: &'a mut dyn FnMut(Progress),
    progress: Progress,
}

impl<'a> Control<'a> {
    /// A control for a write of `total_files` files and `total_bytes` bytes. `report` is called after each file.
    pub fn new(cancel: &'a AtomicBool, total_files: usize, total_bytes: u64, report: &'a mut dyn FnMut(Progress)) -> Self {
        Self {
            cancel,
            report,
            progress: Progress {
                files: 0,
                total_files,
                bytes: 0,
                total_bytes,
            },
        }
    }

    /// Fails when the write was cancelled, before the next file starts.
    pub fn check(&self) -> Result<()> {
        if self.cancel.load(Ordering::SeqCst) {
            return Err(Error::Cancelled);
        }
        Ok(())
    }

    /// Records a finished file of `bytes` bytes and reports. Fails when the write was cancelled.
    pub fn advance(&mut self, bytes: u64) -> Result<()> {
        self.check()?;
        self.progress.files += 1;
        self.progress.bytes += bytes;
        (self.report)(self.progress);
        Ok(())
    }
}

#[cfg(test)]
#[path = "../tests/progress.rs"]
mod tests;
