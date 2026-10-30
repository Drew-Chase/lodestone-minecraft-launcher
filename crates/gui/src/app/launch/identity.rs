//! Who the game plays as: the name, UUID and token it starts with, for a Microsoft account or
//! an offline one.

use md5::{Digest, Md5};

/// The arguments that identify the player.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    pub name: String,
    /// The player's UUID, without dashes.
    pub uuid: String,
    pub access_token: String,
    /// `msa` for a Microsoft account, `legacy` for an offline one.
    pub user_type: &'static str,
    pub xuid: String,
}

impl Identity {
    /// A Microsoft account: the name, dashless UUID and game token its profile reports.
    pub fn microsoft(name: &str, uuid: &str, access_token: &str) -> Self {
        Self {
            name: name.to_string(),
            uuid: uuid.replace('-', ""),
            access_token: access_token.to_string(),
            user_type: "msa",
            xuid: "0".to_string(),
        }
    }

    /// An offline player, named by hand. Its UUID is the one a vanilla server gives that name,
    /// and its token is `0`, as Prism sends for offline accounts.
    pub fn offline(name: &str) -> Self {
        Self {
            name: name.to_string(),
            uuid: offline_uuid(name),
            access_token: "0".to_string(),
            user_type: "legacy",
            xuid: "0".to_string(),
        }
    }
}

/// The UUID a vanilla server gives an offline player: the MD5 of `OfflinePlayer:<name>` with
/// the version-3 and IETF variant bits set, as Java's `UUID.nameUUIDFromBytes` does, shown
/// without dashes.
pub fn offline_uuid(name: &str) -> String {
    let digest = Md5::digest(format!("OfflinePlayer:{name}"));
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest);
    bytes[6] = (bytes[6] & 0x0f) | 0x30;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    format!("{:032x}", u128::from_be_bytes(bytes))
}

#[cfg(test)]
#[path = "../../../tests/app/launch/identity.rs"]
mod tests;
