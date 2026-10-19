# Lodestone Fabric Loader
This crate will handle installing and managing the Fabric mod loader in Lodestone instances.
## What this crate will do:
- Resolve available Fabric loader versions for a given Minecraft version
- Install the Fabric profile for an instance
- Upgrade or remove an existing Fabric install

## What this crate will **NOT** do:
- Launch the game (see `lodestone_launch`)
- Download or manage Fabric mods (see the platform crates)
- Install other mod loaders

Loader crates are interchangeable: they install a playable client for an instance and nothing more.
