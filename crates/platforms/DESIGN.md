# Lodestone Platforms — Design

Design specification for `lodestone_platforms` (`crates/platforms`), the workspace's
unified content-platform module. This document is the implementation contract: module
layout, public API, semantics, and boundaries. Where [README.md](README.md) disagrees,
this document is more specific.

---

## 1. Role and architectural position

`lodestone_platforms` searches content, resolves its versions and files, downloads
files, and parses modpack archives — for the five platforms a Lodestone user browses:

```
gui  ──▶  Platform<Modrinth> / Platform<CurseForge> / Platform<AtLauncher> / Platform<Ftb> / Platform<Technic>
          (this crate)
```

### Hard rules

- **Standalone leaf crate.** Zero path dependencies on workspace siblings; only
  `crates/gui` links to workspace crates.
- **Async, tokio-based** — iced `Task`s await it directly.
- **Dumb about instances.** It never installs content, never launches, never resolves
  loaders. Downloaded files and parsed modpack data are the caller's to place.
- **Wheels are not reinvented** (see §2), and every API binding is verified against the
  live service during development (recorded payloads feed the hermetic tests).

## 2. Wheel audit

| Job | Crate used | Why |
|---|---|---|
| Modrinth API (search, project, versions, files) | `ferinth` 2.13 | Actively maintained gorilla-devs wrapper on reqwest 0.13 (same as the workspace); full surface including search |
| Downloads + progress | `simple_download_utility` | Same utility piston-mc and lodestone_loaders use |
| Modpack archive reading | `zipcrawl` | Read-only jar/zip access without extraction (mrpack index, CurseForge manifest) |
| CurseForge API | hand-rolled | `furse` was considered but wraps **no search**, forcing a split-brain (furse on reqwest 0.12 + own search on 0.13) for part of the surface; a coherent thin binding on the workspace's reqwest 0.13 is less machinery overall — **deviation from the ratified plan**, applied after live inspection of furse's API |
| ATLauncher / FTB / Technic APIs | hand-rolled | No maintained crates exist (searched crates.io, 2026-10); `ftbapi`'s source served as an endpoint reference for FTB |

Whole-launcher libraries (libium, mcvm, portablemc, lighty-*) were rejected: they drag
a parallel launcher architecture into a workspace that already has piston-mc +
lodestone_loaders + lodestone_launch.

## 3. Module layout

```
crates/platforms/
├── Cargo.toml              # lodestone_platforms
├── README.md               # short contract
├── DESIGN.md               # this document
├── src/
│   ├── lib.rs              # crate docs; PlatformKind trait; THE pub struct Platform<K>
│   ├── error.rs            # PlatformError
│   ├── meta.rs             # ContentType, SearchResult, ContentDetails, ContentVersion,
│   │                       # ContentFile, Modpack, ModpackFile
│   ├── http.rs             # shared get_json/download_files helpers (private, closure-free)
│   ├── modrinth.rs         # pub struct Modrinth    — delegates to ferinth
│   ├── curseforge.rs       # pub struct CurseForge  — hand-rolled api.curseforge.com/v1
│   ├── atlauncher.rs       # pub struct AtLauncher  — hand-rolled api.atlauncher.com/v1
│   ├── ftb.rs              # pub struct Ftb         — hand-rolled api.feed-the-beast.com/v1/modpacks
│   ├── technic.rs          # pub struct Technic     — hand-rolled api.technicpack.net
│   └── modpack/            # archive parsing
│       ├── mod.rs          # format detection + dispatch
│       ├── mrpack.rs       # modrinth.index.json
│       └── curseforge.rs   # manifest.json
├── examples/               # live-network smoke tests (run manually)
└── tests/
    ├── fixture/            # archive builder (zip dev-dep)
    └── platforms.rs        # public API over recorded payloads + fixtures
```

Pure helpers (payload parsing, URL building, capability checks) are unit-tested in
`#[cfg(test)]` modules inside their source files, mirroring the `lodestone_launch` /
`lodestone_loaders` pattern.

## 4. Public API

```rust
impl<K: PlatformKind> Platform<K> {
    pub fn new() -> Self;
    pub fn with_client(http: reqwest::Client) -> Self;
    pub async fn search(&self, query: &str, content: ContentType) -> Result<Vec<SearchResult>, PlatformError>;
    pub async fn details(&self, id: &str, content: ContentType) -> Result<ContentDetails, PlatformError>;
    pub async fn versions(&self, id: &str, content: ContentType) -> Result<Vec<ContentVersion>, PlatformError>;
    pub async fn download(
        &self,
        version: &ContentVersion,
        into: impl AsRef<Path>,
        progress: Option<&Sender<DownloadProgress>>,
    ) -> Result<Vec<PathBuf>, PlatformError>;
    pub fn parse_modpack(&self, archive: impl AsRef<Path>) -> Result<Modpack, PlatformError>;
}

// Specialized constructor — CurseForge requires an API key
// (https://console.curseforge.com); the header is baked into the client and the
// key is threaded to the kind's calls.
impl Platform<CurseForge> {
    pub fn with_api_key(key: impl Into<String>) -> Self;
}

// The contents of a Modrinth modpack release: the projects it bundles, named and iconed.
impl Platform<Modrinth> {
    pub async fn bundled(&self, version: &ContentVersion) -> Result<Vec<BundledProject>, PlatformError>;
    // The release each SHA-1 belongs to, and the newest release per SHA-1 for a loader and Minecraft version.
    pub async fn versions_by_hashes(&self, hashes: &[String]) -> Result<HashMap<String, ContentVersion>, PlatformError>;
    pub async fn latest_by_hashes(&self, hashes: &[String], loaders: &[String], game_versions: &[String]) -> Result<HashMap<String, ContentVersion>, PlatformError>;
    // Project names, keyed by project id.
    pub async fn project_names(&self, project_ids: &[String]) -> Result<HashMap<String, String>, PlatformError>;
}

// One CurseForge file, with its name, download url (resolved when hidden), sha1 and size.
impl Platform<CurseForge> {
    pub async fn file(&self, project_id: &str, file_id: &str) -> Result<ContentFile, PlatformError>;
    // The files that fit a Minecraft version and a loader, newest first, from one filtered page.
    pub async fn fitting_files(&self, project_id: &str, game_version: &str, loader: &str) -> Result<Vec<ContentVersion>, PlatformError>;
}
```

| Type | Module | Role |
|---|---|---|
| `Platform<K>` | lib | THE single behavioral struct; `K` is a zero-sized marker. |
| `PlatformKind` | lib | Sealed trait; carries per-kind endpoints/parsers and the capability consts. |
| `Modrinth` / `CurseForge` / `AtLauncher` / `Ftb` / `Technic` | per-kind modules | Zero-sized markers. |
| `ContentType` | meta | `Mod`, `Modpack`, `Shader`, `ResourcePack`, `DataPack`, `World`. |
| `SearchResult` | meta | id, slug, name, author, short description, downloads, icon/web urls, `banner_url` (the featured image, else the first gallery image), content type. |
| `ContentDetails` | meta | SearchResult + long description, `description_format` (Markdown or Html), and `info: ProjectInfo`. |
| `ProjectInfo` | meta | The facts beside a description, all empty where a platform has none: `gallery`, `license`, `published`/`updated`, `followers`, `categories`, `loaders`, `game_versions`, `environments` (`Side`), `links` (`ProjectLinks`), `creators` (`Creator`). |
| `GalleryImage` | meta | thumbnail and full-size urls, title, description, featured flag. |
| `ContentVersion` | meta | id, name, version_number, `minecraft_versions`, `loaders` (join point with lodestone_loaders), downloads, date, changelog_url, files, `changelog` (markdown), `channel` (`Channel`), `dependencies` (`VersionDependency`: `DependencyKind` plus project and version ids). |
| `ContentFile` | meta | name, url, sha1, size, primary. |
| `BundledProject` | meta | A project a modpack release bundles: id, name, icon, and the bundled release's id and number. |
| `Modpack` / `ModpackFile` | meta | Parsed archive: name, version, minecraft, loader + loader_version, files (direct url or CurseForge project/file ids), overrides dir. |
| `DownloadProgress` | re-export | simple_download_utility's progress event. |
| `PlatformError` | error | thiserror; messages user-presentable. |

`Platform::with_client` customizes the HTTP client for the hand-rolled kinds; the
Modrinth kind constructs its own client inside ferinth (Modrinth requires a
descriptive user agent), so the injected client does not reach it there.

## 5. Capability matrix

| Kind | Mod | Modpack | Shader | ResourcePack | DataPack | World | parse_modpack |
|---|---|---|---|---|---|---|---|
| Modrinth | ✓ | ✓ | ✓ | ✓ | ✓ | — | ✓ (.mrpack) |
| CurseForge | ✓ | ✓ | — | ✓ | — | ✓ | ✓ (export .zip) |
| AtLauncher | — | ✓ | — | — | — | — | — |
| Ftb | — | ✓ | — | — | — | — | — |
| Technic | — | ✓ | — | — | — | — | — |

Requests outside a kind's capability return `UnsupportedContentType`; `parse_modpack`
on a kind without an archive format returns `UnsupportedPlatform`. ATLauncher has no
search endpoint: its search is a client-side filter over the full pack list.

## 6. Per-kind endpoints (verified live during development)

| Kind | Endpoints | Notes |
|---|---|---|
| Modrinth | via ferinth: search (project-type facet, with `gallery` and `featured_gallery` on each hit), `project_get` (gallery with `raw_url`, licence, dates, links), `team_list_members` (creators, best-effort), `version_list` (changelog, version type, dependencies), version file URLs, and for modpacks `project_get_multiple` and `version_get_multiple` over the `embedded` dependencies (`Platform::<Modrinth>::bundled`), `version_files` lookups by SHA-1 (`versions_by_hashes`, `latest_by_hashes`) and `project_get_multiple` names (`project_names`) | ferinth builds its own client; Modrinth expects a descriptive user agent |
| CurseForge | `api.curseforge.com/v1`: `/mods/search?gameId=432&classId=…&searchFilter=…`, `/mods/{id}`, `/mods/{id}/description`, `/mods/{id}/files` (every page, by `index`/`pageSize`), `/mods/{id}/files/{fileId}` (`file`) | classIds: mods 6, modpacks 4471, resource packs 12, worlds 17, shaders 6552, datapacks 6945 (the last two not yet checked against a live key); `x-api-key` header required; loaders come from the names in `gameVersions`, and the search's `modLoaderType` is 1 forge, 4 fabric, 5 quilt, 6 neoforge; a file with an empty `downloadUrl` is downloaded from the CDN at `mediafilez.forgecdn.net/files/{id / 1000}/{id % 1000}/{fileName}` with a Firefox user agent (`lodestone_net::download`); files marked `isAvailable: false` are left out; `screenshots` → gallery and banner; `releaseType` 1/2/3 → release/beta/alpha; file `relationType` 1/2/3/5 → embedded/optional/required/incompatible (the field numbers are not yet checked against a live key) |
| ATLauncher | `api.atlauncher.com/v1`: `/pack-list`, `/pack/{code}` | search filters the pack list client-side; downloads resolve to ATLauncher's CDN pack zips |
| FTB | `api.feed-the-beast.com/v1/modpacks`: `/public/modpack/search/{limit}?term=…`, `/public/modpack/{id}`, `/public/modpack/{id}/{versionId}` | search returns ids only — the first page is hydrated via details calls; version files carry install paths served from FTB's CDN |
| Technic | `api.technicpack.net`: `/search/builds?q=…`, `/modpack/{slug}`, `/modpack/{slug}/build/{build}` | build JSON carries the Minecraft version and mod list; `headers[]=X-Build-Data` query parameter is required |

## 7. Modpack parsing

`parse_modpack` sniffs the archive (via zipcrawl) for:

- `modrinth.index.json` → `.mrpack`: dependencies map gives minecraft + loader/loader
  version; files carry direct download urls + sha1 + size; `overrides/` directory.
- `manifest.json` (CurseForge export): `minecraft.version` + `modLoaders[].id`
  (`forge-47.2.0`); files carry `projectID`/`fileID` pairs (no urls — resolution needs
  the CurseForge API at install time); `overrides/` directory.

Anything else → `ModpackFormatUnknown`. CurseForge-modpack entries therefore surface
with `project_id`/`file_id` set and `url` empty; resolving those ids into files is the
install slice's job.

## 8. Errors (`error.rs`)

| Variant | When | Context carried |
|---|---|---|
| `Network` | transport failure | url + source |
| `UnexpectedResponse` | non-success HTTP status | url + status |
| `Json` | response failed to deserialize | url + source |
| `Download` | file download failure | url + source |
| `Io` | creating download target directories | path + source |
| `UnsupportedContentType` | content type outside the kind's capability matrix | platform + content |
| `UnsupportedPlatform` | action not offered by the kind (e.g. `parse_modpack` on Technic) | platform + action |
| `ModpackFormatUnknown` | archive contains neither known manifest | path |
| `ModpackParse` | manifest found but unparsable | path + source |

## 9. Testing strategy

No network in CI:

- `tests/platforms.rs` asserts the public API over recorded real payloads: capability
  gating, CurseForge payload parsing + loader mapping, ATLauncher/FTB/Technic payload
  parsing, and modpack archive parsing against fixture archives built in-test with the
  `zip` dev-dep.
- Unit tests inside the kind modules cover url builders and mappings.
- `examples/` are the live-network smoke tests, run manually; they double as the
  verification vehicle for the hand-rolled endpoints (base urls drift over time —
  each lives in exactly one submodule).
