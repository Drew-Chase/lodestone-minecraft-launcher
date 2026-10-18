# Lodestone Quilt Loader
This crate will handle installing and managing the Quilt mod loader in Lodestone instances.
## What this crate will do:
- Resolve available Quilt loader versions for a given Minecraft version
- Install the Quilt profile for an instance
- Upgrade or remove an existing Quilt install

## What this crate will **NOT** do:
- Launch the game (see `lodestone_launcher_core`)
- Download or manage Quilt mods (see the platform crates)
- Install other mod loaders

Loader crates are interchangeable: they install a playable client for an instance and nothing more.
