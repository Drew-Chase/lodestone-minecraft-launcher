# Modrinth provider

Implementation notes and API reference for the Modrinth integration inside `hopper-mc`. Read this alongside the source if you are extending the provider, investigating a mapping quirk, or debugging a response that looks wrong.

**Upstream API:** Labrinth v2 — <https://api.modrinth.com/v2>
**Official docs:** <https://docs.modrinth.com/api/>
**Status page:** <https://status.modrinth.com/>

---

## Why this is the simplest provider in the crate

Modrinth is the reference implementation that the rest of the crate's abstractions were shaped around, for three reasons:

1. **Anonymous access.** No auth, no API key, no rate-limit registration — just send a descriptive `User-Agent` and you are done.
2. **Uniform endpoints.** A single `/search` endpoint serves every content type we care about; one `/project/{id}` returns full detail. No per-kind path variants.
3. **Predictable shapes.** Response JSON is close to the crate's public model, so the mapping layer is mostly field-renames and a small amount of DTO-tier massaging.

If you are writing a new platform provider, copy this module's shape first and adapt.

---

## Module layout

```
modrinth/
├── mod.rs       # ModrinthProvider + impls of all six content traits
├── api.rs       # Raw HTTP: /search and /project/{id} only
├── dto.rs       # #[derive(Deserialize)] wire structs — SearchHit, Project, …
└── mapping.rs   # DTO → public model conversions, Sort/ContentType mapping
```

Each file stays at exactly one layer of abstraction. If you find yourself tempted to call `reqwest` from `mapping.rs` or parse a JSON body in `mod.rs`, stop and put it in the right module.

---

## Endpoints

### `GET /v2/search`

Used for every `find_*` call. Query parameters we send:

| Param | Source | Notes |
|---|---|---|
| `query` | caller's `Option<&str>` | Omitted when `None`; Modrinth then does a pure browse. |
| `index` | `sort_to_index(sort)` | See sort table below. |
| `offset` | `page * per_page` | Item offset, zero-based. |
| `limit` | `per_page.clamp(1, 100)` | Modrinth's hard cap is 100. |
| `facets` | `content_type_to_facets(kind)` | A JSON-encoded 2-D array, see below. |

Response: `{ hits: [SearchHit], offset, limit, total_hits }`. We ignore everything but `hits`.

### `GET /v2/project/{id_or_slug}`

Used for every `get_*` call. Accepts **both** numeric project ids and slugs transparently — no slug-lookup fallback needed (unlike CurseForge).

- Returns `404` when the project does not exist → the provider maps this to `Ok(None)` in `api::get_project`.
- Returns `200` with the full `Project` document otherwise.

### Everything else

Endpoints we **do not** use (yet): `/project/{id}/version`, `/team/{id}/members`, `/tag/…`, `/user/{id}`. DTOs for a team-members response exist (`dto::TeamMember`, `TeamUser`) as scaffolding for a future authors-resolution pass; they are marked `#[allow(dead_code)]` at the module level.

---

## Sort mapping

```rust
Sort::Relevance => "relevance"
Sort::Downloads => "downloads"
Sort::Follows   => "follows"
Sort::Latest    => "newest"
Sort::Updated   => "updated"
```

Modrinth names the "most-recently-created" axis `newest`; our enum calls it `Latest` for symmetry with the other platforms. Covered by the `sort_index_mapping` unit test in `mapping.rs`.

---

## Content-type → facet mapping

Modrinth filters search by `project_type` facets. We send a single-group 2-D array: `[["project_type:<value>"]]`.

| `ContentType` | `project_type` value | Notes |
|---|---|---|
| `Mod`          | `mod`          | |
| `Modpack`      | `modpack`      | |
| `ResourcePack` | `resourcepack` | |
| `ShaderPack`   | `shader`       | Note singular. |
| `Datapack`     | `datapack`     | See datapack quirk below. |
| `World`        | *(none)*       | Modrinth does not host worlds — returns `ContentError::UnsupportedContentType`. |

Covered by the `content_type_facets` unit test.

### The datapack quirk

Modrinth represents datapacks **two** ways depending on when the project was uploaded:

- **Current:** `project_type = "datapack"` as a first-class type.
- **Legacy:** `project_type = "mod"` with `loaders` containing `"datapack"`.

Our search uses the first form only. Legacy projects may not surface when searching by facet `project_type:datapack`. If full coverage matters to you, widen the facet to the union and filter post-hoc — but doing so also pulls in unrelated mods that happen to mention "datapack" in their loaders array, so it is a tradeoff we chose not to make by default.

---

## DTO tiers: `SearchHit` vs `Project`

Modrinth's two endpoints return deliberately different shapes. `mapping.rs` keeps this explicit rather than papering over it:

| Field on the public `ContentBase` | `/search` (`SearchHit`) | `/project/{id}` (`Project`) |
|---|---|---|
| `id`, `slug`, `title`, `summary` | ✅ | ✅ |
| `description` (long body) | ❌ → `None` | ✅ from `project.body` |
| `authors` | 1 entry from `SearchHit.author` | **empty** — the full project only exposes a `team` id; members require `/team/{id}/members` |
| `icon_url` | ✅ | ✅ |
| `gallery` | ✅ `SearchHit.gallery[]` | ✅ ordered: featured first, then the rest |
| `links` (source/issues/wiki/discord/donation) | **empty** — search hits do not carry these | ✅ |
| `license` | `SearchHit.license` → SPDX id only, `name`/`url` left `None` | full `ProjectLicense` |
| `categories` | union of `categories` + `display_categories` | union of `categories` + `additional_categories` |
| `game_versions` | `SearchHit.versions` | `Project.game_versions` |

If you need the long description or the project's source URL, use `get_mod(id, Platform::Modrinth)` — not `find_mods(…)`.

### The authors caveat

Full `/project/{id}` responses only carry a `team` id, not member details. Populating `authors` from a project endpoint requires a second call to `/team/{id}/members`. Until we wire that, `get_*` calls return items with `authors = []`. `find_*` calls, paradoxically, do have a single author thanks to `SearchHit.author`.

If your UI wants both a primary author *and* support for `get_*` details, do the combined lookup yourself:

```rust
let item = hopper_mc::get_mod("fabric-api", Platform::Modrinth).await?;
// ...then fetch team members via a direct /team/{id}/members call.
```

---

## Loader extraction

Modrinth puts loaders in the `categories` array, side-by-side with real topic categories like `"technology"` and `"adventure"`. We filter against a small allowlist of known loader names defined as the `KNOWN_LOADERS` constant in `mapping.rs`:

```
fabric, forge, neoforge, quilt, liteloader, modloader, risugamis-modloader, rift,
bukkit, spigot, paper, purpur, sponge, bungeecord, velocity, waterfall, folia,
iris, optifine, canvas, vanilla, minecraft, datapack
```

Anything not on the list stays in `categories` but does not appear in `loaders`. Adding a new loader is a one-line change — no taxonomy work required, and the miss (a real loader name we do not know yet) is harmless: it simply does not show up under `loaders`.

Covered by `loader_filter_keeps_only_known_loaders`.

### `Project.loaders` vs `categories`

The full project document has a dedicated `loaders: Vec<String>` field separate from `categories`. We prefer it when populated and fall back to the `categories`-filter path only when `project.loaders` is empty.

---

## User-Agent

Modrinth's terms of service ask clients to send a distinctive `User-Agent` per app. The crate default is `hopper-mc/<version>`, fine for development. For production, brand it with your app name — or better, include a contact address:

```rust
let modrinth = ModrinthProvider::with_user_agent("my-launcher/1.0 (contact@example.com)");
```

Key enforcement: set the UA at `reqwest::Client` construction (see `platforms/user_agent.rs::client_with_ua`) — once per provider — rather than attaching it per-request. `shared()` uses the default.

---

## Error handling

`api::handle_common_status` short-circuits on:

- **429 Too Many Requests** — `ContentError::RateLimited(Retry-After)`. Modrinth returns a `Retry-After` header in seconds; we parse it and surface it. Back off accordingly.
- **400 Bad Request** — `ContentError::BadRequest`. Usually means a malformed facet or limit out of range; check what you sent before assuming Modrinth broke.

Everything else that is not `200` / `404` is routed through `resp.error_for_status()?` and surfaces as `ContentError::Http`. A 5xx is generally transient.

---

## Things this provider does not do (yet)

- **Version lookup.** `/project/{id}/version` is needed to resolve `ModItem::dependencies`. Currently always empty.
- **Team resolution.** `/team/{id}/members` is needed to populate `authors` on `get_*` results. See the caveat above.
- **Shader loader detection.** `ShaderPackItem::shader_loaders` is always empty — Iris vs OptiFine vs Canvas compatibility lives in file metadata, not the project document.
- **Server-pack awareness.** `PackItem::has_server_pack` is always `false`; Modrinth exposes a server-pack flag on version files, not on the project itself.
- **Modpack mod count.** `PackItem::included_mods_count` stays `None`; this requires parsing the `.mrpack` manifest.

All of these are body-only additions when you need them — the trait signatures are already correct.
