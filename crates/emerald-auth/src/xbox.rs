use secrecy::{ExposeSecret, SecretString};
use serde_json::json;

use crate::error::{AuthError, Result};
use crate::types::{XboxLiveToken, XstsToken};

const XBOX_LIVE_AUTH_URL: &str = "https://user.auth.xboxlive.com/user/authenticate";
const XSTS_AUTH_URL: &str = "https://xsts.auth.xboxlive.com/xsts/authorize";

/// Authenticate with Xbox Live using a Microsoft access token.
pub async fn authenticate_xbox_live(
    client: &reqwest::Client,
    microsoft_token: &SecretString,
) -> Result<XboxLiveToken> {
    let body = json!({
        "Properties": {
            "AuthMethod": "RPS",
            "SiteName": "user.auth.xboxlive.com",
            "RpsTicket": format!("d={}", microsoft_token.expose_secret())
        },
        "RelyingParty": "http://auth.xboxlive.com",
        "TokenType": "JWT"
    });

    let resp = client
        .post(XBOX_LIVE_AUTH_URL)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(AuthError::XboxLive(format!("{status}: {text}")));
    }

    let data: serde_json::Value = resp.json().await?;
    let token = data["Token"]
        .as_str()
        .ok_or_else(|| AuthError::XboxLive("missing Token in response".into()))?;

    let user_hash = data["DisplayClaims"]["xui"][0]["uhs"]
        .as_str()
        .ok_or_else(|| AuthError::XboxLive("missing user hash in response".into()))?;

    Ok(XboxLiveToken {
        token: SecretString::from(token.to_owned()),
        user_hash: user_hash.to_owned(),
    })
}

/// Get an XSTS token using an Xbox Live token.
pub async fn authenticate_xsts(
    client: &reqwest::Client,
    xbox_token: &SecretString,
) -> Result<XstsToken> {
    let body = json!({
        "Properties": {
            "SandboxId": "RETAIL",
            "UserTokens": [xbox_token.expose_secret()]
        },
        "RelyingParty": "rp://api.minecraftservices.com/",
        "TokenType": "JWT"
    });

    let resp = client
        .post(XSTS_AUTH_URL)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let data: serde_json::Value = resp.json().await.unwrap_or_default();
        let xerr = data["XErr"].as_u64().unwrap_or(0);
        let message = xsts_error_message(xerr);
        return Err(AuthError::Xsts { xerr, message });
    }

    let data: serde_json::Value = resp.json().await?;
    let token = data["Token"]
        .as_str()
        .ok_or_else(|| AuthError::Xsts {
            xerr: 0,
            message: "missing Token in response".into(),
        })?;

    let user_hash = data["DisplayClaims"]["xui"][0]["uhs"]
        .as_str()
        .ok_or_else(|| AuthError::Xsts {
            xerr: 0,
            message: "missing user hash in response".into(),
        })?;

    Ok(XstsToken {
        token: SecretString::from(token.to_owned()),
        user_hash: user_hash.to_owned(),
    })
}

fn xsts_error_message(xerr: u64) -> String {
    match xerr {
        2148916233 => "This Microsoft account does not have an Xbox account. Please create one first.".into(),
        2148916235 => "Xbox Live is not available in your country/region.".into(),
        2148916236 | 2148916237 => "This account needs adult verification. Please complete verification on Xbox.com.".into(),
        2148916238 => "This is a child account. A parent must add this account to a Microsoft Family before it can sign in.".into(),
        _ => format!("XSTS authentication failed with error code {xerr}"),
    }
}
