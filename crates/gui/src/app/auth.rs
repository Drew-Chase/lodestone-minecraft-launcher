//! Sign-in and account handling for the root: the Microsoft device-code flow,
//! the silent restore of a stored sign-in, offline and demo entry, and saving
//! the stored credentials off the UI thread.

use iced::Task;
use tracing::{debug, info, warn};

use crate::app::pages::Page;
use crate::app::pages::login::Message as LoginMessage;
use crate::app::storage::StoredMicrosoft;

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
                // Persist the refresh token so the next boot restores this
                // sign-in silently.
                let save = match &session.refresh_token {
                    Some(refresh_token) => {
                        self.credentials.microsoft = Some(StoredMicrosoft {
                            refresh_token: refresh_token.clone(),
                            username: session.profile.name.clone(),
                        });
                        self.save_credentials()
                    }
                    None => {
                        warn!("Microsoft issued no refresh token; the sign-in will not persist");
                        Task::none()
                    }
                };
                self.account = Some(Account::Microsoft { session });
                self.login.update(LoginMessage::Reset);
                // Fetch the player's face for the sidebar avatar — the
                // ACTIVE skin's texture, or the vanilla Steve when the
                // profile reports none (default-skin accounts).
                let auth = self.auth.clone();
                let skin_url = self.account.as_ref().and_then(|account| match account {
                    Account::Microsoft { session } => session.profile.skin_url.clone(),
                    Account::Offline { .. } => None,
                });
                let face_task = match skin_url {
                    Some(url) => Task::perform(async move { auth.fetch_skin(&url).await.map_err(|error| error.to_string()) }, |result| {
                        LodestoneAppMessage::FaceReady(result)
                    }),
                    None => Task::perform(
                        async move { auth.fetch_default_steve_face().await.map_err(|error| error.to_string()) },
                        |result| LodestoneAppMessage::FaceReady(result),
                    ),
                };
                self.router.replace(Page::Home);
                Task::batch([save, face_task])
            }
            LoginMessage::SignInFinished(generation, Err(error)) if generation == self.auth_generation => {
                warn!("Sign-in failed: {error}");
                // A failed sign-in also invalidates whatever refresh token
                // was on file — it could not have produced this attempt.
                let save = if self.credentials.microsoft.take().is_some() {
                    self.save_credentials()
                } else {
                    Task::none()
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
                Task::none()
            }
            LoginMessage::OfflineMode => {
                // Open the offline prompt, pre-filled with the stored
                // username when there is one.
                let username = self.credentials.offline_username.clone().unwrap_or_default();
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
                self.credentials.offline_username = Some(username.clone());
                let save = self.save_credentials();
                self.account = Some(Account::Offline { username });
                self.login.update(LoginMessage::Reset);
                // The canonical Steve face — fetched from Mojang's own
                // profile service for the "Steve" account.
                let auth = self.auth.clone();
                let face_task = Task::perform(
                    async move { auth.fetch_default_steve_face().await.map_err(|error| error.to_string()) },
                    |result| LodestoneAppMessage::FaceReady(result),
                );
                self.router.replace(Page::Home);
                Task::batch([save, face_task])
            }
            LoginMessage::DemoMode => {
                info!("Entering offline mode as Demo Player");
                let username = "Demo Player".to_string();
                self.credentials.offline_username = Some(username.clone());
                let save = self.save_credentials();
                self.account = Some(Account::Offline { username });
                self.login.update(LoginMessage::Reset);
                self.router.replace(Page::Home);
                save
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

    /// Boot-time restore: silently re-login a stored Microsoft refresh token
    /// before showing the sign-in card.
    pub(super) fn restore_sign_in(&mut self, username: String, refresh_token: String) -> Task<LodestoneAppMessage> {
        self.auth_generation = self.auth_generation.wrapping_add(1);
        let generation = self.auth_generation;
        info!("Restoring Microsoft sign-in for {username} (generation {generation})");
        self.login.update(LoginMessage::Restoring { username });
        let auth = self.auth.clone();
        Task::perform(
            async move { auth.login_with_refresh_token(&refresh_token).await.map_err(|error| error.to_string()) },
            move |result| LodestoneAppMessage::Login(LoginMessage::SignInFinished(generation, result)),
        )
    }

    /// Saves the current credentials without blocking the UI thread. Writes
    /// never overlap: a save requested while one runs waits for it, and the
    /// newest state is written next.
    fn save_credentials(&mut self) -> Task<LodestoneAppMessage> {
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
