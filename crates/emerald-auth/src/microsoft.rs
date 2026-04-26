use secrecy::{ExposeSecret, SecretString};

use crate::error::{AuthError, Result};
use crate::types::MicrosoftTokens;

const AUTH_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/authorize";
const TOKEN_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";
const SCOPE: &str = "XboxLive.signin offline_access";

/// Build the Microsoft OAuth2 authorization URL that the user should visit.
pub fn build_auth_url(client_id: &str, redirect_uri: &str, state: &str) -> String {
    format!(
        "{AUTH_URL}?client_id={client_id}&response_type=code&redirect_uri={redirect_uri}&scope={scope}&state={state}&prompt=select_account",
        redirect_uri = urlencoded(redirect_uri),
        scope = urlencoded(SCOPE),
    )
}

/// Exchange an authorization code for Microsoft access and refresh tokens.
pub async fn exchange_code(
    client: &reqwest::Client,
    client_id: &str,
    auth_code: &str,
    redirect_uri: &str,
) -> Result<MicrosoftTokens> {
    let params = [
        ("client_id", client_id),
        ("code", auth_code),
        ("grant_type", "authorization_code"),
        ("redirect_uri", redirect_uri),
        ("scope", SCOPE),
    ];

    let resp = client.post(TOKEN_URL).form(&params).send().await?;
    parse_token_response(resp).await
}

/// Refresh Microsoft tokens using an existing refresh token.
pub async fn refresh_tokens(
    client: &reqwest::Client,
    client_id: &str,
    refresh_token: &SecretString,
) -> Result<MicrosoftTokens> {
    let params = [
        ("client_id", client_id.to_owned()),
        ("refresh_token", refresh_token.expose_secret().to_owned()),
        ("grant_type", "refresh_token".to_owned()),
        ("scope", SCOPE.to_owned()),
    ];

    let resp = client.post(TOKEN_URL).form(&params).send().await?;
    parse_token_response(resp).await
}

async fn parse_token_response(resp: reqwest::Response) -> Result<MicrosoftTokens> {
    let body: serde_json::Value = resp.json().await?;

    if let Some(error) = body.get("error") {
        return Err(AuthError::OAuth {
            error: error.as_str().unwrap_or("unknown").to_owned(),
            description: body
                .get("error_description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_owned(),
        });
    }

    let access_token = body["access_token"]
        .as_str()
        .ok_or_else(|| AuthError::MissingParam("access_token".into()))?;

    let refresh_token = body["refresh_token"].as_str().map(|s| SecretString::from(s.to_owned()));

    let expires_in = body["expires_in"].as_u64().unwrap_or(3600);

    Ok(MicrosoftTokens {
        access_token: SecretString::from(access_token.to_owned()),
        refresh_token,
        expires_in,
    })
}

fn urlencoded(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}
