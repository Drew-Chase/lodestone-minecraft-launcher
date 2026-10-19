# Lodestone Launcher — Agent Memory

## Workspace layout (Cargo workspace, resolver 3, edition 2024)
- Root `Cargo.toml`: members `crates/*`. Workspace deps: `tracing`, `thiserror 2.0`, `reqwest 0.13 (rustls, json)`, `serde 1.0 (derive)`, `serde_json`.
- `[profile.release]`: lto, codegen-units=1, panic=abort, opt-level=z, strip.

## Crate roles (from each crate's README)
- `launch` (`lodestone_launch`): headless launcher — takes args, calls java, starts the game. NOT: downloads, java install, auth. Dir: `crates/launcher`.
- `loaders` (`lodestone_loaders`): UNIFIED loader crate (2026-10-19; replaced fabric_loader/forge_loader/neoforge_loader/quilt_loader/vanilla_loader). One pub struct `Loader<K>` with zero-sized marker kinds (`Fabric`, `Forge`, `NeoForge`, `Quilt`, `Vanilla`) behind the sealed `LoaderKind` trait; 4 uniform functions: list_versions / versions_for_minecraft / download_client / extract_mod_info. Async (tokio). Wheel audit: piston-mc (vanilla manifest+client, `downloads` feature only), simple_download_utility (all downloads + progress), modcrawl --no-default-features (fabric/forge/neoforge/mcmod.info metadata + deps), local parser ONLY for quilt.mod.json. Design contract in crates/loaders/DESIGN.md.
- `platforms` (`lodestone_platforms`): UNIFIED platform crate (2026-10-19; replaced modrinth/curseforge/atlauncher/ftb/technic_platform). One pub struct `Platform<K>` with marker kinds (`Modrinth`, `CurseForge`, `AtLauncher`, `Ftb`, `Technic`) behind sealed `PlatformKind`; uniform surface: search / details / versions / download / parse_modpack, gated by per-kind `CONTENT_TYPES` capability consts (UnsupportedContentType/UnsupportedPlatform). Modrinth kind delegates to `ferinth` (BUT hand-rolled search — ferinth's ProjectType facet serializes via Debug = wrong case, 0 hits); CurseForge hand-rolled (furse has NO search; rejected mid-flight — deviation from ratified plan, documented in DESIGN.md §2); ATLauncher/FTB/Technic hand-rolled. FTB responses are FLAT (no data envelope); ATLauncher has no search/list API (UnsupportedPlatform) and no single-file download; Technic API is 401-gated live (documented unverified). parse_modpack: mrpack (modrinth.index.json, keys `fabric-loader`/`quilt-loader` hyphenated) + CF exports (manifest.json, projectID/fileID ids no urls). Design contract in crates/platforms/DESIGN.md.
- `curseforge_platform`, `modrinth_platform`, `technic_platform`, `ftb_platform`, `atlauncher_platform`: DELETED 2026-10-19, folded into `platforms`.
- `auth` (`lodestone_minecraft_authentication`): IMPLEMENTED (2026-10-19) — Microsoft device-code login (two-phase: begin shows url+code, finish polls) + silent refresh-token re-login + profile fetch + XErr→user-presentable error mapping (NoXboxAccount/ChildAccount/XboxLiveUnavailable/RegionUnsupported/AdultVerificationRequired). Wheel audit: `minecraft-msa-auth 0.5` (XBL→XSTS→MC chain, reqwest 0.13, takes OUR client) + `oauth2 5.0` (MS OAuth leg; default-features=false — its reqwest feature pins 0.12, we impl AsyncHttpClient for our 0.13 client in src/http.rs via newtype). Return-only sessions (gui persists tokens). client_id is a ctor arg (Azure app registration is gui's concern). Design contract in crates/auth/DESIGN.md.
- `gui` (`lodestone_launcher`): the ONLY binary crate. iced-rs UI. Calls `piston-mc` directly for assets/downloads/Java runtimes. Does NOT launch (core does) or auth (auth does).
- `aurora_leaf`: pins icon sets/fonts, re-exports iced types.

## External deps
- `piston-mc 0.1.5-beta` — WORKSPACE dep now (default-features=false, features=["downloads"]); gui uses it with features ["assets","downloads","java"] for assets/downloads/Java runtime management.
- Workspace deps added for loaders: `simple_download_utility 0.1.1` (downloads+progress; same crate piston-mc uses), `modcrawl 0.7.0` (mod metadata; 0.8.0 exists, unvetted), `zipcrawl 2.5.0` (jar reads for quilt parser), `tokio`, `anyhow`.
- Workspace deps added for auth: `oauth2 5.0.0` (default-features=false), `minecraft-msa-auth 0.5`, `http`.
- std API notes: `std::env::join_paths` on Windows (rustc 1.98) QUOTES `;`-bearing entries instead of rejecting (only `"` errors; Unix still rejects `:`) — see crates/launcher/DESIGN.md §7.

## Gotchas (hit during implementation)
- RPITIT (`impl Future + Send` in traits) + closures capturing method reference args → rustc "implementation of FnOnce is not general enough". Workaround used in loaders/platforms: closure-free bodies (for-loops, match) in the trait impls.
- oauth2 5.0.0: endpoint-state generics — `exchange_device_code` is non-Result only on `HasDeviceAuthUrl = EndpointSet` (via `set_device_authorization_url`); `RequestTokenError::ServerResponse` (NOT Response); device errors are `StandardErrorResponse<DeviceCodeErrorResponseType>` via `.error()`; `AccessToken::secret()` returns `&String`; `RefreshToken::new` must be bound to a local before `exchange_refresh_token(&...)` (temporary-lifetime E0716). Coherence E0117: can't impl AsyncHttpClient for &reqwest::Client → newtype in crates/auth/src/http.rs.
- ferinth: `ProjectType` facet serializes via Debug = wrong case → 0 hits; Modrinth search is hand-rolled (crates/platforms/src/modrinth.rs). ferinth's `Ferinth::new` takes 3 args (name, version, contact), builds its own client.
- Fabric meta v2 `/versions/loader/{mc}` returns HTTP 400 for unknown MC; quilt v3 loader entries have NO `stable` field (default true); fabric/quilt loader jar is referenced by `loader.maven`, NOT in launcherMeta.libraries; quilt libraries point at maven.fabricmc.net (per-artifact url is authoritative).
- Forge maven-metadata.json: { mc: [combined versions] }; artifacts installer.jar+universal.jar exist for all eras; client.jar is not published.
- NeoForge versions API returns { "isSnapshot": bool, "versions": [...] } (map, not array); prefix rule: MC 1.x.y→x.y., 1.x→x.0., snapshot→0.{mc}., post-1.x numeric→direct.
- Technic API is 401-gated for anonymous callers (verified 2026-10); binding kept, documented unverified in crates/platforms/DESIGN.md.

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
