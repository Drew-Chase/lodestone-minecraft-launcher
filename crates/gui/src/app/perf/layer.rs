//! iced's debug logs describe its renderer's work: an SVG rasterized at a size, an atlas entry allocated
//! or removed, an image allocation dropped. This layer counts them through its own filter, so they are
//! counted even where the file logs leave them out.

use std::fmt::Debug;

use tracing::field::{Field, Visit};
use tracing::{Event, Metadata, Subscriber};
use tracing_subscriber::Layer;
use tracing_subscriber::filter::filter_fn;
use tracing_subscriber::layer::Context;
use tracing_subscriber::registry::LookupSpan;

use super::{recorder, trace};

/// The target prefix of the iced events this layer counts.
const TARGET: &str = "iced_wgpu::image";

/// The layer, boxed over any subscriber and filtered to iced's image events.
pub fn boxed<S>() -> Box<dyn Layer<S> + Send + Sync>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    let filter = filter_fn(|meta: &Metadata<'_>| meta.target().starts_with(TARGET));
    Box::new(<IcedEvents as Layer<S>>::with_filter(IcedEvents, filter))
}

/// What one iced log line reports.
#[derive(Debug, PartialEq, Eq)]
enum IcedEvent {
    /// An SVG rasterized at `pixels` (its width times its height).
    SvgRaster { pixels: u64 },
    /// An atlas entry allocated.
    AtlasAlloc,
    /// An atlas entry removed.
    AtlasEvict,
    /// An image allocation dropped.
    ImageDrop,
}

impl IcedEvent {
    fn record(self) {
        match self {
            IcedEvent::SvgRaster { pixels } => {
                recorder::record_count("iced.svg", pixels);
                trace::instant("iced.svg", pixels);
            }
            IcedEvent::AtlasAlloc => recorder::record_count("iced.atlas.alloc", 1),
            IcedEvent::AtlasEvict => recorder::record_count("iced.atlas.evict", 1),
            IcedEvent::ImageDrop => recorder::record_count("iced.image.drop", 1),
        }
    }
}

/// Counts iced's rasters and atlas traffic as they are logged.
struct IcedEvents;

impl<S: Subscriber> Layer<S> for IcedEvents {
    fn on_event(&self, event: &Event<'_>, _context: Context<'_, S>) {
        let mut message = Message::default();
        event.record(&mut message);
        if let Some(iced) = classify(&message.0) {
            iced.record();
        }
    }
}

/// The text of a log event's `message` field.
#[derive(Default)]
struct Message(String);

impl Visit for Message {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.0 = value.to_owned();
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        if field.name() == "message" {
            self.0 = format!("{value:?}");
        }
    }
}

/// Which iced event a log message reports, if it is one this layer counts.
fn classify(message: &str) -> Option<IcedEvent> {
    if let Some(rest) = message.strip_prefix("allocating ") {
        return raster_pixels(rest).map(|pixels| IcedEvent::SvgRaster { pixels });
    }
    if message.starts_with("Allocated atlas entry") {
        Some(IcedEvent::AtlasAlloc)
    } else if message.starts_with("Removing atlas entry") {
        Some(IcedEvent::AtlasEvict)
    } else if message.starts_with("Dropping image allocation") {
        Some(IcedEvent::ImageDrop)
    } else {
        None
    }
}

/// The pixels of the size in an `allocating <id> WxH` message, given the text after `allocating `.
fn raster_pixels(rest: &str) -> Option<u64> {
    let size = rest.rsplit(' ').next()?;
    let (width, height) = size.split_once('x')?;
    let width: u64 = width.parse().ok()?;
    let height: u64 = height.parse().ok()?;
    width.checked_mul(height)
}

#[cfg(test)]
#[path = "../../../tests/app/perf/layer.rs"]
mod tests;
