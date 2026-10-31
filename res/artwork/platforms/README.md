# Platform logos

The marks the Discover page shows for each content platform. The files are embedded at compile time by `crates/gui/src/app/components/platform_logo.rs`.

| File | Platform | Source | Licence |
|---|---|---|---|
| `modrinth.svg` | Modrinth | Iconify `simple-icons:modrinth`, `https://api.iconify.design/simple-icons/modrinth.svg` | CC0 1.0 (Simple Icons) |
| `curseforge.svg` | CurseForge | Iconify `simple-icons:curseforge`, `https://api.iconify.design/simple-icons/curseforge.svg` | CC0 1.0 (Simple Icons) |
| `atlauncher.svg` | ATLauncher | `https://atlauncher.com/assets/images/logo.svg` | The project's mark, used only to name the platform |
| `ftb.svg` | FTB | `https://www.feed-the-beast.com/_next/static/media/ftb-logo-no-footing.2rxql1xh1_lfd.svg` | The project's mark, used only to name the platform |
| `technic.ico` | Technic | `https://www.technicpack.net/favicon.ico` | The project's mark, used only to name the platform. No vector logo was found |

Modrinth and CurseForge are monochrome: `platform_logo.rs` colours their `currentColor` fills with each platform's brand colour. The other marks keep their own colours. Technic's favicon is decoded from its ICO file.
