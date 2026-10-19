//! Errors surfaced by authenticating with Microsoft / Xbox / Minecraft services.

use minecraft_msa_auth::MinecraftAuthorizationError;
use oauth2::HttpClientError;

/// The oauth2 device-access-token error, whose response carries the user-facing
/// outcomes (`authorization_pending`, `slow_down`, `access_denied`,
/// `expired_token`).
pub(crate) type OAuth2DeviceTokenError =
    oauth2::RequestTokenError<HttpClientError<reqwest::Error>, oauth2::StandardErrorResponse<oauth2::DeviceCodeErrorResponseType>>;

/// The minecraft-msa-auth error for our reqwest-backed client.
pub(crate) type MinecraftAuthError = MinecraftAuthorizationError<reqwest::Error>;

/// Everything that can go wrong while signing in. Messages are written to be
/// shown to a user verbatim: gui surfaces them without extra formatting, and
/// Xbox failures are mapped onto human language instead of raw `XErr` codes.
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    /// A platform endpoint could not be reached.
    #[error("failed to reach {url}: {source}")]
    Network {
        /// The url that was being fetched.
        url: String,
        /// The underlying transport failure.
        #[source]
        source: reqwest::Error,
    },

    /// A platform endpoint answered, but with an unexpected HTTP status.
    #[error("{url} responded with unexpected HTTP status {status}")]
    UnexpectedResponse {
        /// The url that was being fetched.
        url: String,
        /// The HTTP status code that came back.
        status: u16,
    },

    /// A platform response could not be deserialized.
    #[error("failed to parse the response from {url}: {source}")]
    Json {
        /// The url that was being fetched.
        url: String,
        /// The underlying deserialization failure.
        #[source]
        source: serde_json::Error,
    },

    /// The Microsoft leg failed (device-code request, token exchange, or
    /// refresh). The specific user-facing outcomes (expired code, denied
    /// request) have their own variants.
    #[error("microsoft sign-in failed: {message}")]
    Microsoft {
        /// The failure's message, for display.
        message: String,
    },

    /// The device code expired before the sign-in was completed.
    #[error("the sign-in code expired; start the sign-in again")]
    DeviceCodeExpired,

    /// The user denied the sign-in request.
    #[error("the sign-in request was denied")]
    AuthorizationDenied,

    /// The Microsoft account has no Xbox account attached to it.
    #[error("this Microsoft account has no Xbox account; create one at xbox.com to play Minecraft")]
    NoXboxAccount,

    /// Xbox Live is not available for the account's country.
    #[error("Xbox Live is not available in your country/region")]
    XboxLiveUnavailable,

    /// The account's region does not support Xbox Live sign-in.
    #[error("your region does not support Xbox Live sign-in")]
    RegionUnsupported,

    /// The account requires adult verification (South Korea).
    #[error("this account requires adult verification before it can sign in")]
    AdultVerificationRequired,

    /// The account is a minor without a Microsoft family.
    #[error("this account is a minor and must be added to a Microsoft family before it can play")]
    ChildAccount,

    /// Xbox Live rejected the sign-in with an unmapped code.
    #[error("Xbox Live sign-in failed with error code {code}")]
    XboxLiveError {
        /// The raw Xbox `XErr` code, for reporting.
        code: u32,
    },

    /// The Minecraft services token exchange failed.
    #[error("minecraft sign-in failed: {source}")]
    MinecraftAuth {
        /// The underlying failure.
        #[source]
        source: MinecraftAuthError,
    },

    /// The account has no Minecraft Java profile — it does not own the game.
    #[error("this account does not own Minecraft Java Edition")]
    MinecraftNotOwned,
}

impl AuthError {
    /// Logs an oauth2 failure and wraps its display text into
    /// [`AuthError::Microsoft`].
    pub(crate) fn microsoft(source: &impl std::fmt::Display) -> Self {
        tracing::warn!(error = %source, "microsoft sign-in step failed");
        Self::Microsoft { message: source.to_string() }
    }

    /// Maps an oauth2 device-access-token failure onto the user-presentable
    /// variants (`expired`, `denied`); the remaining poll states are internal
    /// and surface as [`AuthError::Microsoft`].
    pub(crate) fn from_device_token_error(source: OAuth2DeviceTokenError) -> Self {
        match source {
            OAuth2DeviceTokenError::ServerResponse(response) => match response.error() {
                oauth2::DeviceCodeErrorResponseType::ExpiredToken => Self::DeviceCodeExpired,
                oauth2::DeviceCodeErrorResponseType::AccessDenied => Self::AuthorizationDenied,
                other => Self::microsoft(&format!("unexpected device sign-in state: {other:?}")),
            },
            other => Self::microsoft(&other),
        }
    }

    /// Maps an Xbox sign-in failure onto the user-presentable variants.
    pub(crate) fn from_minecraft_error(source: MinecraftAuthError) -> Self {
        match source {
            MinecraftAuthError::NoXbox => Self::NoXboxAccount,
            MinecraftAuthError::AddToFamily => Self::ChildAccount,
            MinecraftAuthError::XboxLive { code } => Self::from_xerr(code),
            MinecraftAuthError::Http(source) => Self::MinecraftAuth {
                source: MinecraftAuthorizationError::Http(source),
            },
            MinecraftAuthError::HttpStatus(status) => Self::UnexpectedResponse {
                url: "Xbox Live".to_string(),
                status: status.as_u16(),
            },
            MinecraftAuthError::Json(source) => Self::Json {
                url: "Xbox Live".to_string(),
                source,
            },
            MinecraftAuthError::MissingClaims => Self::UnexpectedResponse {
                url: "Xbox Live".to_string(),
                status: 200,
            },
        }
    }

    /// Maps the remaining Xbox `XErr` codes that `minecraft-msa-auth` does not
    /// interpret (see DESIGN.md §6).
    fn from_xerr(code: u32) -> Self {
        match code {
            2148916234 => Self::XboxLiveUnavailable,
            2148916235 => Self::RegionUnsupported,
            2148916236 | 2148916237 => Self::AdultVerificationRequired,
            other => Self::XboxLiveError { code: other },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The XErr codes the crate maps itself, beyond the two `minecraft-msa-auth`
    /// interprets (`2148916233` no-xbox, `2148916238` add-to-family).
    #[test]
    fn xbox_err_codes_map_onto_user_presentable_variants() {
        assert!(matches!(AuthError::from_xerr(2148916234), AuthError::XboxLiveUnavailable));
        assert!(matches!(AuthError::from_xerr(2148916235), AuthError::RegionUnsupported));
        assert!(matches!(AuthError::from_xerr(2148916236), AuthError::AdultVerificationRequired));
        assert!(matches!(AuthError::from_xerr(2148916237), AuthError::AdultVerificationRequired));
    }

    #[test]
    fn unknown_xerr_codes_carry_their_number() {
        let error = AuthError::from_xerr(999999999);
        assert!(matches!(error, AuthError::XboxLiveError { code: 999999999 }));
        assert_eq!(error.to_string(), "Xbox Live sign-in failed with error code 999999999");
    }

    #[test]
    fn device_token_errors_map_onto_expiry_and_denial() {
        let expired = OAuth2DeviceTokenError::ServerResponse(oauth2::StandardErrorResponse::new(
            oauth2::DeviceCodeErrorResponseType::ExpiredToken,
            None,
            None,
        ));
        assert!(matches!(AuthError::from_device_token_error(expired), AuthError::DeviceCodeExpired));

        let denied = OAuth2DeviceTokenError::ServerResponse(oauth2::StandardErrorResponse::new(
            oauth2::DeviceCodeErrorResponseType::AccessDenied,
            None,
            None,
        ));
        assert!(matches!(AuthError::from_device_token_error(denied), AuthError::AuthorizationDenied));
    }

    #[test]
    fn the_microsoft_variant_carries_a_user_presentable_message() {
        let denied = OAuth2DeviceTokenError::ServerResponse(oauth2::StandardErrorResponse::new(
            oauth2::DeviceCodeErrorResponseType::AccessDenied,
            None,
            None,
        ));
        let error = AuthError::from_device_token_error(denied);
        assert_eq!(error.to_string(), "the sign-in request was denied");

        let error = AuthError::from_xerr(2148916235);
        assert_eq!(error.to_string(), "your region does not support Xbox Live sign-in");
    }
}
