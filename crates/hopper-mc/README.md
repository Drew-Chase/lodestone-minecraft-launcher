# hopper-mc

[![Crates.io](https://img.shields.io/crates/v/hopper-mc.svg)](https://crates.io/crates/hopper-mc)
[![Docs.rs](https://docs.rs/hopper-mc/badge.svg)](https://docs.rs/hopper-mc)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

> *In Minecraft, a hopper pulls items from every container above it and funnels them into a single output below. In your code, `hopper-mc` does the same thing with Minecraft content platforms.*

A unified async Rust client that funnels **mods, modpacks, datapacks, resource packs, worlds, and shader packs** from every major distribution platform — Modrinth, CurseForge, AT Launcher, TechnicPack, and FTB — into one strongly-typed API.

Search once, get the same struct back regardless of which platform served it.

| Platform     | Mods | Modpacks | Datapacks | Resource Packs | Shaders | Worlds | Status |
|--------------|:---:|:---:|:---:|:---:|:---:|:---:|---|
| Modrinth     | ✅  | ✅  | ✅  | ✅  | ✅  | —   | **implemented** |
| CurseForge   | 🚧  | 🚧  | 🚧  | 🚧  | 🚧  | 🚧  | skeleton |
| AT Launcher  | —   | 🚧  | —   | —   | —   | —   | skeleton |
| TechnicPack  | —   | 🚧  | —   | —   | —   | —   | skeleton |
| FTB          | —   | 🚧  | —   | —   | —   | —   | skeleton |

**Legend:** ✅ implemented · 🚧 skeleton (signatures final, bodies return `NotImplemented`) · — platform does not serve this content type.

---

## Why this crate

Every Minecraft launcher, server manager, or mod-management tool ends up writing the same glue: Modrinth's Labrinth API, CurseForge's v1 API, FTB's modpacks.ch, AT Launcher, Technic — five different REST shapes, five different auth stories, five different pagination schemes, five different ways of saying *"this is a Fabric mod for 1.21."*

`hopper-mc` collapses all of that into:

```rust
let mods = find_mods(Some("sodium"), Sort::Downloads, Platform::Modrinth, 0, 10).await?;
```

Swap `Platform::Modrinth` for `Platform::CurseForge` and the return type doesn't change. The shape of every struct your app sees is platform-agnostic.

## Highlights

- **One common model.** `ModItem`, `PackItem`, `DatapackItem`, `ResourcePackItem`, `ShaderPackItem`, `WorldItem` — each flattens a shared `ContentBase` (title, slug, downloads, loaders, categories, authors, license, timestamps, ...). A CurseForge mod and a Modrinth mod look identical to your code.
- **Top-level helpers *or* trait-based extension.** Use `find_mods(…, Platform, …)` for quick dispatch, or take a `&impl ModProvider` when you want code that is generic over the source.
- **Fully async.** Built on `tokio` + `reqwest` with pooled connections per platform (`rustls`, gzip). One `reqwest::Client` per platform via `OnceLock`.
- **Expansible.** Adding a platform is a new struct + a few trait impls + a match arm. Adding a content kind is a new trait + struct + dispatch fns. No sweeping API churn.
- **Polite by default.** Every provider sends a descriptive `User-Agent`; use `Provider::with_user_agent("my-app/1.0")` to brand your own (Modrinth's ToS asks for this).

## Installation

```toml
[dependencies]
hopper-mc = "0.1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

MSRV: **Rust 1.85** (edition 2024, native `async fn` in traits).

## Quick start

```rust
use hopper_mc::{find_mods, get_mod, Platform, Sort};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Browse the top mods on Modrinth by download count.
    let top = find_mods(None, Sort::Downloads, Platform::Modrinth, 0, 10).await?;
    for m in &top {
        println!("{:<30} {:>10} downloads", m.base.title, m.base.downloads);
    }

    // Search for a specific string.
    let hits = find_mods(Some("fabric api"), Sort::Relevance, Platform::Modrinth, 0, 5).await?;
    println!("Found {} hits for \"fabric api\"", hits.len());

    // Fetch a single project by slug (or id).
    if let Some(sodium) = get_mod("sodium", Platform::Modrinth).await? {
        println!("Sodium supports loaders: {:?}", sodium.loaders);
    }

    Ok(())
}
```

## The API at a glance

```rust
// Discovery — all async, all return Result<Vec<XItem>>
find_mods          (q, sort, platform, page, per_page)
find_packs         (q, sort, platform, page, per_page)
find_datapacks     (q, sort, platform, page, per_page)
find_resourcepacks (q, sort, platform, page, per_page)
find_shaderpacks   (q, sort, platform, page, per_page)
find_worlds        (q, sort, platform, page, per_page)

// Single-item lookup — return Result<Option<XItem>>
get_mod          (id_or_slug, platform)
get_pack         (id_or_slug, platform)
get_datapack     (id_or_slug, platform)
get_resourcepack (id_or_slug, platform)
get_shaderpack   (id_or_slug, platform)
get_world        (id_or_slug, platform)
```

```rust
enum Platform    { Modrinth, CurseForge, AtLauncher, Technic, Ftb }
enum Sort        { Relevance, Downloads, Follows, Latest, Updated }
enum ContentType { Mod, Modpack, Datapack, ResourcePack, ShaderPack, World }
```

## Errors

All fallible calls return `Result<_, ContentError>`:

```rust
pub enum ContentError {
    Http(reqwest::Error),
    Decode(serde_json::Error),
    NotFound(String),
    RateLimited(Option<Duration>),
    UnsupportedContentType { platform: Platform, kind: ContentType },
    NotImplemented(Platform),
    BadRequest(String),
    Unexpected(String),
}
```

Two variants worth calling out:

- **`UnsupportedContentType`** — the platform does not serve this kind of content (e.g. Modrinth + `World`, or AT Launcher + `Mod`). No network request is made.
- **`NotImplemented`** — the provider exists but its method bodies are stubs in this release. Wiring real calls is body-only; signatures are stable.

## Using providers directly

The top-level dispatch functions are thin wrappers around per-platform provider singletons. You can hold one directly to avoid repeating the `Platform` argument, supply a custom `User-Agent`, or reuse a `reqwest::Client`:

```rust
use hopper_mc::{ModProvider, ModrinthProvider, Sort};

let modrinth = ModrinthProvider::with_user_agent("my-launcher/1.0 (contact@example.com)");
let mods = modrinth.find_mods(Some("sodium"), Sort::Relevance, 0, 5).await?;
```

Write code generic over the source:

```rust
use hopper_mc::{ContentError, ModItem, ModProvider, Sort};

async fn first_hit<P: ModProvider>(p: &P, q: &str) -> Result<Option<ModItem>, ContentError> {
    Ok(p.find_mods(Some(q), Sort::Relevance, 0, 1).await?.pop())
}
```

## Examples

The [`examples/`](./examples) directory is runnable:

```sh
cargo run --example find_mods                      # browse top mods
cargo run --example find_mods -- "fabric api"      # search by query
cargo run --example get_mod                        # defaults to fabric-api
cargo run --example get_mod -- sodium              # by slug
cargo run --example browse_content_types           # every content kind
cargo run --example custom_provider_trait          # generic over provider
```

## Extending: add a new hopper input

1. Add a variant to `Platform`:

   ```rust
   pub enum Platform { Modrinth, CurseForge, AtLauncher, Technic, Ftb, MyPlatform }
   ```

2. Add a module under `src/platforms/my_platform.rs` with a `struct MyProvider` that implements `ContentProvider` plus the per-kind traits (`ModProvider`, `PackProvider`, ...) it can serve.

3. Re-export it from `src/platforms/mod.rs` and add match arms to the relevant `find_*`/`get_*` functions in `src/lib.rs`. Platforms that don't serve a given kind hit the `UnsupportedContentType` branch automatically.

Skeleton platforms in this crate (CurseForge, AT Launcher, TechnicPack, FTB) are concrete working examples — browse their source for the pattern.

## Design notes

- **Flattened models.** `ModItem` embeds `ContentBase` via `#[serde(flatten)]` so call sites read `item.base.title` (typed, explicit) without extra wrapping, while JSON round-trips remain a single flat object.
- **Per-kind traits, not per-platform traits.** Platforms that don't serve a content kind simply don't implement its trait; the dispatch layer maps the gaps to `UnsupportedContentType`.
- **No dyn trait.** Dispatch is a static `match` with one pooled `reqwest::Client` per platform via `OnceLock`. If you need runtime-variable providers, hold each provider directly.
- **Search vs. get.** Modrinth's `/search` returns a thinner view than `/project/{id}`; the mapping layer is explicit about which fields are populated by which call (the long `description` is `None` on search hits, for example).

## Related

`hopper-mc` is a companion to [`piston-mc-lib`](https://crates.io/crates/piston-mc) — the launcher API library that *pushes* Minecraft data, while `hopper-mc` *pulls* content from the ecosystem.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
