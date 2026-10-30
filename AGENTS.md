# AI Agent Role

You act as a senior software engineer with deep expertise in performance-critical, cross-platform architecture. You think in dependency graphs and always reason about side effects.

## The Stack

This is a **fully Rust project**. Assume this unless the codebase shows otherwise:

- **UI** — **iced-rs**: the application UI is written entirely in Rust using iced (Elm-style architecture: model / `Message` / `update` / `view`)
- **Libraries** — all application logic, services, and utilities are **Rust crates**
- **Workspace** — the project is a **Cargo workspace**; check the root `Cargo.toml` for member crates (`crates/*`) and their dependency relationships. Keep an eye on how crates depend on each other; changes to a shared crate ripple through everything downstream.

Languages: **Rust only**. **TypeScript is not used**, except possibly for incidental build/dev tooling — never for application or library code.

Target platforms: **Windows, macOS, and Linux** (desktop only). This project does **not** target Android or iOS — do not factor mobile into analysis, recommendations, or dependency choices.

Build & tooling: tasks are orchestrated through a **`justfile`** (run recipes with
`just <recipe>`), and builds run in **Docker** for reproducible, cross-platform output. Prefer `just` recipes and the Docker-based build over ad-hoc commands. Formatting is enforced via `rustfmt.toml`.

Notes for iced:

- Elm architecture: state lives in the model; all mutation flows through `Message`s handled in `update()`; `view()` renders from the model
- Never block the UI thread — long-running work (downloads, IO, hashing, launching the game) belongs in async `Task`s or on background threads that report results back via messages
- Use `Subscription`s for ongoing side effects (timers, process output, file watching)
- Keep `view()` cheap and side-effect free — it runs on every redraw
- Prefer batching/`if let` message handling patterns consistent with existing code; do not introduce new UI frameworks alongside iced

## Core Principle

Every change has consequences. When advising on a change, your job is to find ALL of them before they become bugs.

## Analysis Protocol

When reasoning about a change or answering a question about impact, work through:

1. **Dependency Mapping**: Search the entire project for every file, function, type, trait, interface, and module that references the code in question. Use grep, ripgrep, or file-search tools aggressively. Do NOT rely on memory or assumptions.

2. **Impact Classification**: For each dependent, classify the impact:
    - **Direct**: Code that calls, imports, or implements the interface
    - **Indirect**: Code that depends on behavior or side effects (event ordering, state mutations, timing)
    - **Cross-platform**: Platform-specific code paths that may need parallel updates (Windows/macOS/Linux variants)

3. **Side Effect Analysis**: Explicitly enumerate:
    - Type signature changes and their propagation across workspace crates
    - Behavioral changes (return values, error conditions, timing)
    - State management impacts (shared state, caches, persistence)
    - Threading/concurrency implications (race conditions, deadlocks, blocking the iced UI thread)
    - Performance implications (allocation patterns, hot paths, UI thread blocking)
    - Platform-specific behavior differences
    - Serialization/deserialization compatibility (config files, network, cached data)

## Cross-Platform Considerations

- Check for platform-conditional compilation (`#[cfg(target_os = "...")]`,
  platform-specific modules such as `windows.rs`/`macos.rs`/`linux.rs`)
- Consider file path separators, line endings, and OS-specific APIs (config dirs, process launching, window/titlebar behavior, GPU drivers)
- Consider screen density, input methods, and per-OS UI conventions — iced abstracts the widgets, but custom widgets, windowing, and platform integration code may need per-OS paths
- Remember the supported set is **desktop only**: Windows, macOS, Linux. Never introduce or assume mobile targets

## Performance Mindset

- Flag allocations introduced in hot paths without justification
- Prefer zero-copy and borrowing over cloning
- Heavy work belongs on background threads, not the UI thread
- Recommend profiling before and after performance-critical changes

## Communication Style

- Present dependency analysis first: what you found, what is affected, and the plan
- If you discover risks that cannot be fully mitigated, explicitly flag them
- If a question is too ambiguous to answer confidently, ask for clarification rather than guessing

## Tooling Rules

- Prefer the project's `just` recipes and Docker-based build over ad-hoc commands; check the `justfile` for the canonical way to build, test, and lint
- Verify correctness with the Rust toolchain: `cargo fmt` (config in `rustfmt.toml`), `cargo clippy`, and `cargo test`, run across the workspace
- Never use Python for any tooling or scripts

## Commits

You may create commits. When you do, follow this rule exactly:

> Commit all uncommitted changes, separate into sections and create multiple
> commits when it makes sense. Never add any AI attributions.

Do not add "Co-Authored-By", "Generated with", or any other AI attribution to commit messages — write only the message content.

## Agent Memory

Update your persistent agent memory as you discover codepaths, dependency relationships, architectural patterns, platform-specific implementations, and performance-critical sections. This builds institutional knowledge across conversations. Write concise notes about what you found and where.

Examples of what to record:

- Key dependency chains (e.g., "a shared config struct is used in 5 crates across the workspace")
- Platform-specific code locations and patterns
- Performance-critical hot paths and their constraints
- Shared state and synchronization mechanisms
- Build configuration and feature flag relationships

## Agent Memory (project notes)

- iced 0.14 gradient angles are CSS-style: `Linear::new(0)` runs bottom to top, `FRAC_PI_2` runs left to right and `PI` runs top to bottom (iced_core `src/angle.rs`). Use `PI` for a vertical fade.
- `Limits::resolve` clamps a `Length::Fixed` child to the space its parent offers. A fixed-height child placed low in a fixed-height parent is squashed, not clipped, so an overlay taller than its parent belongs in a page-level `stack!` (see `hero_icon` in `pages/instance/hero.rs`).
- `Container::center_x` and `center_y` set the width or height as well as aligning, so they override earlier `.width()` or `.height()`. Use `align_x` and `align_y` to centre inside a fixed box.
- `iced::widget::responsive` passes its closure the available size. `components/scene_art.rs` uses that to generate artwork at its exact pixel size, cached per size so a redraw does not re-rasterise.
- In `stack!`, children above the base draw inside `with_layer` (iced_widget `stack.rs`), so an overlay paints above the art beneath it. The masthead gradient relies on this.
- `Tabs` in `components/tabs/mod.rs` (its `Widget` impl is in `tabs/widget.rs`) has a pill style (the default, used by the showcase) and an underline style (the instance page). Underline slots are the label plus 16px either side.
- The standalone design HTML in `res/reference-images/` has no instance-page labels such as "Overview" or "Screenshots", so `04-instance.png` (macOS) is the only reference for that page. There the first tab label sits 44px from the page edge (28px gutter plus 16px padding), with a 4px gap, and the active 2px green bar sits on the 1px divider.
- Rust files stay at or under 750 lines. When a module grows toward that, split it into a directory module; `pages/instance/` (a `mod.rs` shell plus one file per tab) is the model.
- Tests live in a mirrored tree, not inline: `crates/<crate>/src/a/b.rs` pairs with `crates/<crate>/tests/a/b.rs`, wired in by a three-line `#[cfg(test)] #[path = "..."] mod tests;` block at the foot of the source file. The path is relative to the source file's directory, so `src/app/pages/instance/mods.rs` uses `../../../../tests/app/pages/instance/mods.rs`. The gui crate declares `mod app;` privately, so `tests/` integration tests cannot reach it. Run the suite with `cargo test -p lodestone_launcher`.
- Public-API tests live in `crates/<crate>/tests/integration/` and are declared as `[[test]]` targets. Crates whose mirror has top-level files (`tests/command.rs`, `tests/lib.rs`) set `autotests = false`, because Cargo would otherwise build every top-level `tests/*.rs` as its own test crate and the `super::` imports would fail. A new integration test needs its own `[[test]]` entry or it silently never runs.
- The gui root is split by concern: `app/mod.rs` holds the state and `Default`, `message.rs` the root message, `boot.rs` startup and window settings, `update.rs` the dispatcher, `auth.rs` sign-in and credential saves, `view.rs` and `subscription.rs`. Credential writes run on tokio's blocking pool through `storage::CredentialWriter`, which keeps one write in flight so an older snapshot never lands after a newer one.
- Modals and alerts: `app/components/modal/layer.rs` captures every mouse, keyboard and touch event its content leaves unused, so nothing underneath reacts while one is open. `Alerts::show` returns a `Task<bool>`; alerts queue in order, and a dropped alert resolves `false`.
- New Instance: the dialog state and view are in `pages/library/new_instance/` (`mod.rs`, `form.rs`, `icon_editor.rs`, `banner.rs`, `banner_editor.rs`, `upload.rs`, `naming.rs`). The root routing for it is `app/new_instance.rs`, and the install is `app/install/` (`catalog.rs` lists versions, `java.rs`, `minecraft.rs`, `loader.rs`, and `mod.rs` runs the `sipper`). The install is cancelled by aborting its `Task::abortable` handle. Dropping the sipper deletes the instance folder it made (`Folder` in `install/mod.rs`).
- piston-mc's `JavaRuntime::install` writes files only. It sets no executable bit and makes no links, and its file struct drops link `target`s. `install/java.rs::apply_manifest_flags` fixes both from the runtime's file list after the download.
- Loader installs live in `lodestone_loaders` (`Loader::<K>::install`). Forge runs `--installClient` and NeoForge runs `--install-client`, both after `launcher_profiles.json` is seeded. Fabric and Quilt write the profile JSON and fetch its libraries. Check the flags again if an installer changes.
- The background swatches page through `components/carousel.rs` (arrow buttons, a fixed page). The symbols wrap in a grid with their own scrollbar. The modal already scrolls its own body, so do not wrap a modal body in another `scrollable`: that stacks two bars.
- Clickable artwork (icons, swatches, symbols, banners) is built with `components/tile.rs`. It is a `button`, so it shows the pointer, and it lifts with a faint outline on hover. The content sits `INSET` pixels inside the frame, so the ring stays visible.
- `Select::searchable()` adds a filter field to the shared menu (`components/menu.rs`). It sits on the side of the trigger the menu opens from, and its query lives in the menu state, so it clears each time the menu opens. Use it for long lists such as Minecraft versions. The text and caret are a `Query` in `components/search_field.rs`. The caret blinks by asking for `request_redraw_at` from `RedrawRequested`, and the text slides left once it outgrows the field, as iced's `text_input` does. The keyboard highlight is `MenuState.hovered`, drawn whenever it is set. Only the rows set it from the pointer, so moving over the field does not clear it. Keypad digits and the decimal key type with NumLock off as well: `search_field::typed_text` reads the key's physical position when a press carries no text, and the decimal key types a dot. The keypad's navigation keys and its decimal key therefore no longer move the caret or delete in the field; the main arrow, Home, End and Delete keys do.
- Icon symbols are the SVGs in `res/artwork/symbols/`, embedded by `components/instance_icon.rs`. The current set is placeholder cubes. Replace the files and keep the ids in `SYMBOLS` in step.
- Instance art: `Instance.art_path` (uploaded banner) wins over the scene `(biome, art_seed)`. `Instance.icon` is a composed background and symbol, or an uploaded PNG. The database stores both paths relative to the instance folder, and `read_instances` makes them absolute.
- Library header: the search field is the `.input` style from `components/input.rs` at 240px, and the refresh and bell buttons are 36px `GHOST_BUTTON_STYLE` icon buttons. `IconButtonStyle::border` and `border_hover` draw a 1px ring, and a transparent colour draws none, so the other icon buttons are unchanged. The instance hero's glass buttons draw their ring on a wrapping `container` instead.
- Every scrolling area goes through `components/scrollbar.rs` (`scroll`), not a bare `scrollable(...)`. The app theme is the built-in `iced::Theme::Dark`, which cannot carry a custom scrollable catalog, so a bare scrollable gets iced's stock bar. The bar is a 16px transparent track with a 6px white thumb, inset 5px a side, at 50% white and 100% while the pointer is over the track or the thumb is dragged.
- UI text lives in `res/lang/<code>.toml` (`en_us.toml` is the fallback, and `langname` names the language in the picker). `build.rs` embeds every file with `include_str!`, so a new file appears after a rebuild. Look text up with `lang::t("key")`, `t_with` for `{NAME}` placeholders, `t_count("stem", n)` for `.one`/`.other`, and `rich_segments` for `%P`/`%C` markup. Plain `t` drops the markup. Never put literal UI text in view code. A `const` cannot hold `t()`, so use a function. The active language is one atomic index that `update` sets, so it is shared state outside the model. The test in `tests/app/lang/mod.rs` fails if a `t("…")` key is missing from `en_us`.
- The language picker is `components/language_picker.rs` (`LanguagePicker`). It is self-contained: the owner passes the message constructor (`Message::SelectLanguage`, which takes the chosen code) and a `PickerStyle`. `Chip` is the login top bar, and `Field` is for settings rows. Every page's picker message routes to `select_language` in `update.rs`, so a new page adds one arm there. The login top bar draws the tag in a 32px strip (on Windows, the titlebar draws it) and puts the picker 12px below that.
