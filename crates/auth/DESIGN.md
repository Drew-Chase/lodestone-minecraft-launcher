# Lodestone Minecraft Authentication — Design

Design specification for `lodestone_minecraft_authentication` (`crates/auth`), the
workspace's Microsoft/Xbox/Minecraft authentication module. Where
[README.md](README.md) disagrees, this document is more specific.

---

## 1. Role and architectural position

`lodestone_minecraft_authentication` turns a Microsoft account into a playable
Minecraft session:

```
gui (shows code / stores session)
  ──▶ Authentication (this crate)
        ├── Microsoft OAuth device code + refresh  ──▶ oauth2
        ├── Xbox Live / XSTS / Minecraft services  ──▶ minecraft-msa-auth
        └── Minecraft profile                      ──▶ this crate (one GET)
```

### Hard rules

- **Standalone leaf crate.** Zero path dependencies on workspace siblings; only
  `crates/gui` links to workspace crates.
- **Async, tokio-based** — iced `Task`s await it directly.
- **Dumb about storage.** It returns [`Session`]s and does nothing else: token
  persistence, keychains, and config files are gui's responsibility.
- **The Azure app is not ours.** `client_id` is a constructor argument; the app
  registration (with device-code flow enabled) is a gui/deployment concern.
- **Wheels are not reinvented** (see §2).

## 2. Wheel audit

| Job | Crate used | Why |
|---|---|---|
| Microsoft OAuth (device code, refresh token exchange) | `oauth2` 5.0 (no default features) | The standard Rust OAuth client; its optional `reqwest` integration pins reqwest 0.12, so this crate implements oauth2's `AsyncHttpClient` for the workspace's reqwest 0.13 client instead (~30 lines, no duplicate HTTP stack) |
| Xbox Live → XSTS → Minecraft services | `minecraft-msa-auth` 0.5 | gorilla-devs-adjacent (`minecraft-rs`), actively maintained, reqwest 0.13, surfaces Xbox `XErr` codes |
| Minecraft profile | hand-rolled | One authenticated GET; nothing upstream does it cleanly |

Whole-launcher libraries (portablemc, lyceris, cobble-core, crust_core, mc-launcher-core)
carry auth internally but were rejected — they drag a parallel launcher architecture
into the workspace.

## 3. Module layout

```
crates/auth/
├── Cargo.toml              # lodestone_minecraft_authentication
├── README.md               # short contract
├── DESIGN.md               # this document
├── src/
│   ├── lib.rs              # crate docs + pub struct Authentication + docs of the flow
│   ├── error.rs            # AuthError (user-presentable, XErr-mapped)
│   ├── session.rs          # Session, Profile, DeviceCodeStart
│   ├── http.rs             # oauth2 AsyncHttpClient adapter for reqwest 0.13 (private)
│   └── endpoints.rs        # Microsoft/Minecraft endpoint constants (private)
└── examples/
    └── device_login.rs     # interactive live smoke test
```

Pure helpers (XErr mapping, payload parsing) are unit-tested in `#[cfg(test)]` modules
inside their source files, mirroring the other crates' pattern.

## 4. Public API

```rust
impl Authentication {
    pub fn new(client_id: impl Into<String>) -> Self;

    /// Two-phase device-code login: `begin` returns what the user must see
    /// (verification URL + short code); `finish` polls Microsoft until the user
    /// completes the sign-in, then resolves the full session. Cancelling the
    /// awaited future cancels the poll; the code itself expires server-side
    /// (surfaced as `AuthError::DeviceCodeExpired`).
    pub async fn begin_device_code_login(&self) -> Result<DeviceCodeStart, AuthError>;
    pub async fn finish_device_code_login(&self, start: DeviceCodeStart) -> Result<Session, AuthError>;

    /// Silent re-login from a stored refresh token.
    pub async fn login_with_refresh_token(&self, refresh_token: &str) -> Result<Session, AuthError>;
}

pub struct Session {
    pub minecraft_token: String,          // bearer for Minecraft services / launch
    pub expires_at: u64,                  // unix seconds
    pub refresh_token: Option<String>,    // persist this for silent re-login
    pub profile: Profile,
}

pub struct Profile { pub id: String, pub name: String }   // id: dash-less Minecraft uuid

pub struct DeviceCodeStart { pub user_code: String, pub verification_uri: String, pub expires_in: u64 }
```

`Session` is `Clone` + `Send` + `Sync`. The `minecraft_token` is the bearer token for
`api.minecraftservices.com` — `lodestone_launch`'s callers pass it into game args.

## 5. Auth chain

1. **Microsoft device code** (`login.microsoftonline.com/consumers`, scopes
   `XboxLive.signin offline_access`): `begin` fetches the code; `finish` polls the
   token endpoint (oauth2 handles the interval + retry) until the user completes
   sign-in or the code expires.
2. **Xbox Live** (`user.auth.xboxlive.com`) → XBL token.
3. **XSTS** (`xsts.auth.xboxlive.com`) → XSTS token + user hash.
   Failures surface as mapped Xbox errors (§6).
4. **Minecraft services** (`api.minecraftservices.com/authentication/login_with_xbox`)
   → Minecraft bearer token + expiry (via `minecraft-msa-auth`).
5. **Profile** (`/minecraft/profile`) → uuid + name. **404 → `MinecraftNotOwned`**
   (the account has no Minecraft Java profile).

`login_with_refresh_token` skips step 1's user interaction: the refresh token
exchanges for a fresh Microsoft token, then steps 2–5 run as normal.

## 6. Xbox error mapping

XSTS failures carry numeric `XErr` codes. `minecraft-msa-auth` maps some directly;
the rest surface as `XboxLive { code }` and are mapped here:

| XErr | `AuthError` variant | User message |
|---|---|---|
| (mapped upstream) | `NoXboxAccount` | no Xbox account attached to the Microsoft account |
| 2148916234 | `XboxLiveUnavailable` | Xbox Live is not available in this country/region |
| 2148916235 | `RegionUnsupported` | the account's region does not support Xbox Live |
| 2148916236 / 2148916237 | `AdultVerificationRequired` | adult verification is required (South Korea) |
| (mapped upstream) | `ChildAccount` | the account is a minor and must be added to a Microsoft family |
| anything else | `XboxLiveError { code }` | raw code, for reporting |

## 7. Errors (`error.rs`)

`AuthError`, thiserror, user-presentable:

| Variant | When |
|---|---|
| `Network` / `UnexpectedResponse` / `Json` | transport / HTTP status / body failures (url + context attached) |
| `Microsoft` | Microsoft OAuth failure (device-code request, token exchange, refresh) |
| `DeviceCodeExpired` | the device code expired before sign-in completed |
| `AuthorizationDenied` | the user denied the sign-in request |
| `NoXboxAccount` / `XboxLiveUnavailable` / `RegionUnsupported` / `AdultVerificationRequired` / `ChildAccount` / `XboxLiveError { code }` | Xbox sign-in failures (§6) |
| `MinecraftAuth` | the Minecraft services exchange failed (via `minecraft-msa-auth`) |
| `MinecraftNotOwned` | the account has no Minecraft Java profile |

## 8. Testing strategy

- Unit tests inside `error.rs` (XErr → variant mapping) and `session.rs` (profile
  payload parsing, expiry arithmetic) use recorded real payloads.
- The device-code flow needs a real Azure app + interactive sign-in, so it is
  exercised by `examples/device_login.rs` (manual smoke): pass a `client_id`, open
  the URL, enter the code, and the example prints the profile and session expiry.
