//! Microsoft → Xbox Live → Minecraft services authentication for Lodestone.
//!
//! ```no_run
//! # async fn demo() -> Result<(), lodestone_minecraft_authentication::AuthError> {
//! use lodestone_minecraft_authentication::Authentication;
//!
//! let auth = Authentication::new("lodestones-azure-app-client-id");
//!
//! // Two-phase device-code login: show the code, then wait for the user.
//! let start = auth.begin_device_code_login().await?;
//! println!("Open {} and enter {}", start.verification_uri, start.user_code);
//! let session = auth.finish_device_code_login(start).await?;
//!
//! // Later, silently: store session.refresh_token and re-login with it.
//! let session = auth.login_with_refresh_token("stored-refresh-token").await?;
//! println!("playing as {} ({})", session.profile.name, session.profile.id);
//! # Ok(())
//! # }
//! ```
//!
//! ## Hard rules
//!
//! - **Standalone leaf crate.** Zero path dependencies on workspace siblings;
//!   only `crates/gui` links to workspace crates.
//! - **Async, tokio-based** — iced `Task`s await it directly.
//! - **Dumb about storage.** It returns [`Session`]s and persists nothing;
//!   token storage is gui's responsibility.
//! - **The Azure app is not ours.** `client_id` is a constructor argument; the
//!   app registration must have device-code flow enabled.
//! - **Wheels are not reinvented**: the Microsoft OAuth leg goes through
//!   `oauth2`, the Xbox Live → XSTS → Minecraft services chain goes through
//!   `minecraft-msa-auth`, and this crate owns only the profile fetch and the
//!   user-presentable error mapping (see the wheel audit in `DESIGN.md`).
//!
//! ## What this crate does
//!
//! - Device-code login in two phases (`begin_device_code_login` /
//!   `finish_device_code_login`)
//! - Silent re-login from a stored refresh token (`login_with_refresh_token`)
//! - Profile resolution (`Profile { id, name }`) with game-ownership detection
//!   (`AuthError::MinecraftNotOwned`)
//! - User-presentable Xbox errors (`NoXboxAccount`, `ChildAccount`, …) instead
//!   of raw `XErr` codes
//!
//! ## What this crate does NOT do
//!
//! - Persist tokens or own the Azure app registration
//! - Launch the game (see `lodestone_launch`)

#![deny(missing_docs)]
#![forbid(unsafe_code)]

mod endpoints;
mod error;
mod http;
mod session;

use std::time::{SystemTime, UNIX_EPOCH};

use minecraft_msa_auth::MinecraftAuthorizationFlow;
use oauth2::{AuthUrl, ClientId, DeviceAuthorizationUrl, Scope, TokenResponse, TokenUrl};
use serde::Deserialize;

pub use error::AuthError;
pub use session::{DeviceCodeStart, Profile, Session};

use crate::http::Oauth2HttpClient;
use crate::session::DeviceCodeState;

/// The authentication handle for one Azure app registration. Cheap to clone;
/// all methods take `&self`.
#[derive(Debug, Clone)]
pub struct Authentication {
    client_id: String,
    http: reqwest::Client,
}

/// The Minecraft profile payload of `/minecraft/profile`.
#[derive(Debug, Deserialize)]
struct ProfileResponse {
    id: String,
    name: String,
}

impl Authentication {
    /// Creates an authentication handle for one Azure app registration.
    ///
    /// The app must have device-code flow enabled and the
    /// `XboxLive.signin`/`offline_access` delegated permissions.
    pub fn new(client_id: impl Into<String>) -> Self {
        Self {
            client_id: client_id.into(),
            http: reqwest::Client::new(),
        }
    }

    /// Creates a handle around a preconfigured client (timeouts, proxy, …).
    pub fn with_client(client_id: impl Into<String>, http: reqwest::Client) -> Self {
        Self {
            client_id: client_id.into(),
            http,
        }
    }

    /// The oauth2 client for the Microsoft consumer tenant, with every needed
    /// endpoint set. The endpoint constants are compile-time invariants. In
    /// oauth2 5.0's endpoint-state generics this lands on `EndpointSet` for
    /// auth/token/device — the states whose device-flow methods return the
    /// request builders directly (no extra `Result` unwrap), and the state in
    /// which `exchange_device_access_token` and `exchange_refresh_token` exist.
    fn oauth_client(
        &self,
    ) -> oauth2::basic::BasicClient<oauth2::EndpointSet, oauth2::EndpointSet, oauth2::EndpointNotSet, oauth2::EndpointNotSet, oauth2::EndpointSet>
    {
        oauth2::basic::BasicClient::new(ClientId::new(self.client_id.clone()))
            .set_auth_uri(AuthUrl::new(endpoints::MSA_AUTHORIZE_URL.to_string()).expect("static auth url is valid"))
            .set_token_uri(TokenUrl::new(endpoints::MSA_TOKEN_URL.to_string()).expect("static token url is valid"))
            .set_device_authorization_url(
                DeviceAuthorizationUrl::new(endpoints::DEVICE_CODE_URL.to_string()).expect("static device-code url is valid"),
            )
    }

    /// Phase one of the device-code login: fetches the code the user must
    /// enter at the returned verification url.
    pub async fn begin_device_code_login(&self) -> Result<DeviceCodeStart, AuthError> {
        let client = self.oauth_client();
        let mut request = client.exchange_device_code();
        for scope in endpoints::SCOPES {
            request = request.add_scope(Scope::new(scope.to_string()));
        }
        let details = request
            .request_async(&Oauth2HttpClient(self.http.clone()))
            .await
            .map_err(|source| AuthError::microsoft(&source))?;

        Ok(DeviceCodeStart {
            user_code: details.user_code().secret().clone(),
            verification_uri: details.verification_uri().to_string(),
            expires_in: details.expires_in().as_secs(),
            state: DeviceCodeState { details },
        })
    }

    /// Phase two of the device-code login: polls Microsoft until the user
    /// completes the sign-in (or the code expires), then resolves the full
    /// session — Microsoft token → Xbox → Minecraft → profile.
    ///
    /// Cancelling the awaited future cancels the poll.
    pub async fn finish_device_code_login(&self, start: DeviceCodeStart) -> Result<Session, AuthError> {
        let state = start.state;
        let client = self.oauth_client();
        let token = client
            .exchange_device_access_token(&state.details)
            .request_async(&Oauth2HttpClient(self.http.clone()), tokio::time::sleep, None)
            .await
            .map_err(AuthError::from_device_token_error)?;

        self.session_from_microsoft_token(
            token.access_token().secret(),
            token.refresh_token().map(|refresh| refresh.secret().clone()),
            token.expires_in().unwrap_or_default(),
        )
        .await
    }

    /// Silent re-login from a stored refresh token: exchanges it for a fresh
    /// Microsoft token, then resolves the full session.
    pub async fn login_with_refresh_token(&self, refresh_token: &str) -> Result<Session, AuthError> {
        let client = self.oauth_client();
        let refresh = oauth2::RefreshToken::new(refresh_token.to_string());
        let mut request = client.exchange_refresh_token(&refresh);
        for scope in endpoints::SCOPES {
            request = request.add_scope(Scope::new(scope.to_string()));
        }
        let token = request
            .request_async(&Oauth2HttpClient(self.http.clone()))
            .await
            .map_err(|source| AuthError::microsoft(&source))?;

        self.session_from_microsoft_token(
            token.access_token().secret(),
            token.refresh_token().map(|refresh| refresh.secret().clone()),
            token.expires_in().unwrap_or_default(),
        )
        .await
    }

    /// Xbox → Minecraft → profile: turns a fresh Microsoft access token into a
    /// playable session.
    async fn session_from_microsoft_token(
        &self,
        microsoft_token: &str,
        refresh_token: Option<String>,
        microsoft_expires_in: std::time::Duration,
    ) -> Result<Session, AuthError> {
        let flow = MinecraftAuthorizationFlow::new(self.http.clone());
        let response = flow
            .exchange_microsoft_token(microsoft_token)
            .await
            .map_err(AuthError::from_minecraft_error)?;

        // The session is valid until the earlier of the two expiries.
        let minecraft_expires_in = u64::from(response.expires_in());
        let expires_at = session_expiry(minecraft_expires_in, microsoft_expires_in.as_secs());

        let minecraft_token: String = response.access_token().clone().into();
        let profile = self.fetch_profile(&minecraft_token).await?;
        Ok(Session {
            minecraft_token,
            expires_at,
            refresh_token,
            profile,
        })
    }

    /// Fetches the Minecraft profile for a Minecraft services bearer token.
    /// A 404 means the account has no Minecraft Java profile.
    async fn fetch_profile(&self, minecraft_token: &str) -> Result<Profile, AuthError> {
        let response = self
            .http
            .get(endpoints::PROFILE_URL)
            .bearer_auth(minecraft_token)
            .send()
            .await
            .map_err(|source| AuthError::Network {
                url: endpoints::PROFILE_URL.to_string(),
                source,
            })?;
        let status = response.status();
        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(AuthError::MinecraftNotOwned);
        }
        if !status.is_success() {
            return Err(AuthError::UnexpectedResponse {
                url: endpoints::PROFILE_URL.to_string(),
                status: status.as_u16(),
            });
        }
        let body = response.text().await.map_err(|source| AuthError::Network {
            url: endpoints::PROFILE_URL.to_string(),
            source,
        })?;
        let profile: ProfileResponse = serde_json::from_str(&body).map_err(|source| AuthError::Json {
            url: endpoints::PROFILE_URL.to_string(),
            source,
        })?;
        Ok(Profile {
            id: profile.id,
            name: profile.name,
        })
    }
}

/// Seconds since the unix epoch. The launcher runs on real machines with real
/// clocks; a pre-1970 clock would have broken every timestamp long before this.
fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is after the unix epoch")
        .as_secs()
}

/// The session expiry: unix seconds now plus the *earlier* of the two token
/// lifetimes (Microsoft's and Minecraft's).
fn session_expiry(minecraft_expires_in: u64, microsoft_expires_in: u64) -> u64 {
    unix_now() + minecraft_expires_in.min(microsoft_expires_in)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_profile_payload_maps_onto_the_profile() {
        // Recorded shape of `/minecraft/profile`.
        let payload = r#"{ "id": "b4beefcafe1234567890abcdef123456", "name": "Steve", "skins": [], "capes": [] }"#;
        let profile: ProfileResponse = serde_json::from_str(payload).expect("recorded payload parses");
        let profile = Profile {
            id: profile.id,
            name: profile.name,
        };
        assert_eq!(profile.id, "b4beefcafe1234567890abcdef123456");
        assert_eq!(profile.name, "Steve");
    }

    #[test]
    fn session_expiry_takes_the_earlier_lifetime() {
        let before = unix_now();
        let expiry = session_expiry(86_400, 3_600);
        assert!(expiry >= before + 3_600, "at least the microsoft lifetime");
        assert!(expiry < before + 86_400, "never longer than the shorter lifetime");
    }
}
