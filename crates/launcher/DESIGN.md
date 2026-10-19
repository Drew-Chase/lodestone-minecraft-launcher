# Lodestone Launch — Design

Design specification for `lodestone_launch` (`crates/launcher`), the workspace's headless
launch module. This document is the implementation contract: module layout, public API,
semantics, and the boundaries that keep the crate standalone.

The short-form contract lives in [README.md](README.md). Where the two disagree, this
document is more specific and README stays the summary.

---

## 1. Role and architectural position

`lodestone_launch` turns a validated launch description into a running Minecraft client
process, and supervises that process until exit.

```
loaders / platforms / auth  ──▶  (gui adapter code)  ──▶  Launcher  ──▶  java process
                                                        (this crate)
```

### Hard rules

- **Standalone leaf crate.** Only `crates/gui` links to workspace crates. This crate has
  **zero** path dependencies on siblings and never will. Its types are the boundary that
  gui's adapter code maps loader/auth output *into*.
- **Dependency budget:** `std` + `thiserror` + `tracing` (both from the workspace
  dependency table). No serde, no reqwest, no piston-mc, no tokio, no iced, ever.
- **Headless and dumb about Minecraft, smart about processes.** It does not know what a
  version manifest is; it knows how to assemble a Java command line correctly on every
  desktop platform and how to keep a child process observable.

### What this crate does

- Accept and validate a launch description (Java path, directories, classpath, args, env)
- Expand environmental argument placeholders (`${classpath}`, `${library_directory}`, …)
- Assemble the final `java` command line with platform-correct conventions
- Spawn the JVM, drain its output streams, and report log lines and exit status
- Support kill / wait / poll on the running game

### What this crate does NOT do

- Download Java or Minecraft (gui + `piston-mc`)
- Parse Mojang version manifests or evaluate `rules` conditionals (loader crates filter
  *before* handing args over)
- Extract natives (installer/loader territory; this crate only points
  `-Djava.library.path` at the resulting directory)
- Substitute authentication placeholders (`${auth_player_name}`, `${auth_access_token}`,
  …) — gui composes those from session data
- Create directories, write files, or perform any other pre-launch side effects
- Block a UI thread by design — spawn is cheap; waiting happens through events

---

## 2. Module layout

```
crates/launcher/
├── Cargo.toml              # name = "lodestone_launch"; thiserror + tracing only
├── README.md               # short contract
├── DESIGN.md               # this document
├── src/
│   ├── lib.rs              # crate docs (restates §1); mod decls; pub use re-exports
│   ├── config.rs           # Launcher + LauncherBuilder          (public)
│   ├── command.rs          # Launcher → std::process::Command    (private, pure)
│   ├── process.rs          # launch() + GameProcess + drain      (private impl, public handle)
│   ├── events.rs           # LaunchEvent + LogStream             (public)
│   └── error.rs            # LaunchError                         (public)
├── examples/
│   └── vanilla.rs          # manual smoke launch against a real instance
└── tests/
    ├── common/mod.rs       # per-OS "fake java" stub helper
    ├── config.rs           # builder + validation behavior
    ├── command.rs          # arg order, classpath separator, ${...} expansion
    └── process.rs          # spawn / log events / exit / kill via the stub
```

### Split rationale: pure vs. side-effectful

- `command.rs` is a **pure function** from `Launcher` to a `std::process::Command`. All
  assembly logic (ordering, joining, expansion) lives here and is unit-testable with no
  process ever spawned.
- `process.rs` is the only module with effects: spawn, threads, pipes, kill. Platform-
  conditional code (`#[cfg(windows)]`) is confined to this module — there is deliberately
  no `windows.rs` / `macos.rs` / `linux.rs` split; the crate's platform surface is one
  creation flag wide unless the Job-Object decision in §6.2 changes that.

`events.rs` is kept separate from `process.rs` (it could fold in) because it is the piece
gui's `Subscription` code consumes most and its contract deserves isolation.

`lib.rs` re-exports the public surface and nothing else:

- `config::{Launcher, LauncherBuilder}`
- `events::{LaunchEvent, LogStream}`
- `process::GameProcess`
- `error::LaunchError`

Crate-level: document the §1 contract in `//!` docs, enable `#![deny(missing_docs)]` on
the public API (this API is the workspace's launch ABI — it must be documented), and
prefer `#![forbid(unsafe_code)]` (see §6.2 for the one known future exception).

---

## 3. Public API

The complete ABI gui codes against:

| Type | Module | Kind | Role |
|---|---|---|---|
| `Launcher` | config | struct | Validated, immutable launch description. `launch(&self) -> Result<GameProcess, LaunchError>` |
| `LauncherBuilder` | config | struct | Required-first construction; typed setters; `build() -> Result<Launcher, LaunchError>` validates eagerly |
| `GameProcess` | process | struct | Per-instance handle: `id()`, `try_wait()`, `wait()`, `kill()`, `take_events()` |
| `LaunchEvent` | events | enum | `Log { stream: LogStream, line: String }` · `Exited { status: ExitStatus }` |
| `LogStream` | events | enum | `Stdout` · `Stderr` |
| `LaunchError` | error | enum | thiserror; variants are user-presentable (gui surfaces them verbatim) |

Channel type is `std::sync::mpsc` — deliberately runtime-agnostic. Gui pumps it from a
worker thread inside `Subscription::run`; this crate never pulls in an async runtime.

### Threading contract with the GUI

- `launch()` performs validation re-checks, spawns, starts two drain threads, and returns
  immediately. It is cheap enough to call directly from an iced `update()` — no `Task`
  wrapping required for the spawn itself.
- Waiting belongs to the event channel, not to a blocking call in `update()`.
- `Launcher` is `Clone` + `Send` + `Sync` and `launch(&self)` takes a shared reference:
  launching the same config twice (two instances) is legal and stateless. No globals, no
  statics, no interior mutability.

---

## 4. `Launcher` fields

All paths are `PathBuf`; all argument lists are `Vec<String>`. Single-string argument
fields are forbidden — splitting a `String` on spaces breaks quoted args and paths with
spaces (`C:\Program Files\...`).

| Field | Type | Required | Notes |
|---|---|---|---|
| `java_path` | `PathBuf` | yes | Validated absolute, exists, is a file. No `PATH` lookup is performed. |
| `working_directory` | `PathBuf` | yes | Validated exists. Never created by this crate. |
| `library_directory` | `PathBuf` | yes | Feeds `${library_directory}`. |
| `natives_directory` | `Option<PathBuf>` | no | Drives `-Djava.library.path=...` and `${natives_directory}`. |
| `assets_directory` | `Option<PathBuf>` | no | Feeds `${assets_root}`. |
| `jvm_args` | `Vec<String>` | no | Emitted before `-cp`. |
| `main_class` | `String` | yes | Emitted immediately after `-cp`. |
| `classpath` | `Vec<PathBuf>` | yes* | Non-empty when `main_class` is set. Joined via `std::env::join_paths`. |
| `game_args` | `Vec<String>` | no | `${...}` placeholders expanded by `command.rs`. |
| `env` | `BTreeMap<String, OsString>` | no | BTreeMap for deterministic env ordering. |
| `launcher_name` | `String` | no | Feeds `${launcher_name}`. |
| `launcher_version` | `String` | no | Feeds `${launcher_version}`. |

### Builder

`Launcher::builder(java_path, working_directory)` — the two genuinely required fields are
constructor arguments; everything else is an optional typed setter (`impl Into<PathBuf>`,
`IntoIterator<Item = impl Into<String>>` for arg lists). `build()` performs all
validation in §4's table eagerly so misconfiguration surfaces at build time with the
offending path attached, not at spawn time inside a game session.

---

## 5. Command assembly semantics (`command.rs`)

Argument order emitted to the JVM:

1. `java_path` (program)
2. `jvm_args` (in order)
3. `-Djava.library.path=<natives_directory>` — only when set
4. `-cp <classpath joined>` — separator is `;` on Windows, `:` elsewhere, via
   `std::env::join_paths` (never hand-concatenated)
5. `main_class`
6. `game_args` (in order, placeholders expanded)

### Placeholder expansion

Scope is **environmental only**: `${natives_directory}`, `${library_directory}`,
`${assets_root}`, `${classpath}`, `${classpath_separator}`, `${launcher_name}`,
`${launcher_version}`. Auth placeholders are gui's job (§1).

- Unknown placeholders: `tracing::warn!` and pass through **untouched**. Loaders may add
  custom placeholders for their own args; strict erroring would make the crate refuse
  legitimate future input.
- Expansion operates on whole tokens (an argument that is exactly a placeholder), not on
  substring substitution inside arbitrary args — substring templating of game args is a
  loader concern.
- `${classpath}` expansion uses the already-joined string from step 4 — the entity that
  joins the classpath expands it, so separators can never disagree.

---

## 6. Process supervision semantics (`process.rs`)

### 6.1 Streams and events

- Two dedicated drain threads (stdout, stderr) start **before** `launch()` returns. This
  is mandatory: an undrained pipe fills (~64 KB) and the JVM blocks on write — the
  classic launcher deadlock. Nobody may call `wait()` without drains running.
- Each line is decoded with `from_utf8_lossy` (Windows console code pages are not
  guaranteed UTF-8) and sent as `LaunchEvent::Log`.
- When the process exits and both pipes hit EOF, the drain threads send
  `LaunchEvent::Exited { status }` and drop their senders; the receiver then reports
  `Disconnected`. That is the gui's "game is gone" signal.
- Channel is **bounded** (~1024) with **blocking send**: a stalled consumer applies
  backpressure to the game instead of ballooning launcher memory. Minecraft logs can be
  chatty; unbounded is a memory leak waiting for a slow UI frame.

### 6.2 Platform behavior

- **Windows:** set `CREATE_NO_WINDOW` (`0x0800_0000`) via `CommandExt::creation_flags` so
  no console window flashes behind the GUI. This is the crate's only `#[cfg(windows)]`
  code today. `#![forbid(unsafe_code)]` holds; the known future exception is Job Objects
  (below), which would require the `windows` crate as a target-gated dependency.
- **macOS / Linux:** nothing special. `working_directory` must exist pre-spawn (validated)
  — both `CreateProcess` and `posix_spawn` fail without it.
- **Game lifetime vs launcher lifetime:** `Drop` for `GameProcess` logs a warning and does
  **not** kill — the game may outlive the launcher. `kill()` is explicit. Whether the game
  should be tied to the launcher via a Windows Job Object (and breakaway) is a deferred
  product decision (§8).

### 6.3 `GameProcess`

- `id() -> u32`, `try_wait() -> Result<Option<ExitStatus>, LaunchError>`,
  `wait(self) -> Result<ExitStatus, LaunchError>`, `kill() -> Result<(), LaunchError>`
- `take_events() -> Receiver<LaunchEvent>` — consuming move, not a clone: one owner pumps
  the stream (std mpsc receivers are not `Clone`), which matches gui's one-subscription
  model exactly.

---

## 7. Errors (`error.rs`)

`LaunchError` via thiserror, one variant per failure mode, messages written to be shown
to a user verbatim:

| Variant | When | Context carried |
|---|---|---|
| `JavaNotFound` | build-time validation or pre-spawn re-check | the path |
| `WorkingDirectoryMissing` | validation | the path |
| `InvalidClasspath` | `join_paths` failure (entry contains the platform separator) | source error |
| `SpawnFailed` | `Command::spawn` failed | java path + source `io::Error` |
| `KillFailed` | `kill()` failed | source `io::Error` |
| `WaitFailed` | `wait()` / `try_wait()` failed | source `io::Error` |

Validation errors are attached to the offending path — never a bare "failed to launch".

---

## 8. Open decisions (ratify before or during implementation)

1. **Channel bound** — proposed 1024, blocking send. Any lower risks stalling chatty
   mods; any higher only delays memory pressure.
2. **Job Objects / breakaway on Windows** — does closing Lodestone kill Minecraft?
   Deferred; shapes `process.rs` and could relax `forbid(unsafe_code)`.
3. **Absolute `java_path` enforced** — proposed strict. piston-mc supplies managed JRE
   paths, so nothing legitimate needs `PATH` lookup.
4. **Drop semantics** — proposed warn-and-orphan. Alternative: `detach()` explicitness.
5. **Placeholder strictness** — proposed warn-and-pass-through.

## 9. Testing strategy

No Java installation and no Minecraft assets are required:

- `tests/common/mod.rs` builds a per-OS "fake java": `cmd /c echo %*` on Windows,
  `sh -c 'echo "$@"'` on Unix. Tests point `java_path` at the stub and assert on the
  echoed argv — covering argument ordering, classpath separators, and placeholder
  expansion end-to-end without a JVM.
- `tests/config.rs`: builder validation table (missing java, missing working dir, empty
  classpath with main class set, relative java path rejected).
- `tests/command.rs`: pure assembly assertions.
- `tests/process.rs`: stub-emitted stdout/stderr lines arrive as `LaunchEvent::Log`;
  process exit produces exactly one `Exited`; `kill()` on a long-running stub returns and
  the receiver eventually disconnects.
- `examples/vanilla.rs`: manual smoke launch against a real instance directory; its event
  pump loop is the reference implementation gui's `Subscription` should mirror.
