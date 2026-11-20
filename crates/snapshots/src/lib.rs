//! # lodestone_snapshots
//!
//! The mods an instance held at one moment, written to `.lodestone/snapshots/` so they can be put back.
//!
//! A snapshot is a JSON file, compressed with xz, that names each platform mod by its project id and release id. A mod
//! with no platform record is kept as a jar in a zip beside the JSON, and the JSON lists it with its SHA-1. [`store`]
//! writes, lists, reads, prunes and restores these files, and [`plan`] works out what a rollback would change, from a
//! snapshot and the mods the instance holds now. Nothing here touches the network or the GUI, so every part is checked
//! from files on disk.

mod error;
pub mod hash;
pub mod model;
pub mod naming;
pub mod plan;
pub mod store;

#[cfg(test)]
#[path = "../tests/support.rs"]
pub(crate) mod support;

pub use error::{Error, Result};
pub use model::{Entry, Kind, LocalJar, Platform, Snapshot};
