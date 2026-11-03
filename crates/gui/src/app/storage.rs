//! Credential storage — the launcher's at-rest account data.
//!
//! Owns the on-disk persistence that the auth crate deliberately avoids:
//! every signed-in account, its Microsoft refresh token or offline name,
//! and which account plays by default. The file lives in the platform config
//! directory (`~/.config/lodestone/credentials.json` on Linux), written with
//! owner-only permissions on Unix.
//!
//! Files written before there were several accounts kept one Microsoft
//! sign-in and one offline name. They still load: [`StoredCredentials::load_from`]
//! moves those into the account list, and the next save writes the new shape.
//!
//! **Known limitation**: the refresh tokens are stored in plain text — an
//! OS keyring (libsecret / DPAPI) is the future hardening task. Tokens
//! are never logged.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// A Microsoft sign-in the launcher keeps: enough to log in again silently.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredMicrosoft {
    /// The Microsoft refresh token (`offline_access` scope). A refresh spends it, so the one a refresh produces replaces it.
    pub refresh_token: String,
    /// The profile name of the last successful sign-in.
    pub username: String,
}

/// One account the launcher keeps signed in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StoredAccount {
    /// A Microsoft account, which logs in again with its refresh token.
    Microsoft(StoredMicrosoft),
    /// An offline profile: a name, with no sign-in.
    Offline { username: String },
}

impl StoredAccount {
    /// The name the account plays under.
    pub fn username(&self) -> &str {
        match self {
            StoredAccount::Microsoft(microsoft) => &microsoft.username,
            StoredAccount::Offline { username } => username,
        }
    }

    /// Whether `other` is the same account: the same kind, under the same name.
    fn is(&self, other: &StoredAccount) -> bool {
        match (self, other) {
            (StoredAccount::Microsoft(_), StoredAccount::Microsoft(_)) | (StoredAccount::Offline { .. }, StoredAccount::Offline { .. }) => {
                self.username() == other.username()
            }
            _ => false,
        }
    }
}

/// The launcher's at-rest account data: every account kept signed in, and which one plays by default.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredCredentials {
    /// Every account, in the order they were added.
    #[serde(default)]
    pub accounts: Vec<StoredAccount>,
    /// The index of the account that plays by default. An index past the end means the first account.
    #[serde(default)]
    pub primary: usize,
    /// The Microsoft sign-in of a file from before there were several accounts. [`StoredCredentials::migrate`] moves it.
    #[serde(default, skip_serializing)]
    pub microsoft: Option<StoredMicrosoft>,
    /// The offline name of a file from before there were several accounts. [`StoredCredentials::migrate`] moves it.
    #[serde(default, skip_serializing)]
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

    /// Loads from an explicit path (the entry point for tests). A file from before there were several accounts is migrated.
    pub fn load_from(path: &PathBuf) -> Self {
        let contents = match std::fs::read_to_string(path) {
            Ok(contents) => contents,
            Err(_) => return Self::default(),
        };
        match serde_json::from_str::<Self>(&contents) {
            Ok(mut credentials) => {
                credentials.migrate();
                credentials
            }
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

    /// Moves the single-account fields of an older file into the list: the Microsoft sign-in first, then the offline name.
    /// A name already listed is not added twice.
    pub fn migrate(&mut self) {
        if let Some(microsoft) = self.microsoft.take() {
            self.insert_if_missing(0, StoredAccount::Microsoft(microsoft));
        }
        if let Some(username) = self.offline_username.take() {
            let at = self.accounts.len();
            self.insert_if_missing(at, StoredAccount::Offline { username });
        }
    }

    /// Inserts `account` at `at`, unless the list already holds that account.
    fn insert_if_missing(&mut self, at: usize, account: StoredAccount) {
        if !self.accounts.iter().any(|kept| kept.is(&account)) {
            self.accounts.insert(at.min(self.accounts.len()), account);
        }
    }

    /// The index of the account that plays by default, or none when there are no accounts.
    pub fn primary_index(&self) -> Option<usize> {
        (!self.accounts.is_empty()).then_some(if self.primary < self.accounts.len() { self.primary } else { 0 })
    }

    /// The account that plays by default, when there is one.
    pub fn primary_account(&self) -> Option<&StoredAccount> {
        self.primary_index().map(|index| &self.accounts[index])
    }

    /// The last offline name kept, which the offline prompt starts with.
    pub fn last_offline_name(&self) -> Option<&str> {
        self.accounts.iter().rev().find_map(|account| match account {
            StoredAccount::Offline { username } => Some(username.as_str()),
            StoredAccount::Microsoft(_) => None,
        })
    }

    /// Adds `account`, or replaces the account of the same kind and name, and makes it the one that plays by default.
    /// Returns its index.
    pub fn add_and_make_primary(&mut self, account: StoredAccount) -> usize {
        let index = match self.accounts.iter().position(|kept| kept.is(&account)) {
            Some(index) => {
                self.accounts[index] = account;
                index
            }
            None => {
                self.accounts.push(account);
                self.accounts.len() - 1
            }
        };
        self.primary = index;
        index
    }

    /// Makes the account at `index` the one that plays by default. Returns false when there is no such account.
    pub fn make_primary(&mut self, index: usize) -> bool {
        let exists = index < self.accounts.len();
        if exists {
            self.primary = index;
        }
        exists
    }

    /// Removes the account at `index`, and keeps the default on an account that remains. Returns the removed account.
    pub fn remove(&mut self, index: usize) -> Option<StoredAccount> {
        if index >= self.accounts.len() {
            return None;
        }
        let removed = self.accounts.remove(index);
        if index < self.primary {
            self.primary -= 1;
        }
        if self.primary >= self.accounts.len() {
            self.primary = 0;
        }
        Some(removed)
    }

    /// Replaces the refresh token of the Microsoft account named `username`. Returns false when no such account is kept.
    pub fn replace_refresh_token(&mut self, username: &str, token: String) -> bool {
        for account in &mut self.accounts {
            if let StoredAccount::Microsoft(microsoft) = account
                && microsoft.username == username
            {
                microsoft.refresh_token = token;
                return true;
            }
        }
        false
    }

    /// Removes the Microsoft account named `username`, when it is kept. Returns whether one was removed.
    pub fn remove_microsoft(&mut self, username: &str) -> bool {
        let found = self
            .accounts
            .iter()
            .position(|account| matches!(account, StoredAccount::Microsoft(microsoft) if microsoft.username == username));
        found.and_then(|index| self.remove(index)).is_some()
    }
}

/// `<config_dir>/lodestone/credentials.json`.
fn credentials_path() -> PathBuf {
    let mut path = crate::app::paths::config_dir().unwrap_or_else(|| PathBuf::from("."));
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

/// Serialises credential writes so they can run off the UI thread without
/// landing out of order. At most one write is in flight. A save requested
/// meanwhile only marks the state dirty, and the newest snapshot is written
/// once the current write completes, so the file always ends on the latest
/// state.
#[derive(Debug, Default)]
pub struct CredentialWriter {
    in_flight: bool,
    dirty: bool,
}

impl CredentialWriter {
    /// Asks for a save. Returns `true` when the caller should start a write
    /// now, or `false` when one is running and this save will follow it.
    pub fn request(&mut self) -> bool {
        if self.in_flight {
            self.dirty = true;
            false
        } else {
            self.in_flight = true;
            true
        }
    }

    /// Records a finished write. Returns `true` when a save arrived during
    /// it, so the caller should write the newest state now.
    pub fn complete(&mut self) -> bool {
        if std::mem::take(&mut self.dirty) {
            true
        } else {
            self.in_flight = false;
            false
        }
    }
}

#[cfg(test)]
#[path = "../../tests/app/storage.rs"]
mod tests;
