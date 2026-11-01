//! # lodestone_export
//!
//! Writes an instance out as a package other people can open: a Modrinth pack (`.mrpack`), a CurseForge
//! pack, a plain zip, a Lodestone package (`.lodestone`), and the portable launcher, which is a copy of the
//! running launcher with a Lodestone package appended ([`portable`]).
//!
//! The crate reads the instance folder and writes files. The caller resolves each file's links and passes
//! them in as [`link::Entry`] values, so the writers only route and serialize. Nothing here touches the
//! network or iced, so every format can be checked from files on disk.
//!
//! Writers check for cancellation between files and report progress through [`progress::Control`]. A
//! cancelled write returns [`Error::Cancelled`] and leaves a partial file for the caller to remove.

mod error;
pub mod format;
pub mod hash;
pub mod link;
pub mod plan;
pub mod portable;
pub mod progress;

#[cfg(test)]
#[path = "../tests/support.rs"]
pub(crate) mod support;

pub use error::{Error, Result};
