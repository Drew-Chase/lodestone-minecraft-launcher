# lodestone_export

Writes an instance out as a package other people can open:

- `format::mrpack` — a Modrinth pack (`.mrpack`), with an index and an `overrides/` folder.
- `format::curseforge` — a CurseForge pack, with a manifest that lists mods by project and file id.
- `format::plain` — a zip of the selected files, with a `lodestone.json` at the root.
- `format::lodestone` — a `.lodestone` package: a tar in an xz container at preset 9, with `lodestone.json` and
  `overrides/`. It also reads the manifest and extracts the overrides.
- `portable` — a portable launcher: a copy of the running launcher with a package appended. `locate` finds the package
  again at startup.

The crate reads the instance folder and writes files. The caller resolves each file's links and passes them in as
`link::Entry` values, so nothing here touches the network or iced. Writers check for cancellation between files and
report progress through `progress::Control`.

The GUI's side lives in `crates/gui/src/app/export/`: the dialog, the folder tree, and the job that plans, hashes,
identifies and writes the package.
