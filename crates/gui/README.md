# Lodestone Minecraft Launcher GUI
This is the full launcher gui interface for windows, mac and linux.  This is built with the iced-rs library.
## What this crate will do:
- Render the entire launcher interface using iced (Elm-style model / `Message` / `update` / `view`)
- Own the window shell — custom titlebar, borderless resize handles, and per-platform window settings
- Call `piston-mc` directly for Minecraft assets, downloads, and Java runtimes
- Set up logging (tracing + color-eyre) and generate the build number and Windows icon at compile time
- Create instances (`app/install`): the New Instance dialog drives `piston-mc` for the Java runtime, client, libraries and assets, and `lodestone_loaders` for the loader, then writes the instance folder and its database row

## What this crate will **NOT** do:
- Launch the game (see `lodestone_launch`)
- Handle authentication (see `lodestone_minecraft_authentication`)
- Install loaders itself (`lodestone_loaders` does it) or query content platforms

This is the workspace's only binary crate — every other crate is a library it composes.
