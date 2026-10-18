# Lodestone Vanilla Loader
This crate will handle installing and managing vanilla Minecraft versions in Lodestone instances.
## What this crate will do:
- Resolve available vanilla Minecraft versions (releases and snapshots)
- Install the vanilla client jar, version metadata, and assets for an instance
- Upgrade or remove an existing vanilla install

## What this crate will **NOT** do:
- Launch the game (see `lodestone_launcher_core`)
- Install mod loaders (Fabric, Forge, NeoForge, Quilt)
- Manage mods or other downloadable content

This is the baseline loader every instance starts from.
