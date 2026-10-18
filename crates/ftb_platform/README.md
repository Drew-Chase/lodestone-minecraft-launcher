# Lodestone FTB Platform
This crate will handle all interaction with the FTB modpack API for packs and mods.
## What this crate will do:
- Query the FTB modpack API for available packs and versions
- Download pack files and resolve dependencies
- Parse FTB pack definitions into installable modpack data

## What this crate will **NOT** do:
- Install content into instances (callers decide where files go)
- Provide the unified cross-platform content browser
- Talk to any other platform's API

Every platform crate exposes the same surface so the launcher can treat them interchangeably.
