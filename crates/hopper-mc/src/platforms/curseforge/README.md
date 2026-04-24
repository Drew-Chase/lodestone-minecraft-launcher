# CurseForge provider

Implementation notes, API reference, and security posture for the CurseForge integration inside `hopper-mc`. Read this alongside the source if you are extending the provider, investigating a mapping quirk, diagnosing a 401, or working on the key-handling layer.

**Upstream API:** CurseForge Core API v1 — <https://api.curseforge.com/v1>
**Official docs:** <https://docs.curseforge.com/rest-api/>
**Developer console (key issuance):** <https://console.curseforge.com/>

---

## What makes this provider different from Modrinth

Three properties drive almost every design decision in this module:

1. **No anonymous access.** Every request requires an `x-api-key` header. Keys are issued to approved developers and are meant to be embedded in client apps — they are *developer* secrets, not end-user credentials, but they still must not be left lying around in shipped binaries.
2. **Response envelope.** Every payload is wrapped in `{ "data": ... }`, with list endpoints adding a `pagination` sibling. DTOs are built around a generic `Envelope<T>` / `PaginatedEnvelope<T>` so every call path looks the same.
3. **Multiple content kinds live under one endpoint.** `/v1/mods/search` handles mods, modpacks, resource packs, shaders, datapacks, *and* worlds — disambiguated by a `classId` integer. Slugs are also disambiguated by class: the same slug under `classId=6` (Mod) and `classId=4471` (Modpack) points to different projects.

If Modrinth is the "how to write a provider" reference, this is the "how to handle a provider that needs credentials and a hand-cranked dispatch" reference.

---

## Module layout

```
curseforge/
├── mod.rs       # CurseForgeProvider + impls of all six content traits +
│                #   SecretString-based constructors + key-exposure helper
├── api.rs       # Raw HTTP: /mods/search, /mods/{id}, slug fallback
├── dto.rs       # Wire-format types: Envelope<T>, PaginatedEnvelope<T>,
│                #   CfMod, FileIndex, ModLinks, ModAuthor, Category, CfFile
└── mapping.rs   # Sort/ContentType/loader mapping, DTO → public model,
                 #   MC version / loader filtering, class-id constants
```

See `src/platforms/modrinth/README.md` for the module-responsibility rules — they are identical here.

---

## Authentication

### Obtaining a key

Keys are issued via <https://console.curseforge.com/>. CurseForge only grants keys to approved developers; a non-developer account that signs up will be deleted. This is why the library does not ship a default key — there is no such thing.

### How the provider holds the key

The `api_key` field is `Option<secrecy::SecretString>`. This is not cosmetic:

- **`Debug` is safe.** A `println!("{:?}", provider)` prints `api_key: Some(SecretBox<str>([REDACTED]))`. The key cannot leak through logging or panic messages.
- **Zero on drop.** When the provider is dropped, `SecretString`'s `Zeroize` impl wipes the backing bytes before they return to the allocator. Core dumps taken after that point cannot recover the key.
- **Exposure is explicit.** The only way to get at the plaintext is `SecretString::expose_secret()`, which returns a short-lived `&str`. In this module that call lives inside `CurseForgeProvider::exposed_key()`, which is invoked exactly once per outbound HTTP request to populate the `x-api-key` header.

### What the library deliberately does **not** do

- It does **not** read `CURSEFORGE_API_KEY` from the environment. Env vars are visible to every process under the same UID; a compromised sibling process would harvest them without any prompt or permission request.
- It does **not** bake a default key into the binary. There is no "just works without config" path — callers must plumb a key through explicitly.
- It does **not** log or `Debug`-print the key anywhere in the crate.

### Recommended flows

Two runnable reference examples live in the crate root, each for a different deployment model:

- **Per-user keys** → `examples/curseforge_with_keyring.rs`. Pulls the key from the OS credential store on first run, prompts with a hidden terminal field if missing, persists for subsequent runs. Plaintext never touches an env var, a config file, or the process's command line. Use this when each end-user brings their own key.
- **Packaged app with a shared developer key** → `examples/curseforge_with_embedded_key.rs`. The key is XOR-obfuscated at build time (see `build.rs` under the `embedded-key` feature flag), embedded into the binary, and decoded at runtime via `core::ptr::read_volatile` (critical — see the example's doc comment for why a naïve const-XOR leaks the plaintext). Use this when you are shipping a launcher or companion app under your own CurseForge developer key.

Both examples document the threat model in detail. Obfuscation is not encryption; embedded keys can be recovered by a debugger-wielding attacker. CurseForge's key-issuance model expects embedding; rotate the key in the console if it leaks.

### Constructors

All six constructors end in `…key`. The four that take a `SecretString` are preferred; the two that take a plain `String` exist for ergonomics and wrap the value immediately.

```rust
CurseForgeProvider::new()
CurseForgeProvider::new_with_secret_key(Option<SecretString>)
CurseForgeProvider::new_with_key(Option<String>)
CurseForgeProvider::with_user_agent(&str)
CurseForgeProvider::with_user_agent_and_secret_key(&str, Option<SecretString>)
CurseForgeProvider::with_user_agent_and_key(&str, Option<String>)
CurseForgeProvider::with_client_and_secret_key(Client, Option<SecretString>)
CurseForgeProvider::with_client_and_key(Client, Option<String>)
```

The `shared()` method exists for trait-dispatch symmetry but has no key. Calls through it will return `ContentError::BadRequest` unless and until a keyed provider is constructed directly.

`has_api_key() -> bool` is provided so callers can gate UI before making a failing call.

---

## Endpoints

### `GET /v1/mods/search`

Used for every `find_*` call. Query parameters we send:

| Param | Source | Notes |
|---|---|---|
| `gameId` | `MINECRAFT_GAME_ID` (432) | Constant. |
| `classId` | `content_type_to_class_id(kind)` | Integer per content kind, see below. |
| `sortField` | `sort_to_cf(sort).0` | Integer 1–12. |
| `sortOrder` | `sort_to_cf(sort).1` | Always `"desc"` in our mapping. |
| `index` | `page * per_page`, clamped | Item offset; capped so `index + pageSize ≤ 10_000`. |
| `pageSize` | `per_page.clamp(1, 50)` | CurseForge's hard cap is 50. |
| `searchFilter` | caller's `Option<&str>` | Omitted when `None` or empty; matches name + author only, **not** description. |

Response body (after unwrapping the envelope): `Vec<CfMod>`.

### `GET /v1/mods/{modId}`

Used for numeric-id `get_*` calls.

- Numeric id only — no slug support.
- 404 → `Ok(None)`.
- 200 → `Envelope<CfMod>` → `Some(CfMod)`.

### Slug fallback

`get_*` calls accept `&str`, which may be a numeric id or a slug. `api::get_by_id_or_slug` parses as `u64` first; on parse failure it falls back to a one-result slug search scoped to the correct `classId`:

```
GET /v1/mods/search?gameId=432&classId={class}&slug={slug}&pageSize=1
```

This is why knowing the content kind is mandatory for `get_*` — a slug alone is not globally unique on CurseForge. The public trait methods (`get_mod`, `get_pack`, etc.) each pass the right `classId` constant so callers never see this concern.

### Endpoints we do not use yet

- `POST /v1/mods` (bulk lookup by id) — would let us batch `get_*` calls.
- `GET /v1/mods/{id}/files` — required to surface file-level metadata (downloads, game versions per file, loader per file).
- `GET /v1/mods/{id}/description` — returns the long HTML description. Not called; `ContentBase::description` is always `None` for CurseForge results.
- `GET /v1/categories?gameId=432&classesOnly=true` — would let us *verify* the class-id constants at runtime instead of hardcoding them.

---

## Class-id mapping

Defined as module-level constants in `mapping.rs`:

| Constant | Value | Content kind |
|---|---:|---|
| `CLASS_MODS`          | 6     | Mods |
| `CLASS_RESOURCEPACKS` | 12    | Resource packs |
| `CLASS_WORLDS`        | 17    | Worlds |
| `CLASS_MODPACKS`      | 4471  | Modpacks |
| `CLASS_SHADERS`       | 6552  | Shader packs |
| `CLASS_DATAPACKS`     | 6945  | Datapacks |

Unlike Modrinth, **CurseForge serves every one of our content kinds** — including worlds. This is the only platform in the crate where `find_worlds(…, Platform::CurseForge, …)` will actually return results.

Covered by the `class_ids_cover_every_content_type` unit test.

---

## Sort mapping

```rust
Sort::Relevance => (sortField=1,  sortOrder="desc")  // Featured
Sort::Downloads => (sortField=6,  sortOrder="desc")  // TotalDownloads
Sort::Follows   => (sortField=12, sortOrder="desc")  // Rating
Sort::Latest    => (sortField=11, sortOrder="desc")  // ReleasedDate
Sort::Updated   => (sortField=3,  sortOrder="desc")  // LastUpdated
```

Mapping choices to be aware of:

- **`Relevance` → Featured.** CurseForge does not expose a direct "relevance" axis like Modrinth does. Featured, when combined with a `searchFilter`, biases toward quality matches and is the closest fit.
- **`Follows` → Rating.** CurseForge has no follower count; we surface the `rating` field (the `thumbs_up_count` feeds the public `follows` number, but the API's sort key is named `rating`). If the user-facing semantics of "Follows" matter to you, consider switching this to `Popularity=2`, which blends activity and downloads.

CurseForge's full `sortField` enum (from <https://docs.curseforge.com/rest-api/#tocS_ModsSearchSortField>):
`1=Featured, 2=Popularity, 3=LastUpdated, 4=Name, 5=Author, 6=TotalDownloads, 7=Category, 8=GameVersion, 9=EarlyAccess, 10=FeaturedReleased, 11=ReleasedDate, 12=Rating`. Covered by `sort_mapping_is_complete`.

---

## Loader extraction

CurseForge puts loader info in `latestFilesIndexes[].modLoader` as an integer enum:

| Integer | Name we emit |
|---:|---|
| 0 | *(skipped — "Any")* |
| 1 | `forge` |
| 2 | `cauldron` |
| 3 | `liteloader` |
| 4 | `fabric` |
| 5 | `quilt` |
| 6 | `neoforge` |
| other | *(skipped)* |

Unknown ids are **silently dropped** rather than surfaced as numeric strings, so new loaders added upstream will not appear in `ModItem::loaders` until the table is updated. The miss is harmless — the loader still appears elsewhere (e.g. in the file's `gameVersions` array as a string).

We preserve first-seen order across the file indexes and deduplicate. Covered by `loader_name_covers_known_ids_and_drops_unknown` and `collect_loaders_dedupes_and_preserves_order`.

### Loaders live on files, not the project

One conceptual mismatch with Modrinth: CurseForge does not attach loaders to the *project*, only to the *files* uploaded under it. Our `collect_loaders` aggregates across all file indexes, so `ModItem::loaders` is the union of everything this project has ever supported — not the loader(s) of any single release. If you need "loaders for MC version X", drill down into `latestFilesIndexes[].mod_loader` yourself, keyed by `game_version`.

---

## Game versions extraction

CurseForge's `FileIndex.gameVersion` is a **string** that mixes real MC versions with loader names (`"Forge"`, `"Fabric"`) and channels (`"1.20-Snapshot"`). `is_mc_version` in `mapping.rs` filters strictly — only `X.Y` or `X.Y.Z` forms pass. Covered by `mc_version_filter_is_strict`. The result is deduplicated preserving first-seen order (`collect_game_versions_filters_and_dedupes`).

---

## Pagination

CurseForge caps `index + pageSize ≤ 10_000`. Requests beyond that return 400. `api::search` saturates rather than erroring:

```rust
let offset = raw_offset.min(PAGINATION_HARD_CAP.saturating_sub(limit));
```

Callers paginating too deep get fewer results (or none) instead of a surprise `ContentError::BadRequest`. The response's own `pagination` object carries the real counts; we currently discard it.

---

## Model population

What the CurseForge mapping layer can and cannot populate on `ContentBase` and the per-kind items:

| Public field | CurseForge source | Notes |
|---|---|---|
| `id` | `CfMod.id.to_string()` | Numeric → string for cross-platform uniformity. |
| `slug` | `CfMod.slug` | |
| `title` | `CfMod.name` | |
| `summary` | `CfMod.summary` | |
| `description` | **always `None`** | Requires a separate `/description` call; not yet wired. |
| `authors` | `CfMod.authors` | Has `id`, `name`, `url`. No avatar / email. |
| `icon_url` | `CfMod.logo.thumbnail_url`, falling back to `.url` | |
| `gallery` | `CfMod.screenshots[].url` | |
| `links` | `CfMod.links.{website,source,issues,wiki}_url` | `discord` and `donation` are `None` / empty — CF does not expose them. |
| `downloads` | `CfMod.download_count` | |
| `follows` | `CfMod.thumbs_up_count` | Closest signal; CF has no "followers". |
| `license` | **always `None`** | CurseForge genuinely has no license field anywhere in the v1 response shape. |
| `created` / `updated` | `CfMod.date_created` / `CfMod.date_modified` | ISO-8601 UTC, same as Modrinth. |
| `categories` | `CfMod.categories[].name` | Real topic tags, not loader names. |
| `game_versions` | derived via `collect_game_versions` | Filtered, deduped. |
| `ModItem.loaders` | derived via `collect_loaders` | See loader section. |
| `ModItem.client_side` / `server_side` | `SideSupport::Unknown` | Not exposed on the project document. |
| `ModItem.dependencies` | `Vec::new()` | Requires per-file metadata. |
| `PackItem.included_mods_count` | `None` | Requires parsing the modpack manifest. |
| `PackItem.has_server_pack` | `false` | Not exposed; would require per-file inspection. |

---

## Error handling

`api::handle_common_status` maps CurseForge HTTP statuses to `ContentError`:

- **401 Unauthorized** / **403 Forbidden** → `ContentError::BadRequest("curseforge rejected credentials: …")`. Fail fast; no retry. Usually a bad/revoked key.
- **429 Too Many Requests** → `ContentError::RateLimited(Retry-After)`. CurseForge does not publish its rate limits; back off with jitter.
- **400 Bad Request** → `ContentError::BadRequest`. Often means you hit the pagination cap without the saturation helper, or passed an invalid class/loader combo. `modLoaderType` returns 400 unless `gameVersion` is also set — avoid combining those on one request.
- **404 Not Found** (on `/mods/{id}` only) → `Ok(None)`.

Before any HTTP call, `require_key` short-circuits missing/empty keys with `BadRequest`, so a call without credentials never reaches the network.

---

## Known gotchas

- **`File.downloadUrl` can be `null`.** When the project owner disallows third-party distribution, CurseForge nulls `downloadUrl` instead of omitting it. `hopper-mc` does not currently fetch file details, but if you extend the provider, be ready to fall back to `links.websiteUrl` for the user-facing download path.
- **`searchFilter` is name + author only.** A query that matches text only in the description will not surface. If your users complain about missing results, this is usually why.
- **Slug collisions across classes.** The same slug can refer to different projects under different `classId` values. The slug-fallback path always scopes by class; do not lift it out of context.
- **Envelope unwrapping.** Every response lands inside `data: ...`. If you add a new endpoint, wire it through `Envelope<T>` or `PaginatedEnvelope<T>` rather than parsing directly.
- **`status` field.** CurseForge assigns projects an approval status; `status=4` is Approved. We do not filter on it, so rejected or pending projects can appear in results. Filter in your UI if that matters.

---

## Things this provider does not do (yet)

- **Long description.** No call to `/v1/mods/{id}/description`; `ContentBase::description` is always `None`.
- **File details.** No call to `/v1/mods/{id}/files`; per-file size, release type, explicit download URL, and MC-version-specific loader info are absent.
- **Bulk lookup.** `POST /v1/mods` is not used; `get_*` calls issue one HTTP request per id.
- **Runtime class-id verification.** The class constants are hardcoded rather than fetched from `/categories`.
- **Category-id filter passthrough.** `categoryId` / `categoryIds` search params are not exposed in the trait API.
- **Dependency resolution for modpacks.** Modpack manifests need to be downloaded, unzipped, and parsed; far outside the current scope.

All of these are additive — no trait changes required.
