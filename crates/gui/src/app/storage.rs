//! Credential storage — the launcher's at-rest account data.
//!
//! Owns the on-disk persistence that the auth crate deliberately avoids:
//! the Microsoft refresh token and the offline username live in a JSON
//! file under the platform config directory
//! (`~/.config/lodestone/credentials.json` on Linux), written with
//! owner-only permissions on Unix.
//!
//! **Known limitation**: the refresh token is stored in plain text — an
//! OS keyring (libsecret / DPAPI) is the future hardening task. Tokens
//! are never logged.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// The stored Microsoft sign-in: enough to silently re-login.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredMicrosoft {
    /// The Microsoft refresh token (`offline_access` scope).
    pub refresh_token: String,
    /// The profile name of the last successful sign-in.
    pub username: String,
}

/// The launcher's at-rest account data. Both halves are optional — any
/// combination is valid.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredCredentials {
    /// The last successful Microsoft sign-in, when there was one.
    pub microsoft: Option<StoredMicrosoft>,
    /// The last offline username, when there was one.
    pub offline_username: Option<String>,
}

impl StoredCredentials {
    /// Loads the stored credentials. A missing, unreadable, or corrupt
    /// file yields the empty defaults — a broken credential file must
    /// never block the launcher.
    pub fn load() -> Self {
        Self::load_from(&credentials_path())
    }

    /// Persists the credentials, creating the directory on first write.
    pub fn store(&self) -> Result<(), String> {
        Self::store_to(&credentials_path(), self)
    }

    /// Loads from an explicit path (the entry point for tests).
    pub fn load_from(path: &PathBuf) -> Self {
        let contents = match std::fs::read_to_string(path) {
            Ok(contents) => contents,
            Err(_) => return Self::default(),
        };
        match serde_json::from_str(&contents) {
            Ok(credentials) => credentials,
            Err(error) => {
                tracing::warn!("credential file is corrupt, ignoring it: {error}");
                Self::default()
            }
        }
    }

    /// Persists to an explicit path (the entry point for tests).
    pub fn store_to(path: &PathBuf, credentials: &Self) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| format!("creating {}: {error}", parent.display()))?;
        }
        let json = serde_json::to_string_pretty(credentials).map_err(|error| format!("serializing credentials: {error}"))?;
        std::fs::write(path, json).map_err(|error| format!("writing {}: {error}", path.display()))?;
        restrict_permissions(path);
        Ok(())
    }
}

/// `<config_dir>/lodestone/credentials.json`.
fn credentials_path() -> PathBuf {
    let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("lodestone");
    path.push("credentials.json");
    path
}

/// Owner-only file permissions on Unix; a no-op elsewhere (Windows
/// secures the user profile by default).
fn restrict_permissions(path: &PathBuf) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    }
    #[cfg(not(unix))]
    let _ = path;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!("lodestone-storage-test-{tag}-{}.json", std::process::id()))
    }

    #[test]
    fn credentials_round_trip() {
        let path = temp_path("round-trip");
        let credentials = StoredCredentials {
            microsoft: Some(StoredMicrosoft {
                refresh_token: "refresh-token-value".into(),
                username: "Steve".into(),
            }),
            offline_username: Some("Drew".into()),
        };
        StoredCredentials::store_to(&path, &credentials).expect("store succeeds");
        let loaded = StoredCredentials::load_from(&path);
        assert_eq!(loaded, credentials);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn a_missing_file_loads_as_empty() {
        let path = temp_path("missing");
        let loaded = StoredCredentials::load_from(&path);
        assert_eq!(loaded, StoredCredentials::default());
    }

    #[test]
    fn a_corrupt_file_loads_as_empty() {
        let path = temp_path("corrupt");
        std::fs::write(&path, "{ not json").expect("write succeeds");
        let loaded = StoredCredentials::load_from(&path);
        assert_eq!(loaded, StoredCredentials::default());
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn an_empty_store_serializes_without_secrets() {
        let json = serde_json::to_string(&StoredCredentials::default()).expect("serializes");
        assert_eq!(json, "{\"microsoft\":null,\"offline_username\":null}");
    }
}
