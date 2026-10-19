# Lodestone Platforms
One crate, one struct, every content platform: `Platform<Modrinth>`, `Platform<CurseForge>`,
`Platform<AtLauncher>`, `Platform<Ftb>`, `Platform<Technic>` — all with the same function
surface.

- Search content by query and content type
- Fetch details for a project or pack
- List versions (mod files / pack builds) for a project or pack
- Download a version's files (with progress channels)
- Parse modpack archives (`.mrpack`, CurseForge exports) into installable modpack data

Platforms declare their capabilities: pack-only platforms report
`UnsupportedContentType` for mods and shaders, and only platforms with an archive
format support `parse_modpack`.

## Wheels reused, not reinvented:
- `ferinth` — the Modrinth API (search, projects, versions, files)
- `simple_download_utility` — downloads with progress
- `zipcrawl` — reading modpack archives
- Hand-rolled thin bindings only where no crate exists: CurseForge, ATLauncher, FTB,
  and Technic APIs

## What this crate will NOT do:
- Install content into instances (callers decide where files go)
- Provide the unified cross-platform content browser (gui's job)
- Launch the game (see `lodestone_launch`) or resolve loaders (see `lodestone_loaders`)

The module layout and public API contract are specified in [DESIGN.md](DESIGN.md).
