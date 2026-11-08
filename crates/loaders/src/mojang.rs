//! Mojang's public metadata: the list of Minecraft versions, and each version's JSON. Both are fetched through the launcher's
//! network settings, so the proxy and the timeout apply to them as they do to every other request.

use piston_mc::manifest_v2::ManifestV2;
use piston_mc::version_manifest::VersionManifest;
use reqwest::Client;

/// Mojang's list of every Minecraft version.
pub const VERSION_LIST_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

/// Mojang's version list.
pub async fn version_list(client: &Client) -> Result<ManifestV2, reqwest::Error> {
    lodestone_net::get_json(client, VERSION_LIST_URL).await
}

/// The JSON of one version, from the `url` the version list gives it.
pub async fn version_json(client: &Client, url: &str) -> Result<VersionManifest, reqwest::Error> {
    lodestone_net::get_json(client, url).await
}
