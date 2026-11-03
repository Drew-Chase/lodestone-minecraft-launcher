//! Sign-in and account handling for the root: the Microsoft device-code flow,
//! the silent restore of a stored sign-in, offline and demo entry, switching and
//! signing out of the accounts the launcher keeps, and saving the stored
//! credentials off the UI thread.

use iced::Task;
use lodestone_minecraft_authentication::Session;
use tracing::{debug, info, warn};

use crate::app::components::alert::Alert;
use crate::app::lang;
use crate::app::pages::Page;
use crate::app::pages::login::Message as LoginMessage;
use crate::app::pages::skins::Appearance;
use crate::app::storage::{StoredAccount, StoredMicrosoft};

use super::{Account, LodestoneAppMessage, LodestoneLauncher};

impl LodestoneLauncher {
    /// Handles every login-page message. Messages with app-level effects
    /// (network, persistence, navigation) are handled here; the rest are
    /// display state, forwarded to the page as-is.
    pub(super) fn update_login(&mut self, message: LoginMessage) -> Task<LodestoneAppMessage> {
        match message {
            LoginMessage::Navigate(page) => self.navigate(page),
            LoginMessage::SignInMicrosoft => {
                // Bump the generation so results from any abandoned poll
                // are dropped when they eventually arrive.
                self.auth_generation = self.auth_generation.wrapping_add(1);
                let generation = self.auth_generation;
                info!("Beginning Microsoft device-code sign-in (generation {generation})");
                self.login.update(LoginMessage::Opening);
                let auth = self.auth.clone();
                Task::perform(
                    async move { auth.begin_device_code_login().await.map_err(|error| error.to_string()) },
                    move |result| LodestoneAppMessage::Login(LoginMessage::DeviceCodeStarted(generation, result)),
                )
            }
            LoginMessage::DeviceCodeStarted(generation, Ok(start)) if generation == self.auth_generation => {
                let user_code = start.user_code.clone();
                let verification_uri = start.verification_uri.clone();
                info!("Device code ready: {user_code} at {verification_uri}");
                self.login.update(LoginMessage::CodeReady { user_code, verification_uri });
                let auth = self.auth.clone();
                Task::perform(
                    async move { auth.finish_device_code_login(start).await.map_err(|error| error.to_string()) },
                    move |result| LodestoneAppMessage::Login(LoginMessage::SignInFinished(generation, result)),
                )
            }
            LoginMessage::DeviceCodeStarted(generation, Err(error)) if generation == self.auth_generation => {
                warn!("Device-code request failed: {error}");
                self.login.update(LoginMessage::SignInFailed(error));
                Task::none()
            }
            LoginMessage::SignInFinished(generation, Ok(session)) if generation == self.auth_generation => {
                info!("Signed in as {}", session.profile.name);
                self.restoring = None;
                // Keep the refresh token so the next boot restores this sign-in silently. The new account plays by default.
                let save = match &session.refresh_token {
                    Some(refresh_token) => {
                        self.credentials.add_and_make_primary(StoredAccount::Microsoft(StoredMicrosoft {
                            refresh_token: refresh_token.clone(),
                            username: session.profile.name.clone(),
                        }));
                        self.save_credentials()
                    }
                    None => {
                        warn!("Microsoft issued no refresh token; the sign-in will not persist");
                        Task::none()
                    }
                };
                self.account = Some(Account::Microsoft { session });
                self.login.update(LoginMessage::Reset);
                // The sidebar's face and the Skins page's viewer come from the account's appearance.
                let appearance = self.appearance_task();
                let destination = self.signed_in_destination();
                Task::batch([save, appearance, destination])
            }
            LoginMessage::SignInFinished(generation, Err(error)) if generation == self.auth_generation => {
                warn!("Sign-in failed: {error}");
                // A restore that fails means its refresh token was spent or revoked, so that account leaves the list.
                // A new sign-in that fails leaves the list as it is.
                let save = match self.restoring.take() {
                    Some(username) if self.credentials.remove_microsoft(&username) => self.save_credentials(),
                    _ => Task::none(),
                };
                self.login.update(LoginMessage::SignInFailed(error));
                save
            }
            // Results from cancelled generations — dropped silently.
            LoginMessage::DeviceCodeStarted(generation, _) | LoginMessage::SignInFinished(generation, _) => {
                debug!("Stale sign-in result from generation {generation} dropped");
                Task::none()
            }
            LoginMessage::CopyCode { code } => {
                info!("Device code copied to clipboard");
                self.login.update(LoginMessage::CodeCopied);
                iced::clipboard::write(code)
            }
            LoginMessage::OpenVerificationLink { url } => {
                match open::that_detached(&url) {
                    Ok(()) => info!("Opened {url} in the default browser"),
                    Err(error) => warn!("Failed to open {url}: {error}"),
                }
                Task::none()
            }
            LoginMessage::CancelSignIn => {
                info!("Sign-in cancelled");
                self.auth_generation = self.auth_generation.wrapping_add(1);
                self.login.update(LoginMessage::Reset);
                if std::mem::take(&mut self.adding_account) {
                    return self.navigate(Page::Settings);
                }
                Task::none()
            }
            LoginMessage::OfflineMode => {
                // Open the offline prompt, pre-filled with the last offline name kept, when there is one.
                let username = self.credentials.last_offline_name().unwrap_or_default().to_owned();
                self.login.update(LoginMessage::OfflinePrompt { username });
                Task::none()
            }
            LoginMessage::OfflineUsernameChanged(username) => {
                self.login.update(LoginMessage::OfflineUsernameChanged(username));
                Task::none()
            }
            LoginMessage::OfflineConfirm { username } => {
                let username = username.trim().to_string();
                if username.is_empty() {
                    return Task::none();
                }
                info!("Entering offline mode as {username}");
                self.credentials
                    .add_and_make_primary(StoredAccount::Offline { username: username.clone() });
                let save = self.save_credentials();
                self.account = Some(Account::Offline { username });
                self.login.update(LoginMessage::Reset);
                // The canonical Steve face — fetched from Mojang's own
                // profile service for the "Steve" account.
                let appearance = self.appearance_task();
                let destination = self.signed_in_destination();
                Task::batch([save, appearance, destination])
            }
            LoginMessage::DemoMode => {
                info!("Entering offline mode as Demo Player");
                let username = "Demo Player".to_string();
                self.credentials
                    .add_and_make_primary(StoredAccount::Offline { username: username.clone() });
                let save = self.save_credentials();
                self.account = Some(Account::Offline { username });
                self.login.update(LoginMessage::Reset);
                let destination = self.signed_in_destination();
                Task::batch([save, destination])
            }
            LoginMessage::LearnMore => {
                info!("'Learn more' requested — opening a browser is not wired yet");
                Task::none()
            }
            // The root applies and saves the language before it reaches here (see `update.rs`).
            LoginMessage::SelectLanguage(_) => Task::none(),
            // Display-state messages: pure page state, forwarded as-is.
            message @ (LoginMessage::Opening
            | LoginMessage::CodeReady { .. }
            | LoginMessage::CodeCopied
            | LoginMessage::SignInFailed(_)
            | LoginMessage::Reset
            | LoginMessage::CodeHover(_)
            | LoginMessage::LinkHover(_)
            | LoginMessage::Restoring { .. }
            | LoginMessage::OfflinePrompt { .. }) => {
                self.login.update(message);
                Task::none()
            }
        }
    }

    /// Boot-time restore: silently re-login the stored Microsoft refresh token of the account that plays by default,
    /// before showing the sign-in card.
    pub(super) fn restore_sign_in(&mut self, username: String, refresh_token: String) -> Task<LodestoneAppMessage> {
        self.auth_generation = self.auth_generation.wrapping_add(1);
        let generation = self.auth_generation;
        info!("Restoring Microsoft sign-in for {username} (generation {generation})");
        self.restoring = Some(username.clone());
        self.login.update(LoginMessage::Restoring { username });
        let auth = self.auth.clone();
        Task::perform(
            async move { auth.login_with_refresh_token(&refresh_token).await.map_err(|error| error.to_string()) },
            move |result| LodestoneAppMessage::Login(LoginMessage::SignInFinished(generation, result)),
        )
    }

    /// The page a successful sign-in leads to: the launcher, or the Settings page when the sign-in added an account.
    fn signed_in_destination(&mut self) -> Task<LodestoneAppMessage> {
        if std::mem::take(&mut self.adding_account) {
            self.navigate(Page::Settings)
        } else {
            self.enter_launcher()
        }
    }

    /// The task that fetches the appearance of the account in use: its skin and the cape it wears, or the vanilla Steve when
    /// the profile has no skin. An offline account has no cape.
    pub(super) fn appearance_task(&self) -> Task<LodestoneAppMessage> {
        let auth = self.auth.clone();
        let profile = match self.account.as_ref() {
            Some(Account::Microsoft { session }) => Some(session.profile.clone()),
            Some(Account::Offline { .. }) | None => None,
        };
        Task::perform(
            async move {
                let Some(profile) = profile else {
                    let skin_png = auth.fetch_default_steve_face().await.map_err(|error| error.to_string())?;
                    return Ok(Appearance {
                        skin_png,
                        cape_png: None,
                        slim: false,
                    });
                };
                let skin_png = match &profile.skin_url {
                    Some(url) => auth.fetch_skin(url).await,
                    None => auth.fetch_default_steve_face().await,
                }
                .map_err(|error| error.to_string())?;
                let cape_png = match profile.capes.iter().find(|cape| cape.active) {
                    Some(cape) => Some(auth.fetch_skin(&cape.url).await.map_err(|error| error.to_string())?),
                    None => None,
                };
                Ok(Appearance {
                    skin_png,
                    cape_png,
                    slim: profile.slim,
                })
            },
            LodestoneAppMessage::AppearanceReady,
        )
    }

    /// Makes the account at `index` the one that plays. An offline profile takes over at once. A Microsoft account logs in
    /// again with its refresh token first, and takes over once that succeeds.
    pub(super) fn switch_account(&mut self, index: usize) -> Task<LodestoneAppMessage> {
        if self.credentials.primary_index() == Some(index) {
            return Task::none();
        }
        let Some(account) = self.credentials.accounts.get(index).cloned() else {
            return Task::none();
        };
        match account {
            StoredAccount::Offline { username } => {
                info!("Switching to the offline profile {username}");
                self.credentials.make_primary(index);
                self.account = Some(Account::Offline { username });
                let save = self.save_credentials();
                Task::batch([save, self.appearance_task()])
            }
            StoredAccount::Microsoft(microsoft) => {
                info!("Switching to {} (signing in again)", microsoft.username);
                let auth = self.auth.clone();
                Task::perform(
                    async move {
                        auth.login_with_refresh_token(&microsoft.refresh_token)
                            .await
                            .map_err(|error| error.to_string())
                    },
                    move |result| LodestoneAppMessage::AccountSwitched(index, result),
                )
            }
        }
    }

    /// The login a switch started finished. Its refresh token is the one the login produced, and the account plays once
    /// it is kept. A failed switch says so, and the account that played stays.
    pub(super) fn account_switched(&mut self, index: usize, result: Result<Session, String>) -> Task<LodestoneAppMessage> {
        match result {
            Ok(session) => {
                info!("Switched to {}", session.profile.name);
                if let Some(token) = session.refresh_token.clone() {
                    self.credentials.replace_refresh_token(&session.profile.name, token);
                }
                self.credentials.make_primary(index);
                let save = self.save_credentials();
                self.account = Some(Account::Microsoft { session });
                Task::batch([save, self.appearance_task()])
            }
            Err(error) => {
                warn!("Switching accounts failed: {error}");
                self.alerts
                    .show(Alert::warn(lang::t_with("settings.accounts.switch_failed", &[("error", error.as_str())])))
                    .discard()
            }
        }
    }

    /// Signs the account at `index` out, so it leaves the list. Removing the account that plays hands the launcher to the
    /// next one, or to the sign-in page when none is left.
    pub(super) fn remove_account(&mut self, index: usize) -> Task<LodestoneAppMessage> {
        let was_playing = self.credentials.primary_index() == Some(index);
        let Some(removed) = self.credentials.remove(index) else {
            return Task::none();
        };
        info!("Signed {} out", removed.username());
        let save = self.save_credentials();
        if self.credentials.accounts.is_empty() {
            self.account = None;
            self.face = None;
            return Task::batch([save, self.navigate(Page::Login)]);
        }
        if !was_playing {
            return save;
        }
        let next = self.credentials.primary_index().unwrap_or(0);
        Task::batch([save, self.switch_account(next)])
    }

    /// Saves the current credentials without blocking the UI thread. Writes
    /// never overlap: a save requested while one runs waits for it, and the
    /// newest state is written next.
    pub(super) fn save_credentials(&mut self) -> Task<LodestoneAppMessage> {
        if self.credential_writer.request() {
            self.write_credentials()
        } else {
            Task::none()
        }
    }

    /// Writes a snapshot of the current credentials on the blocking pool.
    fn write_credentials(&self) -> Task<LodestoneAppMessage> {
        let snapshot = self.credentials.clone();
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || snapshot.store())
                    .await
                    .unwrap_or_else(|error| Err(format!("credential write task failed: {error}")))
            },
            LodestoneAppMessage::CredentialsSaved,
        )
    }

    /// A background write finished. Starts the next one when a save arrived
    /// while it ran.
    pub(super) fn credentials_saved(&mut self, result: Result<(), String>) -> Task<LodestoneAppMessage> {
        if let Err(error) = result {
            warn!("Storing credentials failed: {error}");
        }
        if self.credential_writer.complete() {
            self.write_credentials()
        } else {
            Task::none()
        }
    }
}

impl LodestoneLauncher {
    /// Signs a Microsoft account in for the account list: the sign-in page opens, and the sign-in returns to Settings.
    pub(super) fn add_account(&mut self) -> Task<LodestoneAppMessage> {
        self.adding_account = true;
        self.navigate(Page::Login)
    }
}
