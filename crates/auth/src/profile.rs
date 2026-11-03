//! The wire shape of the Minecraft profile, and how it becomes a [`Profile`].
//!
//! Mojang lists every skin and cape the player has, each marked `ACTIVE` or `INACTIVE`.

use serde::Deserialize;

use crate::session::{Cape, Profile};

/// The state Mojang gives the skin or cape in use.
const ACTIVE: &str = "ACTIVE";

/// A skin entry of the profile.
#[derive(Debug, Deserialize)]
pub(crate) struct SkinEntry {
    pub(crate) state: String,
    pub(crate) url: String,
    /// `CLASSIC` or `SLIM`.
    #[serde(default)]
    pub(crate) variant: String,
}

/// A cape entry of the profile.
#[derive(Debug, Deserialize)]
pub(crate) struct CapeEntry {
    pub(crate) id: String,
    pub(crate) state: String,
    pub(crate) url: String,
    /// The cape's name.
    #[serde(default)]
    pub(crate) alias: String,
}

/// The Minecraft profile payload, as `/minecraft/profile` and the skin and cape changes answer it.
#[derive(Debug, Deserialize)]
pub(crate) struct ProfileResponse {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) skins: Option<Vec<SkinEntry>>,
    pub(crate) capes: Option<Vec<CapeEntry>>,
}

impl ProfileResponse {
    /// The profile the payload describes: the active skin's texture and model, and every cape with the worn one marked.
    pub(crate) fn into_profile(self) -> Profile {
        let active_skin = self.skins.unwrap_or_default().into_iter().find(|skin| skin.state == ACTIVE);
        let skin_url = active_skin.as_ref().map(|skin| skin.url.clone());
        let slim = active_skin.is_some_and(|skin| skin.variant == "SLIM");
        let capes = self
            .capes
            .unwrap_or_default()
            .into_iter()
            .map(|cape| Cape {
                active: cape.state == ACTIVE,
                id: cape.id,
                name: cape.alias,
                url: cape.url,
            })
            .collect();
        Profile {
            id: self.id,
            name: self.name,
            skin_url,
            slim,
            capes,
        }
    }
}
