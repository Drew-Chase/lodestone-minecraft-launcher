# Lodestone Loaders
One crate, one struct, every loader: `Loader<Fabric>`, `Loader<Forge>`, `Loader<NeoForge>`,
`Loader<Quilt>`, `Loader<Vanilla>` — all with the same function surface.

- List the loader's versions
- Get versions for a Minecraft version
- Download the loader's client libraries
- Extract information from a mod file (modid, name, version, description, authors, license, homepage, dependencies)

This crate delegates to purpose-built crates instead of reinventing them: `piston-mc`
(vanilla version manifest + client download), `simple_download_utility` (downloads with
progress), and `modcrawl` (mod metadata for Fabric / Forge / NeoForge / legacy
`mcmod.info`, plus dependency analysis). Only `quilt.mod.json` is parsed here, because
nothing upstream covers it yet.

## What this crate will NOT do:
- Launch the game (see `lodestone_launch`)
- Install loader profiles for instances (merging version JSONs) — a follow-up slice
- Download or manage mods (see the platform crates)
- Authenticate (see `auth`)

The module layout and public API contract are specified in [DESIGN.md](DESIGN.md).
