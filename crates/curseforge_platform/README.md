# Lodestone CurseForge Platform
This crate will handle all interaction with the CurseForge API for mods, modpacks, shaders, resource packs, and worlds.
## What this crate will do:
- Search CurseForge projects and fetch project, file, and dependency metadata
- Download project files
- Parse CurseForge modpack exports (`.zip` with `manifest.json`) into installable modpack data

## What this crate will **NOT** do:
- Install content into instances (callers decide where files go)
- Provide the unified cross-platform content browser
- Talk to any other platform's API

Every platform crate exposes the same surface so the launcher can treat them interchangeably.
