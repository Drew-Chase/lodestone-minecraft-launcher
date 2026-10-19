//! # lodestone_launch
//!
//! Turns a validated launch description into a running Minecraft client process,
//! and supervises that process until exit.
//!
//! ```text
//! loaders / platforms / auth ──▶ (gui adapter code) ──▶ Launcher ──▶ java process
//!                                                     (this crate)
//! ```
//!
//! ## Hard rules
//!
//! - **Standalone leaf crate.** Zero path dependencies on workspace siblings;
//!   this crate's types are the boundary that gui's adapter code maps loader/auth
//!   output *into*. Only `crates/gui` links to workspace crates.
//! - **Dependency budget:** `std` + `thiserror` + `tracing`. No serde, no
//!   reqwest, no piston-mc, no tokio, no iced, ever.
//! - **Headless, and dumb about Minecraft while smart about processes.** It does
//!   not know what a version manifest is; it knows how to assemble a `java`
//!   command line correctly on every desktop platform and how to keep a child
//!   process observable.
//!
//! ## What this crate does
//!
//! - Accept and validate a launch description (Java path, directories,
//!   classpath, args, env)
//! - Expand environmental argument placeholders (`${classpath}`,
//!   `${library_directory}`, …)
//! - Assemble the final `java` command line with platform-correct conventions
//! - Spawn the JVM, drain its output streams, and report log lines and exit
//!   status
//! - Support kill / wait / poll on the running game
//!
//! ## What this crate does NOT do
//!
//! - Download Java or Minecraft (gui + `piston-mc`)
//! - Parse Mojang version manifests or evaluate `rules` conditionals (loader
//!   crates filter *before* handing args over)
//! - Extract natives (this crate only points `-Djava.library.path` at the
//!   resulting directory)
//! - Substitute authentication placeholders (`${auth_player_name}`,
//!   `${auth_access_token}`, …) — gui composes those from session data
//! - Create directories, write files, or perform any other pre-launch side
//!   effects
//! - Block a UI thread by design — spawn is cheap; waiting happens through
//!   events
//!
//! ## Threading contract with the GUI
//!
//! - [`Launcher::launch`] performs validation re-checks, spawns, starts two
//!   drain threads, and returns immediately. It is cheap enough to call directly
//!   from an iced `update()` — no `Task` wrapping required for the spawn itself.
//! - Waiting belongs to the event channel ([`GameProcess::take_events`]), not to
//!   a blocking call in `update()`.
//! - [`Launcher`] is [`Clone`] + [`Send`] + [`Sync`] and `launch` takes a shared
//!   reference: launching the same configuration twice (two instances) is legal
//!   and stateless. No globals, no statics, no interior mutability.
//!
//! The channel is `std::sync::mpsc` — deliberately runtime-agnostic. Gui pumps
//! it from a worker thread inside `Subscription::run`; this crate never pulls in
//! an async runtime. See `DESIGN.md` in the crate root for the full contract.

#![deny(missing_docs)]
#![forbid(unsafe_code)]

mod command;
mod config;
mod error;
mod events;
mod process;

pub use config::{Launcher, LauncherBuilder};
pub use error::LaunchError;
pub use events::{LaunchEvent, LogStream};
pub use process::GameProcess;
