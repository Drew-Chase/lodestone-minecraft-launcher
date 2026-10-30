//! Measures where the launcher spends its time. It is built with the `perf` feature (`just perf`), and
//! without the feature every item here is an inert stub, so the call sites carry no `cfg`.
//!
//! - [`scope`] times a block: each message's update, each page's view, each file read.
//! - [`count`] counts an event, such as an art cache hit.
//! - [`frame`] wraps the root element, so layout, drawing and event handling are timed for the whole UI.
//! - [`layer`] counts iced's own SVG rasters and atlas traffic from its debug logs.
//!
//! Once a second [`init`]'s reporter prints the figures to the console and `logs/perf.log`. Every scope is
//! also written to a Chrome trace in `logs/`, and F3 toggles an overlay of the latest second ([`Hud`]).

mod kind;

pub use kind::message_kind;

#[cfg(feature = "perf")]
mod frame;
#[cfg(feature = "perf")]
mod hud;
#[cfg(feature = "perf")]
mod layer;
#[cfg(feature = "perf")]
mod recorder;
#[cfg(feature = "perf")]
mod summary;
#[cfg(feature = "perf")]
mod trace;

#[cfg(feature = "perf")]
pub use hud::Hud;

use std::path::Path;

use iced::{Element, Subscription};

/// Messages the overlay sends to itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Message {
    /// F3 was pressed: show or hide the overlay.
    Toggle,
    /// A second passed while the overlay is shown: redraw it with the latest figures.
    Refresh,
}

/// The F3 overlay's state. Without the `perf` feature it has nothing to show, and the private field
/// keeps it from being a unit struct, so it is built the same way as the real one.
#[cfg(not(feature = "perf"))]
#[derive(Debug, Default)]
pub struct Hud {
    _private: (),
}

#[cfg(not(feature = "perf"))]
impl Hud {
    #[inline(always)]
    pub fn update(&mut self, _message: Message) {}
}

/// Times the enclosing block under a name, from the moment the guard is made until it is dropped.
#[cfg(feature = "perf")]
pub struct Scope {
    name: &'static str,
    start: std::time::Instant,
}

#[cfg(feature = "perf")]
impl Drop for Scope {
    fn drop(&mut self) {
        let elapsed = self.start.elapsed();
        recorder::record_time(self.name, elapsed);
        trace::span(self.name, self.start, elapsed);
    }
}

/// Starts timing `name`. Keep the guard in a named binding (`let _scope = …`) so it lives to the end of
/// the block.
#[cfg(feature = "perf")]
pub fn scope(name: &'static str) -> Scope {
    Scope {
        name,
        start: std::time::Instant::now(),
    }
}

/// Times nothing: the guard has no state and no drop.
#[cfg(not(feature = "perf"))]
pub struct Scope;

#[cfg(not(feature = "perf"))]
#[inline(always)]
pub fn scope(_name: &'static str) -> Scope {
    Scope
}

/// Counts `value` units of `name`, such as one cache hit (`value` 1).
#[cfg(feature = "perf")]
pub fn count(name: &'static str, value: u64) {
    recorder::record_count(name, value);
}

/// Counts nothing.
#[cfg(not(feature = "perf"))]
#[inline(always)]
pub fn count(_name: &'static str, _value: u64) {}

/// Wraps the root element so the whole window's layout, drawing and event handling are timed.
#[cfg(feature = "perf")]
pub fn frame<'a, Message: 'static>(content: Element<'a, Message>) -> Element<'a, Message> {
    Element::new(frame::Frame::new(content))
}

/// Returns the root element unchanged.
#[cfg(not(feature = "perf"))]
#[inline(always)]
pub fn frame<'a, Message>(content: Element<'a, Message>) -> Element<'a, Message> {
    content
}

/// The overlay for the window: a panel of the latest second's figures while shown, else nothing.
#[cfg(feature = "perf")]
pub fn overlay<'a, M: 'a>(hud: &Hud) -> Element<'a, M> {
    hud::overlay(hud)
}

/// Nothing: the overlay does not exist without the feature.
#[cfg(not(feature = "perf"))]
pub fn overlay<'a, M: 'a>(_hud: &Hud) -> Element<'a, M> {
    iced::widget::Space::new().into()
}

/// The F3 key, and the one-second refresh while the overlay is shown.
#[cfg(feature = "perf")]
pub fn subscription(hud: &Hud) -> Subscription<Message> {
    hud::subscription(hud)
}

/// Nothing: the overlay does not listen without the feature.
#[cfg(not(feature = "perf"))]
pub fn subscription(_hud: &Hud) -> Subscription<Message> {
    Subscription::none()
}

/// The layer that counts iced's SVG rasters and atlas traffic, for the logging stack. `None` without the
/// feature.
#[cfg(feature = "perf")]
pub fn layer<S>() -> Option<Box<dyn tracing_subscriber::Layer<S> + Send + Sync>>
where
    S: tracing::Subscriber + for<'span> tracing_subscriber::registry::LookupSpan<'span>,
{
    Some(layer::boxed())
}

/// `None` without the feature: no layer is added.
#[cfg(not(feature = "perf"))]
pub fn layer<S>() -> Option<Box<dyn tracing_subscriber::Layer<S> + Send + Sync>>
where
    S: tracing::Subscriber + for<'span> tracing_subscriber::registry::LookupSpan<'span>,
{
    None
}

/// Starts the reporter and the Chrome trace in `logs_dir`. `dropped_lines` reports how many log lines the
/// logging path has dropped. Does nothing without the feature.
#[cfg(feature = "perf")]
pub fn init(logs_dir: &Path, dropped_lines: fn() -> u64) {
    summary::start(logs_dir, dropped_lines);
    trace::start(logs_dir);
}

/// Does nothing without the feature.
#[cfg(not(feature = "perf"))]
pub fn init(_logs_dir: &Path, _dropped_lines: fn() -> u64) {}

#[cfg(all(test, feature = "perf"))]
#[path = "../../../tests/app/perf/mod.rs"]
mod tests;
