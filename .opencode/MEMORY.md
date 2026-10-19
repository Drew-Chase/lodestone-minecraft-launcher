# Lodestone Launcher — Agent Memory

## Workspace layout (Cargo workspace, resolver 3, edition 2024)
- Root `Cargo.toml`: members `crates/*`. Workspace deps: `tracing`, `thiserror 2.0`, `reqwest 0.13 (rustls, json)`, `serde 1.0 (derive)`, `serde_json`.
- `[profile.release]`: lto, codegen-units=1, panic=abort, opt-level=z, strip.

## Crate roles (from each crate's README)
- `launch` (`lodestone_launch`): headless launcher — takes args, calls java, starts the game. NOT: downloads, java install, auth. Dir: `crates/launcher` (user renamed from `crates/core` themselves mid-session; package name `lodestone_launch`, design contract in `crates/launcher/DESIGN.md`).
- `vanilla_loader`, `fabric_loader`, `forge_loader`, `neoforge_loader`, `quilt_loader`: install/manage game versions & loaders per instance. Explicitly do NOT launch (see core).
- `curseforge_platform`, `modrinth_platform`, `technic_platform`, `ftb_platform`, `atlauncher_platform`: content platforms.
- `auth`: Microsoft/Xbox Live auth.
- `gui` (`lodestone_launcher`): the ONLY binary crate. iced-rs UI. Calls `piston-mc` directly for assets/downloads/Java runtimes. Does NOT launch (core does) or auth (auth does).
- `aurora_leaf`: pins icon sets/fonts, re-exports iced types.

## External deps
- `piston-mc 0.1.5-beta` (features: assets, downloads, java) — referenced in gui/Cargo.toml; used for Minecraft assets, downloads, Java runtime management.

## Architecture rules (user-declared)
- ONLY the gui crate links to other workspace crates (path deps). Every other crate is standalone and modular — no inter-crate deps, no shared "types" crate. Glue/adapters live in gui.
- Naming family: `lodestone_<subject>_<role>` with underscores (e.g. `lodestone_minecraft_authentication`, `lodestone_vanilla_loader`). gui binary crate = `lodestone_launcher`.

## Status (as of Oct 2026)
- `lodestone_launch` IMPLEMENTED (2026-10-19) per `crates/launcher/DESIGN.md`; §8 decisions ratified (see §8 of the doc): sync_channel(1024) blocking send, no Job Objects (warn-and-orphan Drop), strict absolute java_path, warn-and-pass-through placeholders. Still no consumers (gui not wired).
- Implementation shape: `config.rs` (Launcher w/ getters only + LauncherBuilder, eager validation in build()); `command.rs` pure `pub(crate) build_command` + whole-token `${...}` expansion, unit tests in-module (integration tests can't see pub(crate)); `process.rs` launch() + GameProcess: 2 drain threads (`read_until`+`from_utf8_lossy`, NOT BufRead::lines) + finalizer thread (Arc<Mutex<Child>>, emits exactly one Exited after both pipe EOFs, reaps child -> no unix zombie), bounded sync_channel backpressure, `#[cfg(windows)]` CREATE_NO_WINDOW only platform code; `events.rs`; `error.rs` (8 variants; `EmptyClasspath` added beyond the §7 table).
- std gotcha discovered: `std::env::join_paths` on Windows (rustc 1.98) no longer REJECTS `;`-bearing entries — it QUOTES them; only `"` entries error (Unix still rejects `:`). InvalidClasspath maps std's failure; see note added to DESIGN.md §7.
- Test stub gotchas (tests/common/mod.rs): Windows `.cmd` stub MUST start with `@echo off` or cmd echoes its prompt into stdout (empty first line); std quotes args with `;` for batch files (strip `"` in assertions); sleeper stub must redirect the grandchild's streams off the pipes (`ping >nul 2>&1` / `sleep >/dev/null 2>&1`) or killing the parent leaves the pipe open; spawning `.cmd` directly works (CreateProcess runs it via cmd.exe).

## Tooling
- `justfile`: powershell shell on Windows (`set windows-shell`), bash otherwise; only `just --list` recipe so far.
