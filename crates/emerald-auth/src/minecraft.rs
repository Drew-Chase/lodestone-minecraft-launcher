use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;

use crate::error::{AuthError, Result};
use crate::types::{Cape, MinecraftToken, Skin, SkinVariant};

const MC_AUTH_URL: &str = "https://api.minecraftservices.com/authentication/login_with_xbox";
const MC_ENTITLEMENTS_URL: &str = "https://api.minecraftservices.com/entitlements/mcstore";
const MC_PROFILE_URL: &str = "https://api.minecraftservices.com/minecraft/profile";

/// Exchange an XSTS token for a Minecraft access token.
pub async fn authenticate_minecraft(
    client: &reqwest::Client,
    xsts_token: &SecretString,
    user_hash: &str,
) -> Result<MinecraftToken> {
    let identity_token = format!("XBL3.0 x={};{}", user_hash, xsts_token.expose_secret());

    let resp = client
        .post(MC_AUTH_URL)
        .json(&serde_json::json!({ "identityToken": identity_token }))
        .send()
        .await?;

    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(AuthError::Minecraft(text));
    }

    let data: serde_json::Value = resp.json().await?;
    let access_token = data["access_token"]
        .as_str()
        .ok_or_else(|| AuthError::Minecraft("missing access_token in response".into()))?;

    let expires_in = data["expires_in"].as_u64().unwrap_or(86400);

    Ok(MinecraftToken {
        access_token: SecretString::from(access_token.to_owned()),
        expires_in,
    })
}

/// Check whether the authenticated account owns Minecraft.
pub async fn check_ownership(client: &reqwest::Client, minecraft_token: &SecretString) -> Result<bool> {
    let resp = client
        .get(MC_ENTITLEMENTS_URL)
        .bearer_auth(minecraft_token.expose_secret())
        .send()
        .await?;

    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(AuthError::Minecraft(format!("entitlement check failed: {text}")));
    }

    let data: serde_json::Value = resp.json().await?;
    let items = data["items"].as_array();
    let owns = items.is_some_and(|arr| !arr.is_empty());
    Ok(owns)
}

/// Fetch the Minecraft profile (username, UUID, skins, capes).
pub async fn fetch_profile(
    client: &reqwest::Client,
    minecraft_token: &SecretString,
) -> Result<ProfileResponse> {
    let resp = client
        .get(MC_PROFILE_URL)
        .bearer_auth(minecraft_token.expose_secret())
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(AuthError::Minecraft(format!("profile fetch failed ({status}): {text}")));
    }

    let profile: ProfileResponse = resp.json().await?;
    Ok(profile)
}

/// Raw profile response from Minecraft Services API.
#[derive(Debug, Deserialize)]
pub struct ProfileResponse {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub skins: Vec<SkinResponse>,
    #[serde(default)]
    pub capes: Vec<CapeResponse>,
}

#[derive(Debug, Deserialize)]
pub struct SkinResponse {
    pub id: String,
    pub url: String,
    #[serde(default = "default_variant")]
    pub variant: String,
}

#[derive(Debug, Deserialize)]
pub struct CapeResponse {
    pub id: String,
    pub url: String,
    #[serde(default)]
    pub alias: String,
}

fn default_variant() -> String {
    "CLASSIC".to_owned()
}

impl ProfileResponse {
    /// Convert the active skin from the API response.
    pub fn active_skin(&self) -> Option<Skin> {
        self.skins.first().map(|s| Skin {
            id: s.id.clone(),
            url: s.url.clone(),
            variant: match s.variant.to_uppercase().as_str() {
                "SLIM" => SkinVariant::Slim,
                _ => SkinVariant::Classic,
            },
        })
    }

    /// Convert the active cape from the API response.
    pub fn active_cape(&self) -> Option<Cape> {
        self.capes.first().map(|c| Cape {
            id: c.id.clone(),
            url: c.url.clone(),
            alias: c.alias.clone(),
        })
    }
}
