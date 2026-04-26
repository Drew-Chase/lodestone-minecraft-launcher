use secrecy::SecretString;
use serde::{Deserialize, Serialize};

/// The final result of a successful authentication flow.
///
/// Contains the player's Minecraft profile along with the access and refresh
/// tokens needed for API calls and future re-authentication.
#[derive(Debug, Clone)]
pub struct MinecraftProfile {
    /// Minecraft UUID (without dashes).
    pub uuid: String,
    /// In-game username.
    pub username: String,
    /// Active skin, if any.
    pub skin: Option<Skin>,
    /// Active cape, if any.
    pub cape: Option<Cape>,
    /// Minecraft access token for API calls.
    pub access_token: SecretString,
    /// Microsoft refresh token for re-authentication without browser.
    pub refresh_token: Option<SecretString>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skin {
    pub id: String,
    pub url: String,
    pub variant: SkinVariant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum SkinVariant {
    Classic,
    Slim,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cape {
    pub id: String,
    pub url: String,
    pub alias: String,
}

/// Microsoft OAuth tokens from the authorization code exchange.
#[derive(Debug, Clone)]
pub struct MicrosoftTokens {
    pub access_token: SecretString,
    pub refresh_token: Option<SecretString>,
    pub expires_in: u64,
}

/// Xbox Live authentication result.
#[derive(Debug, Clone)]
pub struct XboxLiveToken {
    pub token: SecretString,
    pub user_hash: String,
}

/// XSTS authorization result.
#[derive(Debug, Clone)]
pub struct XstsToken {
    pub token: SecretString,
    pub user_hash: String,
}

/// Minecraft Services access token.
#[derive(Debug, Clone)]
pub struct MinecraftToken {
    pub access_token: SecretString,
    pub expires_in: u64,
}
