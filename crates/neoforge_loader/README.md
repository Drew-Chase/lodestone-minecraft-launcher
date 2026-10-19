# Lodestone NeoForge Loader
This crate will handle installing and managing the NeoForge mod loader in Lodestone instances.
## What this crate will do:
- Resolve available NeoForge versions for a given Minecraft version
- Install the NeoForge profile for an instance
- Upgrade or remove an existing NeoForge install

## What this crate will **NOT** do:
- Launch the game (see `lodestone_launch`)
- Download or manage NeoForge mods (see the platform crates)
- Install other mod loaders

Loader crates are interchangeable: they install a playable client for an instance and nothing more.
