# Lodestone Minecraft Authentication
Microsoft → Xbox Live → Minecraft services authentication for Lodestone.

- Device-code login: gui shows a URL + short code, the user confirms on any device
- Silent re-login from a stored refresh token
- Sessions carry the Minecraft access token, expiry, refresh token, and profile (uuid + name)
- Xbox sign-in failures surface as user-presentable errors (no Xbox account, child
  account needs family setup, region unsupported, …) instead of raw Xbox `XErr` codes

## Wheels reused, not reinvented:
- `oauth2` — the Microsoft OAuth leg (device code + refresh token exchange)
- `minecraft-msa-auth` — the Xbox Live → XSTS → Minecraft services chain
- The Minecraft profile fetch (`/minecraft/profile`) is the only endpoint implemented
  here

## What this crate will NOT do:
- Persist tokens (it returns [`Session`]s; gui owns storage)
- Own the Azure app registration — `client_id` is a constructor argument
- Launch the game (see `lodestone_launch`)

The module layout and public API contract are specified in [DESIGN.md](DESIGN.md).
