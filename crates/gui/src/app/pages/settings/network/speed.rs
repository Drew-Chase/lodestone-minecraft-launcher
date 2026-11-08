//! The download speed the Network tab shows: the bytes received each second, kept for the last hour.

use std::collections::VecDeque;

/// How many one-second samples the meter keeps: an hour.
const HOUR: usize = 3600;
/// How many seconds make one point of the chart.
const MINUTE: usize = 60;

/// Samples the bytes received once a second, and keeps the last hour of samples.
#[derive(Debug, Default)]
pub struct Meter {
    /// The running total at the last sample.
    last_total: u64,
    /// The bytes received in each second, oldest first.
    samples: VecDeque<u64>,
}

impl Meter {
    /// Starts again from `total` without taking a sample, after the meter was not sampled for a while.
    pub fn resume(&mut self, total: u64) {
        self.last_total = total;
    }

    /// Takes a sample: the bytes received since the last one, given the running total of every byte received.
    pub fn tick(&mut self, total: u64) {
        let received = total.saturating_sub(self.last_total);
        self.last_total = total;
        if self.samples.len() == HOUR {
            self.samples.pop_front();
        }
        self.samples.push_back(received);
    }

    /// The average bytes per second over the samples taken so far, which is at most an hour.
    pub fn average(&self) -> f64 {
        if self.samples.is_empty() {
            return 0.0;
        }
        self.samples.iter().sum::<u64>() as f64 / self.samples.len() as f64
    }

    /// The average bytes per second of each minute, oldest first. The newest minute averages the seconds sampled so far.
    pub fn minutes(&self) -> Vec<f32> {
        let samples: Vec<u64> = self.samples.iter().copied().collect();
        samples
            .chunks(MINUTE)
            .map(|minute| minute.iter().sum::<u64>() as f32 / minute.len() as f32)
            .collect()
    }
}

#[cfg(test)]
#[path = "../../../../../tests/app/pages/settings/network/speed.rs"]
mod tests;
