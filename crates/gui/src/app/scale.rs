//! The window's scale factor, for the places that size a picture to the box it is drawn in. The root keeps it up to date from
//! the window, and the picture loaders read it off the UI thread, so it is one atomic, as the active language is.

use std::sync::atomic::{AtomicU32, Ordering};

static FACTOR: AtomicU32 = AtomicU32::new(f32::to_bits(1.0));

/// Records the window's scale factor. A value that is not a positive number is ignored.
pub fn set(factor: f32) {
    if factor.is_finite() && factor > 0.0 {
        FACTOR.store(factor.to_bits(), Ordering::Relaxed);
    }
}

/// The window's scale factor: 1 until the window reports one.
pub fn factor() -> f32 {
    f32::from_bits(FACTOR.load(Ordering::Relaxed))
}

/// The side, in device pixels, of a square drawn `logical` pixels wide at the window's scale factor.
pub fn device_px(logical: f32) -> u32 {
    (logical * factor()).round().max(1.0) as u32
}

#[cfg(test)]
#[path = "../../tests/app/scale.rs"]
mod tests;
