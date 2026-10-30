//! The one-second report. A window of figures becomes lines for the console and `perf.log`, and the
//! numbers the F3 overlay shows.

use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::{Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use super::recorder::{self, Stat, Window};
use super::trace;

/// How often the report prints.
const INTERVAL: Duration = Duration::from_secs(1);

/// How many scopes the report lists, largest total time first.
const TOP: usize = 5;

/// The figures of one window. Rates are per second; timings are `Stat`s in nanoseconds.
#[derive(Debug, Clone, PartialEq)]
pub struct Figures {
    /// How long the window covered.
    pub seconds: f64,
    /// Frames drawn per second, counted by `frame.draw`.
    pub frames_per_s: f64,
    /// The frame's draw and layout times.
    pub draw: Stat,
    pub layout: Stat,
    /// Messages handled per second, with the update times folded together.
    pub updates_per_s: f64,
    pub update: Stat,
    /// The whole root view build.
    pub view: Stat,
    /// SVG rasters per second, and the pixels they covered per second.
    pub svg_per_s: f64,
    pub svg_pixels_per_s: f64,
    /// Atlas entries allocated and removed per second.
    pub atlas_allocs_per_s: f64,
    pub atlas_evicts_per_s: f64,
    /// Art cache hits and misses within the window.
    pub art_hits: u64,
    pub art_misses: u64,
    /// Log lines and trace events dropped since the launch.
    pub dropped_lines: u64,
    pub dropped_trace_events: u64,
    /// The scopes that took the most time, largest first. Scopes nest, so the totals overlap.
    pub slowest: Vec<(&'static str, Stat)>,
}

impl Figures {
    /// The figures of `window`, which covered `seconds`.
    pub fn new(window: &Window, seconds: f64, dropped_lines: u64, dropped_trace_events: u64) -> Self {
        let seconds = seconds.max(f64::MIN_POSITIVE);
        let per_second = |value: u64| value as f64 / seconds;
        let mut slowest: Vec<_> = window.timings().collect();
        slowest.sort_by_key(|(_, stat)| std::cmp::Reverse(stat.sum));
        slowest.truncate(TOP);
        let update = window.timings_under("update.");
        let svg = window.count("iced.svg");
        Self {
            seconds,
            frames_per_s: per_second(window.timing("frame.draw").count),
            draw: window.timing("frame.draw"),
            layout: window.timing("frame.layout"),
            updates_per_s: per_second(update.count),
            update,
            view: window.timing("view.root"),
            svg_per_s: per_second(svg.count),
            svg_pixels_per_s: per_second(svg.sum),
            atlas_allocs_per_s: per_second(window.count("iced.atlas.alloc").count),
            atlas_evicts_per_s: per_second(window.count("iced.atlas.evict").count),
            art_hits: window.count("art.hit").count,
            art_misses: window.count("art.miss").count,
            dropped_lines,
            dropped_trace_events,
            slowest,
        }
    }
}

/// The report as console lines: a headline, then the slowest scopes.
pub fn lines(figures: &Figures) -> Vec<String> {
    let mut lines = vec![format!(
        "perf {seconds:.1}s  frames {frames:.0}/s  draw {draw:.2} ms (max {draw_max:.2})  layout {layout:.2} ms  \
         updates {updates:.0}/s (max {update_max:.2} ms)  view {view:.2} ms (max {view_max:.2})  \
         svg {svg:.0}/s {svg_mpx:.2} Mpx/s  atlas +{allocs:.0} -{evicts:.0}/s  art {hits} hit {misses} miss  \
         dropped {lines} log lines, {trace} trace events",
        seconds = figures.seconds,
        frames = figures.frames_per_s,
        draw = ms(figures.draw.mean()),
        draw_max = ms(figures.draw.max),
        layout = ms(figures.layout.mean()),
        updates = figures.updates_per_s,
        update_max = ms(figures.update.max),
        view = ms(figures.view.mean()),
        view_max = ms(figures.view.max),
        svg = figures.svg_per_s,
        svg_mpx = figures.svg_pixels_per_s / 1_000_000.0,
        allocs = figures.atlas_allocs_per_s,
        evicts = figures.atlas_evicts_per_s,
        hits = figures.art_hits,
        misses = figures.art_misses,
        lines = figures.dropped_lines,
        trace = figures.dropped_trace_events,
    )];
    for (name, stat) in &figures.slowest {
        lines.push(format!(
            "    {name}: {} calls, mean {:.2} ms, max {:.2} ms",
            stat.count,
            ms(stat.mean()),
            ms(stat.max)
        ));
    }
    lines
}

/// Nanoseconds as milliseconds.
fn ms(nanos: u64) -> f64 {
    nanos as f64 / 1_000_000.0
}

/// The figures of the last window the reporter finished, for the overlay.
static LATEST: Mutex<Option<Figures>> = Mutex::new(None);

/// The last window's figures, or `None` until the first report.
pub fn latest() -> Option<Figures> {
    LATEST.lock().unwrap_or_else(PoisonError::into_inner).clone()
}

/// Starts the reporter. Every second it prints the window to the console and to `perf.log` in
/// `logs_dir`, and it keeps the figures for the overlay. `dropped_lines` reports the logging path's count.
pub fn start(logs_dir: &Path, dropped_lines: fn() -> u64) {
    let file = match File::create(logs_dir.join("perf.log")) {
        Ok(file) => file,
        Err(error) => {
            eprintln!("perf.log unavailable: {error}");
            return;
        }
    };
    let spawned = thread::Builder::new()
        .name("perf-report".into())
        .spawn(move || report_forever(file, dropped_lines));
    if let Err(error) = spawned {
        eprintln!("perf reporter unavailable: {error}");
    }
}

fn report_forever(mut file: File, dropped_lines: fn() -> u64) {
    let mut since = Instant::now();
    loop {
        thread::sleep(INTERVAL);
        let seconds = since.elapsed().as_secs_f64();
        since = Instant::now();

        let window = recorder::take();
        let figures = Figures::new(&window, seconds, dropped_lines(), trace::dropped());
        if !window.is_empty() {
            for line in lines(&figures) {
                println!("{line}");
                let _ = writeln!(file, "{line}");
            }
            let _ = file.flush();
        }
        *LATEST.lock().unwrap_or_else(PoisonError::into_inner) = Some(figures);
    }
}

#[cfg(test)]
#[path = "../../../tests/app/perf/summary.rs"]
mod tests;
