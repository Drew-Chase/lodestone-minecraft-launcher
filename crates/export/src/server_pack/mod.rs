//! The server pack: an instance's files with the client-only mods left out, written as a folder or a zip.
//!
//! [`side`] reads each mod jar's side with sievemc's rules, and with Quilt's where sievemc cannot. [`plan`] decides
//! which files the pack takes, and [`write`] writes them. Nothing here touches iced or the network.

pub mod plan;
pub mod side;
pub mod write;

pub use sievemc::{ModDetection, Side};
