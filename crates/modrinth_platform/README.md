# Lodestone Modrinth Platform
This crate will handle all interaction with the Modrinth API for mods, modpacks, shaders, resource packs, and data packs.
## What this crate will do:
- Search Modrinth projects and fetch project, version, and file metadata
- Download project files, including dependency resolution
- Parse `.mrpack` modpack files into installable modpack data

## What this crate will **NOT** do:
- Install content into instances (callers decide where files go)
- Provide the unified cross-platform content browser
- Talk to any other platform's API

Every platform crate exposes the same surface so the launcher can treat them interchangeably.
