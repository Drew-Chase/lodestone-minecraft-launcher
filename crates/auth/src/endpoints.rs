//! Microsoft, Xbox, and Minecraft endpoint constants.

/// Microsoft device-code initiation (consumer accounts).
pub(crate) const DEVICE_CODE_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode";
/// Microsoft OAuth authorization endpoint (consumer accounts).
pub(crate) const MSA_AUTHORIZE_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/authorize";
/// Microsoft OAuth token endpoint (consumers tenant — this app registration
/// is MSA-only, so `/common` rejects its requests with AADSTS9002331).
pub(crate) const MSA_TOKEN_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";

/// The Minecraft profile endpoint — uuid + name for the signed-in account.
pub(crate) const PROFILE_URL: &str = "https://api.minecraftservices.com/minecraft/profile";

/// Instance discovery — a lightweight, tenant-agnostic endpoint that also
/// reports the region scope Microsoft assigns to the requesting address.
/// Used as the auth-services reachability probe for the login status bar.
pub(crate) const INSTANCE_DISCOVERY_URL: &str = "https://login.microsoftonline.com/common/discovery/instance";

/// Mojang's name-lookup API — resolves a player name to their uuid.
/// Used for the canonical "Steve" account (offline/demo faces).
pub(crate) const MOJANG_NAME_LOOKUP_URL: &str = "https://api.mojang.com/users/profiles/minecraft/Steve";

/// Mojang's session server — serves a profile's textures by uuid.
pub(crate) const SESSIONSERVER_URL: &str = "https://sessionserver.mojang.com/session/minecraft/profile";

/// How long the reachability probe may take before it gives up — the
/// status bar would rather report OFFLINE than wait.
pub(crate) const PROBE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// The scopes a launcher needs: sign in to Xbox Live, and `offline_access` so
/// Microsoft issues a refresh token for silent re-login.
pub(crate) const SCOPES: [&str; 2] = ["XboxLive.signin", "offline_access"];
