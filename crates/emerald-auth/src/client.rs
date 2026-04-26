use std::time::Duration;

use rand::Rng;
use secrecy::SecretString;

use crate::callback::CallbackServer;
use crate::error::{AuthError, Result};
use crate::types::MinecraftProfile;
use crate::{microsoft, minecraft, xbox};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(300);

/// High-level orchestrator for the full Microsoft → Minecraft authentication flow.
///
/// # Example
///
/// ```no_run
/// use emerald_auth::MicrosoftAuth;
///
/// # async fn example() -> emerald_auth::Result<()> {
/// let auth = MicrosoftAuth::new("your-azure-client-id");
/// let profile = auth.authenticate().await?;
/// println!("Logged in as {} ({})", profile.username, profile.uuid);
/// # Ok(())
/// # }
/// ```
pub struct MicrosoftAuth {
    client_id: String,
    http: reqwest::Client,
    timeout: Duration,
    port: Option<u16>,
}

impl MicrosoftAuth {
    /// Create a new authenticator with a Microsoft Azure application client ID.
    ///
    /// Register your app at <https://portal.azure.com> with `http://localhost`
    /// as an allowed redirect URI for "Mobile and desktop applications".
    pub fn new(client_id: impl Into<String>) -> Self {
        Self {
            client_id: client_id.into(),
            http: reqwest::Client::new(),
            timeout: DEFAULT_TIMEOUT,
            port: None,
        }
    }

    /// Use a custom `reqwest::Client` (e.g. with proxy or custom headers).
    pub fn with_http_client(mut self, client: reqwest::Client) -> Self {
        self.http = client;
        self
    }

    /// Set the timeout for waiting for the browser callback (default: 5 minutes).
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Force the callback server to bind to a specific port instead of an ephemeral one.
    pub fn with_port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    /// Run the full authentication flow:
    ///
    /// 1. Start a local callback server
    /// 2. Open the Microsoft login page in the default browser
    /// 3. Wait for the OAuth redirect with the authorization code
    /// 4. Exchange tokens through the full chain: Microsoft → Xbox Live → XSTS → Minecraft
    /// 5. Verify game ownership and fetch the player profile
    pub async fn authenticate(&self) -> Result<MinecraftProfile> {
        // 1. Start callback server.
        let (server, redirect_uri) = match self.port {
            Some(port) => CallbackServer::bind_to(port).await?,
            None => CallbackServer::bind().await?,
        };

        // 2. Generate CSRF state and build auth URL.
        let state = generate_state();
        let auth_url = microsoft::build_auth_url(&self.client_id, &redirect_uri, &state);

        // 3. Open browser.
        log::info!("opening browser for Microsoft login");
        if let Err(e) = open::that(&auth_url) {
            return Err(AuthError::BrowserOpen(format!(
                "{e}. Please visit this URL manually: {auth_url}"
            )));
        }

        // 4. Wait for callback.
        let (code, received_state) = server.wait_for_callback(self.timeout).await?;

        // 5. Verify CSRF state.
        if state != received_state {
            return Err(AuthError::StateMismatch {
                expected: state,
                actual: received_state,
            });
        }

        // 6. Exchange auth code for Microsoft tokens.
        log::info!("exchanging authorization code for Microsoft tokens");
        let ms_tokens = microsoft::exchange_code(&self.http, &self.client_id, &code, &redirect_uri).await?;

        // 7. Complete the rest of the chain.
        self.exchange_chain(&ms_tokens.access_token, ms_tokens.refresh_token)
            .await
    }

    /// Re-authenticate using a previously obtained refresh token, without opening the browser.
    pub async fn refresh(&self, refresh_token: &SecretString) -> Result<MinecraftProfile> {
        log::info!("refreshing Microsoft tokens");
        let ms_tokens = microsoft::refresh_tokens(&self.http, &self.client_id, refresh_token).await?;

        self.exchange_chain(&ms_tokens.access_token, ms_tokens.refresh_token)
            .await
    }

    /// The shared portion of the auth chain: Xbox Live → XSTS → Minecraft → profile.
    async fn exchange_chain(
        &self,
        ms_access_token: &SecretString,
        ms_refresh_token: Option<SecretString>,
    ) -> Result<MinecraftProfile> {
        // Xbox Live
        log::info!("authenticating with Xbox Live");
        let xbox = xbox::authenticate_xbox_live(&self.http, ms_access_token).await?;

        // XSTS
        log::info!("obtaining XSTS token");
        let xsts = xbox::authenticate_xsts(&self.http, &xbox.token).await?;

        // Minecraft
        log::info!("authenticating with Minecraft services");
        let mc = minecraft::authenticate_minecraft(&self.http, &xsts.token, &xsts.user_hash).await?;

        // Ownership check
        log::info!("verifying game ownership");
        let owns = minecraft::check_ownership(&self.http, &mc.access_token).await?;
        if !owns {
            return Err(AuthError::NoGameOwnership);
        }

        // Profile
        log::info!("fetching Minecraft profile");
        let profile_resp = minecraft::fetch_profile(&self.http, &mc.access_token).await?;

        let skin = profile_resp.active_skin();
        let cape = profile_resp.active_cape();

        Ok(MinecraftProfile {
            uuid: profile_resp.id,
            username: profile_resp.name,
            skin,
            cape,
            access_token: mc.access_token,
            refresh_token: ms_refresh_token,
        })
    }
}

fn generate_state() -> String {
    let bytes: [u8; 32] = rand::rng().random();
    // Base64url-encode without padding for URL safety.
    base64url_encode(&bytes)
}

fn base64url_encode(data: &[u8]) -> String {
    let mut s = String::with_capacity(data.len() * 4 / 3 + 4);
    for chunk in data.chunks(3) {
        let mut buf = [0u8; 3];
        buf[..chunk.len()].copy_from_slice(chunk);
        let b = ((buf[0] as u32) << 16) | ((buf[1] as u32) << 8) | (buf[2] as u32);
        let chars = [
            b64char((b >> 18) & 0x3F),
            b64char((b >> 12) & 0x3F),
            if chunk.len() > 1 { b64char((b >> 6) & 0x3F) } else { break },
            if chunk.len() > 2 { b64char(b & 0x3F) } else { break },
        ];
        for &c in &chars {
            s.push(c);
        }
    }
    s
}

fn b64char(val: u32) -> char {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    TABLE[val as usize] as char
}
