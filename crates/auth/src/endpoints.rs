//! Microsoft, Xbox, and Minecraft endpoint constants.

/// Microsoft device-code initiation (consumer accounts).
pub(crate) const DEVICE_CODE_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode";
/// Microsoft OAuth authorization endpoint (consumer accounts).
pub(crate) const MSA_AUTHORIZE_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/authorize";
/// Microsoft OAuth token endpoint (common tenant — refresh tokens issued to
/// consumers resolve there).
pub(crate) const MSA_TOKEN_URL: &str = "https://login.microsoftonline.com/common/oauth2/v2.0/token";

/// The Minecraft profile endpoint — uuid + name for the signed-in account.
pub(crate) const PROFILE_URL: &str = "https://api.minecraftservices.com/minecraft/profile";

/// The scopes a launcher needs: sign in to Xbox Live, and `offline_access` so
/// Microsoft issues a refresh token for silent re-login.
pub(crate) const SCOPES: [&str; 2] = ["XboxLive.signin", "offline_access"];
