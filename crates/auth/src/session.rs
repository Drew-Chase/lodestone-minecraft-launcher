//! Session and device-code types returned by the authentication flow.

/// A signed-in Minecraft session: everything the launcher and
/// `lodestone_launch` need to start the game.
///
/// Sessions are returned, never stored — gui persists `refresh_token` in its
/// own storage for silent re-login via
/// [`Authentication::login_with_refresh_token`](crate::Authentication::login_with_refresh_token).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    /// The Minecraft services bearer token (`api.minecraftservices.com`), also
    /// the game's `--accessToken`.
    pub minecraft_token: String,
    /// When the token expires, in unix seconds. Gui can refresh ahead of this
    /// with the refresh token.
    pub expires_at: u64,
    /// The Microsoft refresh token for silent re-login, when Microsoft issued
    /// one (the `offline_access` scope requests it).
    pub refresh_token: Option<String>,
    /// The signed-in player's profile.
    pub profile: Profile,
}

/// The signed-in player's Minecraft profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    /// The player's Minecraft uuid, dash-less as the API returns it
    /// (e.g. `"b4ccccad..."`). The launcher's `--uuid` argument.
    pub id: String,
    /// The player's current name (the launcher's `--username` argument).
    pub name: String,
    /// The texture URL of the player's ACTIVE skin (the full 64×64 PNG on
    /// Mojang's texture CDN), when the profile reports one.
    pub skin_url: Option<String>,
    /// Whether the ACTIVE skin uses the slim (3-pixel arm) model.
    pub slim: bool,
    /// Every cape the player owns, with the one being worn marked `active`.
    pub capes: Vec<Cape>,
}

/// A cape the player owns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cape {
    /// The cape's id, exactly as the profile returned it. It is sent back unchanged to wear the cape.
    pub id: String,
    /// The cape's name, such as `"Migrator"`.
    pub name: String,
    /// The texture URL of the cape PNG (64×32 on Mojang's texture CDN).
    pub url: String,
    /// Whether the player is wearing this cape.
    pub active: bool,
}

/// The first phase of a device-code login: what the user must see and confirm.
///
/// Pass this back to
/// [`Authentication::finish_device_code_login`](crate::Authentication::finish_device_code_login)
/// once the user has completed the sign-in; that call polls until then (or
/// until the code expires — [`crate::AuthError::DeviceCodeExpired`]).
#[derive(Debug, Clone)]
pub struct DeviceCodeStart {
    /// The short code the user enters, e.g. `"ABCD-EFGH"`.
    pub user_code: String,
    /// The url the user opens, `https://www.microsoft.com/link`.
    pub verification_uri: String,
    /// How long the code stays valid, in seconds (Microsoft-issued).
    pub expires_in: u64,

    pub(crate) state: DeviceCodeState,
}

/// The oauth2 state needed to complete the poll in phase two: the device
/// authorization response itself. The oauth2 client is rebuilt on demand from
/// `Authentication`'s client id.
#[derive(Debug, Clone)]
pub(crate) struct DeviceCodeState {
    pub(crate) details: oauth2::StandardDeviceAuthorizationResponse,
}
