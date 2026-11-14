//! Sign-in and account handling for the root: the Microsoft device-code flow,
//! the silent restore of a stored sign-in, offline and demo entry, switching and
//! signing out of the accounts the launcher keeps, and saving the stored
//! credentials off the UI thread.

use iced::Task;
use lodestone_minecraft_authentication::{AuthError, Authentication, Profile, Session};
use tracing::{debug, info, warn};

use crate::app::appearance_cache;
use crate::app::components::account_menu::{Entry as AccountEntry, Message as AccountMenuMessage};
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
                // Keep the refresh token so the next boot restores this sign-in silently. The new account plays by default.
                let save = match &session.refresh_token {
                    Some(refresh_token) => {
                        self.credentials.add_and_make_primary(StoredAccount::Microsoft(StoredMicrosoft {
                            refresh_token: refresh_token.clone(),
                            username: session.profile.name.clone(),
                            uuid: Some(session.profile.id.clone()),
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
                // A new sign-in that fails leaves the account list as it is.
                self.login.update(LoginMessage::SignInFailed(error));
                Task::none()
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
                    return self.leave_to(Page::Settings);
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
            | LoginMessage::OfflinePrompt { .. }) => {
                self.login.update(message);
                Task::none()
            }
        }
    }

    /// Boot: enters the launcher at once with the account that plays by default, so no network call stands between the
    /// start and the first page. With no account stored, the launcher opens its sign-in gate instead: the setup wizard on
    /// first run, else the sign-in page (see `setup::signed_out_page`).
    pub(super) fn restore_at_boot(&mut self) -> Task<LodestoneAppMessage> {
        if self.credentials.primary_account().is_none() {
            let page = self.signed_out_page();
            self.router.replace(page);
            return Task::none();
        }
        Task::batch([self.start_primary(), self.enter_launcher()])
    }

    /// Makes the account that plays by default the signed-in one, without moving to another page. An offline profile plays as
    /// it is. A Microsoft account starts on a stand-in session that is already due for a refresh, and that refresh runs in the
    /// background through the single refresh a launch uses. A launch or a skin change pressed meanwhile waits for it, so the
    /// refresh token is never spent twice. The face shows from the copy kept on disk until the restored profile is read.
    pub(super) fn start_primary(&mut self) -> Task<LodestoneAppMessage> {
        let Some(account) = self.credentials.primary_account().cloned() else {
            return Task::none();
        };
        match account {
            StoredAccount::Offline { username } => {
                info!("Starting as the offline profile {username}");
                self.account = Some(Account::Offline { username });
                self.appearance_task()
            }
            StoredAccount::Microsoft(microsoft) => {
                info!("Starting as {}; the Microsoft sign-in is restored in the background", microsoft.username);
                self.account = Some(Account::Microsoft {
                    session: stand_in_session(&microsoft),
                });
                self.restoring = Some(microsoft.username.clone());
                let cached = cached_appearance(microsoft.uuid.clone());
                let refresh = self.start_session_refresh(microsoft.refresh_token);
                Task::batch([cached, refresh])
            }
        }
    }

    /// The restore at boot failed while the account still plays on its stand-in session. A refused sign-in drops the
    /// account and opens the sign-in page with the reason, as a failed restore always did. A failure to reach the
    /// services keeps the account: its session is still due, so the next launch tries again. `alerted` says whether a
    /// launch that waited has already reported the failure.
    pub(super) fn restore_failed(&mut self, username: String, error: RefreshError, alerted: bool) -> Task<LodestoneAppMessage> {
        if !error.needs_sign_in {
            warn!(
                "Restoring the sign-in of {username} failed, and is tried again before the next launch: {}",
                error.message
            );
            if alerted {
                return Task::none();
            }
            return self
                .alerts
                .show(Alert::warn(lang::t_with("login.restore_unreachable", &[("name", username.as_str())])))
                .discard();
        }
        warn!("The saved sign-in of {username} can no longer be used: {}", error.message);
        let save = if self.credentials.remove_microsoft(&username) {
            self.save_credentials()
        } else {
            Task::none()
        };
        self.account = None;
        self.face = None;
        // Another account is still kept, so the launcher plays with it, and the sign-in page stays closed.
        if self.credentials.primary_account().is_some() {
            let notice = self
                .alerts
                .show(Alert::warn(lang::t_with("login.restore_refused", &[("name", username.as_str())])))
                .discard();
            return Task::batch([save, notice, self.start_primary()]);
        }
        self.login.update(LoginMessage::SignInFailed(error.message));
        let page = self.signed_out_page();
        Task::batch([save, self.navigate(page)])
    }

    /// The page a successful sign-in leads to. In the setup wizard it moves the wizard on to its next step. Otherwise it is the
    /// launcher, or the Settings page when the sign-in added an account.
    fn signed_in_destination(&mut self) -> Task<LodestoneAppMessage> {
        if self.router.current() == Page::Setup {
            self.setup.advance();
            return Task::none();
        }
        if std::mem::take(&mut self.adding_account) {
            self.leave_to(Page::Settings)
        } else {
            self.enter_launcher()
        }
    }

    /// Returns from a sign-in page to `page`, the page the sign-in was started from. The sign-in's own entry leaves the history,
    /// so Back does not return to the sign-in page while an account plays.
    pub(super) fn leave_to(&mut self, page: Page) -> Task<LodestoneAppMessage> {
        self.router.leave(page);
        info!("Returned to {}", page.title());
        self.open_page(page)
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
                let appearance = fetch_appearance(&auth, profile.as_ref()).await?;
                // A Microsoft account keeps its appearance on disk, so the next start shows the face before it signs in.
                if let Some(uuid) = profile.map(|profile| profile.id) {
                    let kept = appearance.clone();
                    let stored = tokio::task::spawn_blocking(move || appearance_cache::store(&uuid, &kept)).await;
                    if let Ok(Err(error)) = stored {
                        warn!("Keeping the appearance on disk failed: {error}");
                    }
                }
                Ok(appearance)
            },
            LodestoneAppMessage::AppearanceReady,
        )
    }

    /// Makes the account at `index` the one that plays. An offline profile takes over at once. A Microsoft account logs in
    /// again with its refresh token first, and takes over once that succeeds. While such a login runs, another switch waits:
    /// a refresh token is spent once, so two logins must never use it.
    pub(super) fn switch_account(&mut self, index: usize) -> Task<LodestoneAppMessage> {
        if self.credentials.primary_index() == Some(index) || self.switching_account {
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
                self.switching_account = true;
                let auth = self.auth.clone();
                // The result names the account by what it is, not by its place in the list, which can move while the login runs.
                let account = StoredAccount::Microsoft(microsoft.clone());
                Task::perform(
                    async move {
                        auth.login_with_refresh_token(&microsoft.refresh_token)
                            .await
                            .map_err(|error| error.to_string())
                    },
                    move |result| LodestoneAppMessage::AccountSwitched(account, result),
                )
            }
        }
    }

    /// The login a switch started finished. The account is found again by what it is, and when it is still kept, its
    /// refresh token is the one the login produced, and it plays. An account signed out meanwhile takes nothing. A failed
    /// switch says so, and the account that played stays.
    pub(super) fn account_switched(&mut self, account: StoredAccount, result: Result<Session, String>) -> Task<LodestoneAppMessage> {
        self.switching_account = false;
        let Some(index) = self.credentials.position(&account) else {
            info!(
                "{} was signed out while it signed in again, so the new session is not kept",
                account.username()
            );
            return Task::none();
        };
        match result {
            Ok(session) => {
                info!("Switched to {}", session.profile.name);
                if let Some(token) = session.refresh_token.clone() {
                    self.credentials.replace_refresh_token(&session.profile.id, &session.profile.name, token);
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
            let page = self.signed_out_page();
            return Task::batch([save, self.navigate(page)]);
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

    /// Handles the account menu on the player's face. A switch or an add shuts the menu, then does what the Accounts
    /// tab does for the same account.
    pub(super) fn account_menu(&mut self, message: AccountMenuMessage) -> Task<LodestoneAppMessage> {
        match message {
            AccountMenuMessage::Toggle => {
                self.account_menu_open = !self.account_menu_open;
                Task::none()
            }
            AccountMenuMessage::Dismiss => {
                self.account_menu_open = false;
                Task::none()
            }
            AccountMenuMessage::Switch(index) => {
                self.account_menu_open = false;
                self.switch_account(index)
            }
            AccountMenuMessage::Add => {
                self.account_menu_open = false;
                self.add_account()
            }
        }
    }

    /// The accounts the account menu lists, in the order the Accounts tab shows them. Only the account that plays has
    /// the face the sidebar loaded.
    pub(super) fn account_menu_entries(&self) -> Vec<AccountEntry> {
        let playing = self.credentials.primary_index();
        self.credentials
            .accounts
            .iter()
            .enumerate()
            .map(|(index, account)| {
                let is_playing = playing == Some(index);
                let microsoft = matches!(account, StoredAccount::Microsoft(_));
                AccountEntry {
                    username: account.username().to_owned(),
                    microsoft,
                    playing: is_playing,
                    face: if is_playing && microsoft { self.face.clone() } else { None },
                }
            })
            .collect()
    }
}

/// Why a sign-in with a saved refresh token failed: the message to show, and whether only the player can resolve it, as
/// opposed to a service that could not be reached.
#[derive(Debug, Clone)]
pub struct RefreshError {
    pub message: String,
    pub needs_sign_in: bool,
}

impl From<AuthError> for RefreshError {
    fn from(error: AuthError) -> Self {
        Self {
            needs_sign_in: error.needs_sign_in(),
            message: error.to_string(),
        }
    }
}

/// The session a saved Microsoft account plays on until its refresh lands. It carries the account's name, id and refresh
/// token, and it expired long ago, so every launch or skin change refreshes it before it uses the token.
fn stand_in_session(microsoft: &StoredMicrosoft) -> Session {
    Session {
        minecraft_token: String::new(),
        expires_at: 0,
        refresh_token: Some(microsoft.refresh_token.clone()),
        profile: Profile {
            id: microsoft.uuid.clone().unwrap_or_default(),
            name: microsoft.username.clone(),
            skin_url: None,
            slim: false,
            capes: Vec::new(),
        },
    }
}

/// Reads the appearance kept on disk for the account `uuid`, off the UI thread.
fn cached_appearance(uuid: Option<String>) -> Task<LodestoneAppMessage> {
    let Some(uuid) = uuid else {
        return Task::none();
    };
    Task::perform(
        async move { tokio::task::spawn_blocking(move || appearance_cache::load(&uuid)).await.ok().flatten() },
        LodestoneAppMessage::CachedAppearance,
    )
}

/// Fetches the skin and the worn cape of `profile`, or the vanilla Steve for an offline account or a profile with no skin.
async fn fetch_appearance(auth: &Authentication, profile: Option<&Profile>) -> Result<Appearance, String> {
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
}
