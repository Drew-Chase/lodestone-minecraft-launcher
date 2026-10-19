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
- Whole workspace is greenfield scaffolding: empty manifests, stub READMEs, `examples/vanilla.rs` and `impl Launcher` are placeholders.
- Core has no consumers wired yet; proposed contract: loaders build a resolved launch config -> core assembles command line + spawns java + supervises process.

## Tooling
- `justfile`: powershell shell on Windows (`set windows-shell`), bash otherwise; only `just --list` recipe so far.
