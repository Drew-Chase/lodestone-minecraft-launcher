# Lodestone Aurora Leaf
This crate pins the icon sets and font families the launcher uses, re-exported as ready-to-use iced types.
## What this crate will do:
- Generate the Lucide and Material Symbols icon sets via `aurora_iconify`
- Register the Roboto font family via `aurora_fonts`
- Re-export `Icon` and `FontFamily` for the rest of the workspace

## What this crate will **NOT** do:
- Draw or lay out any UI (that is `lodestone_launcher`'s job)
- Touch any launcher, loader, or platform logic

This is the only place the launcher meets the `aurora_*` ecosystem — pin or swap icon and font sets here and the whole UI follows.
