# Lodestone Forge Loader
This crate will handle installing and managing the MinecraftForge mod loader in Lodestone instances.
## What this crate will do:
- Resolve available Forge versions for a given Minecraft version
- Install the Forge profile for an instance (running the Forge installer headlessly where required)
- Upgrade or remove an existing Forge install

## What this crate will **NOT** do:
- Launch the game (see `lodestone_launch`)
- Download or manage Forge mods (see the platform crates)
- Install other mod loaders

Loader crates are interchangeable: they install a playable client for an instance and nothing more.
