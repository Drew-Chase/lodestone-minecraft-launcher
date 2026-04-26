//! # emerald-auth
//!
//! Standalone Microsoft OAuth2 authentication for Minecraft launchers.
//!
//! Handles the full authentication chain:
//! **Microsoft OAuth → Xbox Live → XSTS → Minecraft access token + profile.**
//!
//! ## Quick start
//!
//! ```no_run
//! use emerald_auth::MicrosoftAuth;
//!
//! # async fn example() -> emerald_auth::Result<()> {
//! let auth = MicrosoftAuth::new("your-azure-client-id");
//! let profile = auth.authenticate().await?;
//! println!("Logged in as {} ({})", profile.username, profile.uuid);
//! # Ok(())
//! # }
//! ```
//!
//! ## Token refresh
//!
//! ```no_run
//! use emerald_auth::MicrosoftAuth;
//! use secrecy::SecretString;
//!
//! # async fn example(stored_refresh_token: SecretString) -> emerald_auth::Result<()> {
//! let auth = MicrosoftAuth::new("your-azure-client-id");
//! let profile = auth.refresh(&stored_refresh_token).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Lower-level API
//!
//! Each step of the authentication chain is exposed as a public function
//! in the [`microsoft`], [`xbox`], and [`minecraft`] modules for callers
//! who need fine-grained control.

pub mod callback;
pub mod client;
pub mod error;
pub mod microsoft;
pub mod minecraft;
pub mod types;
pub mod xbox;

pub use client::MicrosoftAuth;
pub use error::{AuthError, Result};
pub use types::{
    Cape, MinecraftProfile, MinecraftToken, MicrosoftTokens, Skin, SkinVariant, XboxLiveToken,
    XstsToken,
};
