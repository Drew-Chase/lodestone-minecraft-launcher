# Lodestone Loaders — Design

Design specification for `lodestone_loaders` (`crates/loaders`), the workspace's unified
loader module. This document is the implementation contract: module layout, public API,
semantics, and boundaries.

The short-form contract lives in [README.md](README.md). Where the two disagree, this
document is more specific.

---

## 1. Role and architectural position

`lodestone_loaders` resolves loader versions, downloads loader client libraries, and
extracts mod metadata — for all five "kinds" a Lodestone instance can be based on:

```
gui  ──▶  Loader<Fabric> / Loader<Forge> / Loader<NeoForge> / Loader<Quilt> / Loader<Vanilla>
          (this crate)
```

### Hard rules

- **Standalone leaf crate.** Zero path dependencies on workspace siblings. Only
  `crates/gui` links to workspace crates.
- **Runtime:** async, tokio-based. The API is `async fn`-first so iced `Task`s can
  await it directly.
- **Dumb about instances, smart about loader ecosystems.** It knows the meta APIs and
  maven layouts; it does not know what an instance is. `install` writes a loader's version
  JSON and libraries into the game folder it is handed, and runs installers; it never
  launches the game.
- It never launches the game (`lodestone_launch`) and never talks to content platforms.

## 2. Wheel audit (do not reinvent)

| Job | Crate used | Why |
|---|---|---|
| Vanilla version manifest + client jar | `piston-mc` (`downloads` feature) | Already the workspace's Mojang API layer |
| File downloads (with progress channels) | `simple_download_utility` | Same utility piston-mc uses; gui's Download Manager consumes its progress events |
| Mod metadata (fabric.mod.json, META-INF/mods.toml, META-INF/neoforge.mods.toml, legacy mcmod.info) + dependency analysis incl. jar-in-jar | `modcrawl` (no default features: no CLI) | Complete, tested parsers; upstream gap only for Quilt |
| quilt.mod.json | parsed here | No upstream crate covers it (yet) |
| Loader version meta APIs (fabricmc/quiltmc/neoforged/forge maven) | thin serde bindings here | No upstream crate exists for these APIs (searched crates.io, 2026-10) |

Whole-launcher libraries (mcvm, portablemc, crust_core, lyceris, lighty-*) were rejected:
they would drag a parallel launcher architecture into a workspace that already has
`piston-mc` + `lodestone_launch`.

## 3. Module layout

```
crates/loaders/
├── Cargo.toml              # lodestone_loaders
├── README.md               # short contract
├── DESIGN.md               # this document
├── src/
│   ├── lib.rs              # crate docs; LoaderKind trait; THE pub struct Loader<K>
│   ├── error.rs            # LoaderError
│   ├── meta.rs             # LoaderVersion, ModInfo, ModDependency, ModDependencyKind
│   ├── http.rs             # shared helpers: get_json, download_files, maven paths (private)
│   ├── install.rs          # shared install steps: version JSON, launcher profile, installer runs (private)
│   ├── fabric.rs           # pub struct Fabric   — meta.fabricmc.net/v2
│   ├── quilt.rs            # pub struct Quilt    — meta.quiltmc.org/v3 (fabric-shaped)
│   ├── neoforge.rs         # pub struct NeoForge — maven.neoforged.net versions API
│   ├── forge.rs            # pub struct Forge    — files.minecraftforge.net maven-metadata.json
│   ├── vanilla.rs          # pub struct Vanilla  — delegated to piston-mc
│   └── mod_info/           # mod metadata extraction
│       ├── mod.rs          # modcrawl dispatch + mapping into ModInfo
│       └── quilt.rs        # quilt.mod.json parser (local)
├── examples/               # live-network smoke tests (run manually)
│   ├── list_versions.rs
│   ├── download_client.rs
│   ├── install.rs
│   └── mod_info.rs
└── tests/
    └── mod_info.rs         # public extract_mod_info over fixture jars built in-test
```

Pure helpers (version-index parsing, mc-version prefix mapping, URL builders, quilt JSON
parsing) are unit-tested in `#[cfg(test)]` modules inside their source files — they are
`pub(crate)` and invisible to integration tests, mirroring the `lodestone_launch`
pattern.

## 4. Public API

| Type | Module | Kind | Role |
|---|---|---|---|
| `Loader<K>` | lib | struct | THE single behavioral struct. `new()`, `with_client(Client)`, and the four uniform functions below. |
| `LoaderKind` | lib | trait | Sealed; implemented only by the five markers here. Carries per-kind endpoints/parsers. |
| `Fabric` / `Forge` / `NeoForge` / `Quilt` / `Vanilla` | per-kind modules | marker structs | Zero-sized phantom kinds: `Loader::<Fabric>::new()`. |
| `LoaderVersion` | meta | struct | `version` (verbatim source id), `minecraft: Option<String>`, `stable`, `changelog_url: Option<String>`. |
| `ModInfo` | meta | struct | `loader` (actual detected format), `id`, `version`, `name`, `description`, `authors`, `license`, `homepage`, `dependencies`. |
| `ModDependency` / `ModDependencyKind` | meta | struct/enum | `id`, `kind` (Required/LoadBefore/Optional/Recommended/Suggested/Incompatible/Discouraged), `version_range` (raw string). |
| `DownloadProgress` | re-export | struct | `simple_download_utility`'s progress event, re-exported for gui. |
| `LoaderError` | error | enum | thiserror; messages user-presentable. |

### The four uniform functions

```rust
impl<K: LoaderKind> Loader<K> {
    pub async fn list_versions(&self) -> Result<Vec<LoaderVersion>, LoaderError>;
    pub async fn versions_for_minecraft(&self, minecraft: &str) -> Result<Vec<LoaderVersion>, LoaderError>;
    pub async fn download_client(
        &self,
        version: &LoaderVersion,
        into: impl AsRef<Path>,
        progress: Option<&Sender<DownloadProgress>>,
    ) -> Result<Vec<PathBuf>, LoaderError>;
    pub async fn install(
        &self,
        version: &LoaderVersion,
        game_dir: impl AsRef<Path>,
        java: impl AsRef<Path>,
        lines: Option<&Sender<String>>,
        progress: Option<&Sender<DownloadProgress>>,
    ) -> Result<InstalledVersion, LoaderError>;
    pub fn extract_mod_info(&self, jar: impl AsRef<Path>) -> Result<ModInfo, LoaderError>;
}
```

## 5. Per-kind semantics

| Kind | Version source | `download_client` fetches | Notes |
|---|---|---|---|
| `Fabric` | `meta.fabricmc.net/v2/versions/loader[/mc]` | `launcherMeta.libraries.client + common` maven artifacts (per-artifact base URL respected; default `maven.fabricmc.net`) | Unknown MC → HTTP 400 → `UnexpectedResponse` |
| `Quilt` | `meta.quiltmc.org/v3/versions/loader[/mc]` | same fabric-shaped launcherMeta (shared libs live on `maven.fabricmc.net` — per-artifact URLs are authoritative) | |
| `Forge` | `files.minecraftforge.net/.../maven-metadata.json` — map `{ mc: [combined versions] }` | `{V}/forge-{V}-installer.jar` + `{V}/forge-{V}-universal.jar` from `maven.minecraftforge.net` | `V` is the full combined string verbatim (e.g. `1.7.10-10.13.4.1614-1.7.10`); `client.jar` is not published (the installer produces it) |
| `NeoForge` | `maven.neoforged.net/api/maven/versions/releases/net/neoforged/neoforge` (flat array) | `{V}/neoforge-{V}-installer.jar` + `-universal.jar` from `maven.neoforged.net/releases` | MC `1.x` → prefix `x.0.` (two-part MC) or `x.y.` (three-part); snapshot `25w14craftmine` → `0.25w14craftmine.`; post-1.x numeric (`26.1`) → direct prefix |
| `Vanilla` | `piston-mc::manifest_v2::ManifestV2::fetch()` | client jar via `VersionManifest::download_client` → `{mc}.jar` | `stable` = Mojang release type `release` |

### `minecraft` binding rule

`versions_for_minecraft` populates `LoaderVersion.minecraft = Some(mc)`.
`list_versions` leaves it `None` for fabric/quilt/neoforge (their global listings carry
no per-version MC binding) — but fills it for forge (derivable from the combined
version string) and vanilla (the version *is* the MC version). `download_client`
requires `minecraft: Some`; otherwise `MinecraftVersionUnknown` tells the caller to use
`versions_for_minecraft`.

### Stability flags

- fabric/quilt: the meta API's `stable` field.
- forge: always `true` (the maven JSON carries no stability metadata; promotions come later).
- neoforge: `false` when the version carries a `-beta` / `-prerelease` / `-rc` suffix.
- vanilla: Mojang release type `release`.

### Install

`install(version, game_dir, java, lines, progress)` turns a version into a `.minecraft`-style
folder: the version JSON goes to `versions/<id>/<id>.json` and libraries to `libraries/`.

| Kind | Install |
|---|---|
| `Fabric` / `Quilt` | Fetch `<meta>/<mc>/<loader>/profile/json`, which inherits the Minecraft version. Write it under `versions/<id>/`, then download its libraries from their own base urls. No installer runs. |
| `Forge` / `NeoForge` | `download_client` fetches the installer jar into `libraries/`. Seed `launcher_profiles.json` with `{"profiles":{}}`, then run `java -jar <installer> --installClient <dir>` (NeoForge: `--install-client`). The new `versions/<id>` folder is the result. Output lines stream to `lines`; dropping that receiver stops the installer. |
| `Vanilla` | `InstallUnsupported`. The gui installs vanilla through piston-mc: client jar, libraries, assets. |

Installers run on a blocking thread. A failed run keeps the last 40 output lines in
`LoaderError::InstallerFailed`.

## 6. Mod metadata extraction

`extract_mod_info` works on the jar as-is; the generic `K` only picks the entry point,
and the actually-detected metadata format wins — `ModInfo.loader` reports what was
found ("fabric", "forge", "neoforge", "quilt"), never the requested kind. This means
`Loader::<Fabric>::extract_mod_info` happily reads a NeoForge jar; the marker is a
convenience, not a filter.

- modcrawl reads the metadata (sentinel files, JSON/TOML parsing) and analyzes
  dependencies (kind-prioritized, jar-in-jar aware) — both are mapped into `ModInfo`.
- For `mods.toml` / `neoforge.mods.toml`, the first `[[mods]]` entry is the jar's
  primary mod.
- Forge's legacy `mcmod.info` (pre-1.14) is supported through modcrawl.
- Server-plugin metadata (Bukkit/Paper) is intentionally out of scope →
  `ModMetadataMissing` (Lodestone launches clients).
- Quilt: `quilt.mod.json` is parsed locally (contributors map-or-list → authors;
  `depends` → Required, or Optional when `unless` is present; `breaks` → Incompatible;
  loader `version` may be a string or a maven-range object — stored as its raw string).
  If the jar has no `quilt.mod.json` but carries `fabric.mod.json` (quilt's fabric
  compatibility), the fabric path is used instead.
- `Loader::<Vanilla>::extract_mod_info` is always `ModInfoUnsupported`.

## 7. Errors (`error.rs`)

| Variant | When | Context carried |
|---|---|---|
| `Network` | reqwest transport failure | url + source |
| `UnexpectedResponse` | non-success HTTP status from a meta API | url + status |
| `Json` | meta response failed to deserialize | url + source |
| `Download` | `simple_download_utility` failure | url + source |
| `Io` | creating download target directories | path + source |
| `MinecraftVersionUnknown` | `download_client` on a version from `list_versions` that has no MC binding | version |
| `VersionNotFound` | the requested loader version does not exist for the requested MC version | version + minecraft |
| `ModInfoUnsupported` | `Loader::<Vanilla>::extract_mod_info` | loader |
| `ModMetadataMissing` | jar carries no client-mod metadata we support | path |
| `ModParse` | metadata file found but unparsable | path + source |
| `InstallUnsupported` | `install` on vanilla | loader |
| `InstallerLaunch` | the Java program for an installer could not start | program + source |
| `InstallerFailed` | an installer exited unsuccessfully or was stopped | loader + status + last output lines |
| `InstallerVersionMissing` | an installer exited cleanly without creating a version folder | loader + directory |
| `InstallerMissing` | the installer jar was not downloaded | loader |

## 8. Testing strategy

No network in CI:

- `tests/mod_info.rs` builds fixture jars in-test with the `zip` crate (dev-dep) and
  asserts `extract_mod_info` through the public API for every format, plus the
  fabric-fallback quilt path, missing-metadata, and vanilla-unsupported cases.
- Unit tests inside `src/forge.rs` (metadata.json parsing, combined-version → MC,
  artifact URLs), `src/neoforge.rs` (prefix mapping, stability, flat-array parsing),
  `src/fabric.rs` / `src/quilt.rs` (entry parsing, maven path building), and
  `src/mod_info/quilt.rs` (schema quirks: object versions, contributor map/list,
  license arrays) use recorded real API payloads.
- `tests/install.rs` covers version JSON placement, launcher profile seeding, installer
  discovery, and fake-`java` runs (unix) for success, failure, a missing version and a
  missing program.
- `examples/` are the live-network smoke tests, run manually.
