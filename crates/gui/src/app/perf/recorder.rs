//! The figures gathered in one window of time. Scopes and counters write into the current window, and
//! the reporter takes the window whole once a second (see `summary`).

use std::collections::BTreeMap;
use std::mem;
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Duration;

/// How often and how much one key fired within a window.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Stat {
    /// Times it fired.
    pub count: u64,
    /// What it added up to: nanoseconds for a timed scope, the counted units for a counter.
    pub sum: u64,
    /// The largest single value.
    pub max: u64,
}

impl Stat {
    fn add(&mut self, value: u64) {
        self.count += 1;
        self.sum += value;
        self.max = self.max.max(value);
    }

    /// The average value, or zero before anything fired.
    pub fn mean(&self) -> u64 {
        self.sum.checked_div(self.count).unwrap_or(0)
    }
}

/// The figures of one window, keyed by scope or counter name.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Window {
    timings: BTreeMap<&'static str, Stat>,
    counts: BTreeMap<&'static str, Stat>,
}

impl Window {
    pub const fn new() -> Self {
        Self {
            timings: BTreeMap::new(),
            counts: BTreeMap::new(),
        }
    }

    /// The timed scope `name`, or an empty `Stat` when it never ran.
    pub fn timing(&self, name: &str) -> Stat {
        self.timings.get(name).copied().unwrap_or_default()
    }

    /// The counter `name`, or an empty `Stat` when it never fired.
    pub fn count(&self, name: &str) -> Stat {
        self.counts.get(name).copied().unwrap_or_default()
    }

    /// Every timed scope, in name order.
    pub fn timings(&self) -> impl Iterator<Item = (&'static str, Stat)> + '_ {
        self.timings.iter().map(|(name, stat)| (*name, *stat))
    }

    /// Every timed scope whose name starts with `prefix`, folded into one `Stat`.
    pub fn timings_under(&self, prefix: &str) -> Stat {
        self.timings
            .iter()
            .filter(|(name, _)| name.starts_with(prefix))
            .fold(Stat::default(), |mut total, (_, stat)| {
                total.count += stat.count;
                total.sum += stat.sum;
                total.max = total.max.max(stat.max);
                total
            })
    }

    /// True when nothing was recorded.
    pub fn is_empty(&self) -> bool {
        self.timings.is_empty() && self.counts.is_empty()
    }

    pub(super) fn record_timing(&mut self, name: &'static str, nanos: u64) {
        self.timings.entry(name).or_default().add(nanos);
    }

    pub(super) fn record_count(&mut self, name: &'static str, value: u64) {
        self.counts.entry(name).or_default().add(value);
    }
}

/// The window being filled now.
static CURRENT: Mutex<Window> = Mutex::new(Window::new());

fn lock(window: &Mutex<Window>) -> MutexGuard<'_, Window> {
    window.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Adds one run of the timed scope `name` to the current window.
pub fn record_time(name: &'static str, elapsed: Duration) {
    let nanos = u64::try_from(elapsed.as_nanos()).unwrap_or(u64::MAX);
    lock(&CURRENT).record_timing(name, nanos);
}

/// Adds `value` units of the counter `name` to the current window.
pub fn record_count(name: &'static str, value: u64) {
    lock(&CURRENT).record_count(name, value);
}

/// Ends the current window and hands it over. Recording continues into a fresh one.
pub fn take() -> Window {
    take_from(&CURRENT)
}

fn take_from(window: &Mutex<Window>) -> Window {
    mem::take(&mut *lock(window))
}

#[cfg(test)]
#[path = "../../../tests/app/perf/recorder.rs"]
mod tests;
