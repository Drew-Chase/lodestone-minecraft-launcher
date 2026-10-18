<p align="center">
  <img src="res/icon/icon.svg" alt="Lodestone Logo" width="80" />
</p>

<h1 align="center">Lodestone Minecraft Launcher</h1>

<p align="center">
  A modern, cross-platform Minecraft launcher built entirely in Rust.
  <br />
  Manage instances, discover mods, sync with friends, and launch the game — all from one place.
</p>

---

## Screenshots

> Design references for the new fully-native interface.

<p align="center">
  <img src="res/reference-images/01-login.png" alt="Login Screen" width="720" />
  <br /><em>Sign in with Microsoft, play offline, or explore in demo mode.</em>
</p>

<p align="center">
  <img src="res/reference-images/02-library.png" alt="Library" width="720" />
  <br /><em>Your instance library — launch, manage, and organize your Minecraft profiles.</em>
</p>

<p align="center">
  <img src="res/reference-images/03-discover.png" alt="Discover" width="720" />
  <br /><em>Browse mods, modpacks, shaders, resource packs, and more from Modrinth, CurseForge, ATLauncher, FTB, and Technic.</em>
</p>

<p align="center">
  <img src="res/reference-images/04-instance.png" alt="Instance Page" width="720" />
  <br /><em>Per-instance detail — mod management with enable/disable toggles.</em>
</p>

<p align="center">
  <img src="res/reference-images/05-worlds.png" alt="Worlds" width="720" />
  <br /><em>World management — browse, back up, and jump into your saves.</em>
</p>

<p align="center">
  <img src="res/reference-images/06-servers.png" alt="Servers" width="720" />
  <br /><em>Server browser — saved servers, ping, and player counts, with direct launch into multiplayer.</em>
</p>

<p align="center">
  <img src="res/reference-images/07-friends.png" alt="Friends" width="720" />
  <br /><em>Friends — see who's online and what they're playing.</em>
</p>

<p align="center">
  <img src="res/reference-images/08-downloads.png" alt="Downloads" width="720" />
  <br /><em>Download manager — track mod, modpack, and loader downloads in one place.</em>
</p>

### Filters

<p align="center">
  <img src="res/reference-images/09-filter-popover.png" alt="Filter Popover" width="540" />
  <br /><em>Quick filters, one click away.</em>
</p>

<p align="center">
  <img src="res/reference-images/10-filter-drawer.png" alt="Filter Drawer" width="540" />
  <br /><em>The full filter drawer for narrowing things down.</em>
</p>

### Modals

<p align="center">
  <img src="res/reference-images/11-modal-new-instance.png" alt="New Instance Modal" width="640" />
  <br /><em>Create a new instance — pick your version and mod loader.</em>
</p>

<p align="center">
  <img src="res/reference-images/12-modal-import.png" alt="Import Instance Modal" width="640" />
  <br /><em>Import instances from `.zip`, `.mrpack`, `.mcpack` files or URLs.</em>
</p>

<p align="center">
  <img src="res/reference-images/13-modal-join-server.png" alt="Join Server Modal" width="640" />
  <br /><em>Quick-connect to servers with a saved server list and ping info.</em>
</p>

<p align="center">
  <img src="res/reference-images/14-modal-coop-sync.png" alt="Co-op Sync Modal" width="640" />
  <br /><em>Co-op Sync — mirror a friend's mods, shaders, and instance, then join their world.</em>
</p>

### Settings

| General | Accounts |
|---------|----------|
| <img src="res/reference-images/15-settings-general.png" alt="General Settings" width="100%" /> <em>General settings.</em> | <img src="res/reference-images/16-settings-accounts.png" alt="Accounts Settings" width="100%" /> <em>Manage your Microsoft and offline accounts.</em> |
| **Java** | **Appearance** |
| <img src="res/reference-images/17-settings-java.png" alt="Java Settings" width="100%" /> <em>Java runtimes — detection and per-instance matching.</em> | <img src="res/reference-images/18-settings-appearance.png" alt="Appearance Settings" width="100%" /> <em>Make it yours.</em> |
| **Network** | **Privacy** |
| <img src="res/reference-images/19-settings-network.png" alt="Network Settings" width="100%" /> <em>Network configuration.</em> | <img src="res/reference-images/20-settings-privacy.png" alt="Privacy Settings" width="100%" /> <em>Privacy controls.</em> |

### Website

<p align="center">
  <img src="res/reference-images/21-website.png" alt="Website" width="720" />
  <br /><em>The Lodestone website — the launcher is free; Sync is optional.</em>
</p>

---

## Features

- **Instance Management** — Create, import, duplicate, and organize Minecraft instances.
- **Mod Loaders** — Vanilla plus first-class support for Fabric, Forge, NeoForge, and Quilt.
- **Content Discovery** — Unified search across Modrinth, CurseForge, ATLauncher, FTB, and Technic Pack in a single browser.
- **Mod Management** — Per-instance mod lists with enable/disable toggles, drag-and-drop import, and one-click updates.
- **Co-op Sync** — Mirror a friend's instance (mods, shaders, resource packs) and join their world in one click.
- **Server Browser** — Save servers, check ping and player counts, and launch directly into multiplayer.
- **Friends** — See who's online, what they're playing, and join along.
- **World Management** — Browse, back up, and manage your saves.
- **Download Manager** — Track every mod, modpack, and loader download in one place.
- **Microsoft Authentication** — Secure OAuth2 login via Microsoft, with offline and demo mode options.
- **Instance Import/Export** — Import from `.zip`, `.mrpack`, `.mcpack`, CurseForge URLs, or backup archives.
- **Auto Java Detection** — Automatically detects and matches the correct Java version for each instance.
- **Cross-Platform** — Native performance on Windows, macOS, and Linux.
- **Free** — The launcher is free. Sync is optional.

---

## Architecture

Lodestone is a pure-Rust Cargo workspace — the UI included. There is no JavaScript anywhere; the desktop interface is built with [iced](https://github.com/iced-rs/iced).

| Crate | Directory | Description |
|-------|-----------|-------------|
| **lodestone_launcher** | `crates/gui` | Desktop application — the full launcher UI, built with iced |
| **lodestone_launcher_core** | `crates/core` | Headless launcher core — calls Java, handles game arguments, starts the Minecraft client |
| **lodestone_minecraft_authentication** | `crates/auth` | Microsoft / Xbox Live authentication for Minecraft |
| **lodestone_vanilla_loader** | `crates/vanilla_loader` | Vanilla Minecraft installation and management — the baseline every instance starts from |
| **lodestone_fabric_loader** | `crates/fabric_loader` | Fabric loader installation and management |
| **lodestone_forge_loader** | `crates/forge_loader` | Forge loader installation and management |
| **lodestone_neoforge_loader** | `crates/neoforge_loader` | NeoForge loader installation and management |
| **lodestone_quilt_loader** | `crates/quilt_loader` | Quilt loader installation and management |
| **lodestone_modrinth_platform** | `crates/modrinth_platform` | Modrinth API — search, downloads, `.mrpack` modpack imports |
| **lodestone_curseforge_platform** | `crates/curseforge_platform` | CurseForge API — search, downloads, modpack imports |
| **lodestone_atlauncher_platform** | `crates/atlauncher_platform` | ATLauncher API — packs and mods |
| **lodestone_ftb_platform** | `crates/ftb_platform` | FTB modpack API — packs and mods |
| **lodestone_technic_platform** | `crates/technic_platform` | Technic / Solder API — packs and mods |

Each platform crate speaks its own API behind a consistent surface, and each loader crate installs a playable client for an instance — see the individual crate READMEs for exact boundaries.

### Tech Stack

- **Language**: Rust (2024 edition) — the entire project, UI included
- **UI**: [iced](https://github.com/iced-rs/iced) — Elm-style GUI in Rust
- **Build**: Cargo workspace, [`just`](https://github.com/casey/just) recipes, Docker-based builds
- **Formatting**: `rustfmt.toml`

---

## Status

Lodestone is in early development. The workspace is scaffolded and each crate's README documents what it will handle — implementation is underway.

---

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [`just`](https://github.com/casey/just) (optional, for build tasks)

### Build

```bash
# Clone the repository
git clone https://github.com/drew-chase/lodestone-minecraft-launcher.git
cd lodestone-minecraft-launcher

# Build the workspace
cargo build --release

# Or see the available build tasks
just --list
```

---

## License

This project is open source. See the repository for license details.
