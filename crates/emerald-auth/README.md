# emerald-auth

Standalone Microsoft OAuth2 authentication library for Minecraft launchers.

Handles the full authentication chain:

**Microsoft OAuth → Xbox Live → XSTS → Minecraft access token + profile**

## Features

- Opens the Microsoft login page in the user's default browser
- Captures the OAuth redirect via a temporary localhost TCP server
- Exchanges tokens through the full Microsoft → Xbox Live → XSTS → Minecraft chain
- Returns the player's Minecraft profile (username, UUID, skins, capes)
- Token refresh support (re-authenticate without opening the browser)
- All tokens wrapped in `SecretString` for memory safety
- CSRF protection via random `state` parameter
- Lower-level API for fine-grained control over each auth step
- No framework dependencies for the callback server (raw TCP)
- Cross-platform (Windows, macOS, Linux)

## Quick Start

```rust
use emerald_auth::MicrosoftAuth;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let auth = MicrosoftAuth::new("your-azure-client-id");
    let profile = auth.authenticate().await?;

    println!("Logged in as {} ({})", profile.username, profile.uuid);
    Ok(())
}
```

## Token Refresh

After the initial authentication, you receive a refresh token that can be used
to re-authenticate without opening the browser:

```rust
use emerald_auth::MicrosoftAuth;
use secrecy::SecretString;

async fn refresh(client_id: &str, refresh_token: &SecretString) -> emerald_auth::Result<()> {
    let auth = MicrosoftAuth::new(client_id);
    let profile = auth.refresh(refresh_token).await?;
    println!("Refreshed as {}", profile.username);
    Ok(())
}
```

## Builder Configuration

```rust
use std::time::Duration;
use emerald_auth::MicrosoftAuth;

let auth = MicrosoftAuth::new("client-id")
    .with_timeout(Duration::from_secs(120))  // Custom timeout (default: 5 min)
    .with_port(25585)                         // Fixed port (default: OS-assigned)
    .with_http_client(reqwest::Client::new()); // Custom HTTP client
```

## Lower-Level API

Each step of the auth chain is exposed as a standalone public function:

```rust
use emerald_auth::{callback, microsoft, xbox, minecraft};

// Each function can be called independently:
// microsoft::build_auth_url(client_id, redirect_uri, state)
// microsoft::exchange_code(client, client_id, code, redirect_uri)
// microsoft::refresh_tokens(client, client_id, refresh_token)
// xbox::authenticate_xbox_live(client, microsoft_token)
// xbox::authenticate_xsts(client, xbox_token)
// minecraft::authenticate_minecraft(client, xsts_token, user_hash)
// minecraft::check_ownership(client, minecraft_token)
// minecraft::fetch_profile(client, minecraft_token)
```

See the [`step_by_step`](examples/step_by_step.rs) example for a complete walkthrough.

## Azure App Registration

To use this library, you need a Microsoft Azure application:

1. Go to [Azure Portal](https://portal.azure.com) → Microsoft Entra ID → App registrations
2. Click **New registration**
3. Under **Redirect URI**, select **Mobile and desktop applications** and set the URI to `http://localhost`
4. Copy the **Application (client) ID** — this is your `client_id`
5. Under **API permissions**, ensure `XboxLive.signin` is granted

## Authentication Flow

```
┌──────────┐     ┌─────────┐     ┌───────────┐     ┌──────┐     ┌───────────┐
│  Browser  │────>│Microsoft│────>│  Callback  │────>│ Xbox  │────>│ Minecraft │
│  Login    │     │  OAuth  │     │  Server    │     │ Live  │     │ Services  │
└──────────┘     └─────────┘     └───────────┘     └──────┘     └───────────┘
     │                │                │                │              │
     │  User logs in  │  Auth code     │  MS tokens     │  MC token    │
     │  via browser   │  via redirect  │  → Xbox → XSTS │  + profile   │
```

1. A localhost TCP server starts on an ephemeral port
2. The Microsoft OAuth URL opens in the default browser
3. User signs in and is redirected to `http://localhost:{port}/callback?code=...`
4. The callback server extracts the authorization code
5. The code is exchanged for Microsoft access + refresh tokens
6. Microsoft token → Xbox Live token (with user hash)
7. Xbox Live token → XSTS token
8. XSTS token → Minecraft access token
9. Minecraft token → profile fetch (username, UUID, skins, capes)

## Error Handling

The library provides detailed error types for each failure point:

```rust
use emerald_auth::AuthError;

match auth.authenticate().await {
    Ok(profile) => println!("Welcome, {}!", profile.username),
    Err(AuthError::NoGameOwnership) => {
        println!("This account doesn't own Minecraft.");
    }
    Err(AuthError::Xsts { xerr, message }) => {
        // Xbox-specific errors (underage account, regional blocks, etc.)
        println!("Xbox error {xerr}: {message}");
    }
    Err(AuthError::Timeout(duration)) => {
        println!("Login timed out after {duration:?}");
    }
    Err(AuthError::OAuth { error, description }) => {
        println!("OAuth error: {error} — {description}");
    }
    Err(e) => println!("Authentication failed: {e}"),
}
```

### XSTS Error Codes

| Code | Meaning |
|------|---------|
| `2148916233` | No Xbox account — needs to create one |
| `2148916235` | Xbox Live unavailable in country/region |
| `2148916236` | Needs adult verification |
| `2148916238` | Child account — needs Microsoft Family |

## Security

- **Token secrecy**: All tokens are wrapped in `secrecy::SecretString`, which redacts them from `Debug` output and zeroes memory on drop
- **CSRF protection**: A random 32-byte `state` parameter is generated and verified on callback
- **Localhost only**: The callback server binds to `127.0.0.1`, never `0.0.0.0`
- **Ephemeral port**: OS-assigned port by default to minimize attack surface
- **Single-use server**: The TCP server accepts exactly one connection and shuts down
- **No token storage**: The library returns tokens but never persists them — storage is the caller's responsibility

## Examples

```sh
# Full authentication flow
cargo run --example authenticate -- <CLIENT_ID>

# Re-authenticate with a refresh token
cargo run --example refresh -- <CLIENT_ID> <REFRESH_TOKEN>

# Step-by-step lower-level API walkthrough
cargo run --example step_by_step -- <CLIENT_ID>

# Fixed callback port
cargo run --example custom_port -- <CLIENT_ID> 25585
```

Enable debug logging with `RUST_LOG=emerald_auth=debug` for verbose output.
