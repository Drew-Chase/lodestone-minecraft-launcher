//! The root's side of the Skins page: reading the skin folders and the profile, uploading a skin, wearing or taking off a cape,
//! and the questions and sign-ins that come with them. A change to the profile is a request to Mojang with the account's
//! Minecraft token, so it runs only with a fresh session, and one runs at a time.

use std::path::PathBuf;

use iced::Task;
use lodestone_minecraft_authentication::{Cape, Profile};
use tracing::{info, warn};

use super::{Account, LodestoneAppMessage, LodestoneLauncher};
use crate::app::components::alert::Alert;
use crate::app::face;
use crate::app::lang;
use crate::app::launch::prepare::{self, SessionCheck};
use crate::app::pages::instance::files::move_to_trash;
use crate::app::pages::skins::archive;
use crate::app::pages::skins::capes::{self, CapeArt};
use crate::app::pages::skins::catalog::BUNDLED;
use crate::app::pages::skins::library;
use crate::app::pages::skins::previews::{self, Drawn};
use crate::app::pages::skins::{Account as SkinsAccount, Appearance, Message as SkinsMessage, Scan};

/// A change to the account's appearance that Mojang makes.
pub enum ProfileChange {
    /// Upload `png` as the skin, in the classic or the slim model. `name` is what the question shows.
    Skin { name: String, png: Vec<u8>, slim: bool },
    /// Wear the cape with this id, or take the cape off when `id` is `None`. A cape is saved from the page, without a question.
    Cape { id: Option<String> },
}

/// Reads the saved skins, the archive, and the digests of the bundled skins, off the UI thread.
async fn scan_folders(skins: PathBuf) -> Scan {
    tokio::task::spawn_blocking(move || {
        let _ = std::fs::create_dir_all(&skins);
        Scan {
            saved: library::scan(&skins),
            archived: archive::scan(&archive::dir(&skins)),
            bundled: BUNDLED.iter().map(|skin| library::digest(skin.png)).collect(),
        }
    })
    .await
    .unwrap_or(Scan {
        saved: Vec::new(),
        archived: Vec::new(),
        bundled: Vec::new(),
    })
}

impl LodestoneLauncher {
    /// The account as the Skins page shows it: whether it signed in with Microsoft, and the capes it owns.
    pub(super) fn skins_account(&self) -> SkinsAccount<'_> {
        match &self.account {
            Some(Account::Microsoft { session }) => SkinsAccount {
                microsoft: true,
                capes: &session.profile.capes,
            },
            _ => SkinsAccount {
                microsoft: false,
                capes: &[],
            },
        }
    }

    /// Whether the signed-in account is a Microsoft account, which may change its skin and cape.
    fn signed_in_microsoft(&self) -> bool {
        matches!(self.account, Some(Account::Microsoft { .. }))
    }

    /// Takes the profile a change or a fresh read left, as the signed-in account's profile.
    fn replace_profile(&mut self, profile: Profile) {
        if let Some(Account::Microsoft { session }) = self.account.as_mut() {
            session.profile = profile;
        }
    }

    /// The task that reads the skin folders.
    pub(super) fn scan_task(&self) -> Task<LodestoneAppMessage> {
        Task::perform(scan_folders(self.paths.skins()), LodestoneAppMessage::SkinsLoaded)
    }

    /// The task that fetches the front of each cape the account owns.
    fn capes_task(&self) -> Task<LodestoneAppMessage> {
        let capes: Vec<Cape> = match &self.account {
            Some(Account::Microsoft { session }) => session.profile.capes.clone(),
            _ => Vec::new(),
        };
        if capes.is_empty() {
            return Task::none();
        }
        let auth = self.auth.clone();
        Task::perform(
            async move {
                // A cape whose picture cannot be fetched is left out of the list.
                let mut pictures = Vec::with_capacity(capes.len());
                for cape in capes {
                    if let Ok(png) = auth.fetch_skin(&cape.url).await {
                        pictures.push((cape.id, png));
                    }
                }
                tokio::task::spawn_blocking(move || {
                    pictures
                        .into_iter()
                        .filter_map(|(id, png)| capes::front(&png).map(|face| CapeArt { id, face, png }))
                        .collect::<Vec<CapeArt>>()
                })
                .await
                .unwrap_or_default()
            },
            LodestoneAppMessage::CapesLoaded,
        )
    }

    /// Shows the account's appearance: its skin for the sidebar's face and the Skins page's viewer, and its cape. The skin
    /// previews are drawn again, since they wear the cape.
    pub(super) fn apply_appearance(&mut self, appearance: Appearance) -> Task<LodestoneAppMessage> {
        self.face = face::face_from_skin_png(&appearance.skin_png)
            .and_then(|helm| face::expanded_avatar(&helm))
            .map(|expanded| iced::widget::image::Handle::from_rgba(face::AVATAR_SIZE as u32, face::AVATAR_SIZE as u32, expanded));
        self.skins.set_appearance(appearance);
        self.previews_task()
    }

    /// The task that draws the frame on show for the skins that have none yet. The drawing runs on a background thread, so the
    /// list stays responsive while it runs.
    pub(super) fn previews_task(&mut self) -> Task<LodestoneAppMessage> {
        let Some(job) = self.skins.preview_job() else {
            return Task::none();
        };
        let (generation, frame) = (job.generation, job.frame);
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || previews::draw(job)).await.unwrap_or_else(|_| Drawn {
                    generation,
                    frame,
                    previews: Vec::new(),
                })
            },
            LodestoneAppMessage::PreviewsReady,
        )
    }

    /// The Skins page opened: the folders are read, and a fresh Microsoft session reads the profile again, so changes made in
    /// Minecraft's own launcher show up. The capes are fetched either way.
    pub(super) fn open_skins(&mut self) -> Task<LodestoneAppMessage> {
        let scan = self.scan_task();
        let Some(account) = self.account.clone() else {
            return scan;
        };
        let Account::Microsoft { session } = &account else {
            return scan;
        };
        if !matches!(prepare::session_check(&account, prepare::unix_now()), SessionCheck::Fresh) {
            // Without a fresh session the profile is left as it is, and the capes the account holds are fetched.
            return Task::batch([scan, self.capes_task()]);
        }
        let token = session.minecraft_token.clone();
        let auth = self.auth.clone();
        let read = Task::perform(
            async move { auth.fetch_profile(&token).await.map_err(|error| error.to_string()) },
            LodestoneAppMessage::ProfileRefreshed,
        );
        Task::batch([scan, read])
    }

    /// The fresh profile read when the Skins page opened. The appearance and the capes follow it.
    pub(super) fn profile_refreshed(&mut self, result: Result<Profile, String>) -> Task<LodestoneAppMessage> {
        match result {
            Ok(profile) => {
                self.replace_profile(profile);
                Task::batch([self.appearance_task(), self.capes_task()])
            }
            Err(error) => {
                warn!("Reading the profile for the Skins page failed: {error}");
                Task::none()
            }
        }
    }

    /// Applies a message from the Skins page. The page applies its own changes; the ones that start a request, a question, a
    /// move to the trash, a reveal or a navigation are carried out here.
    pub(super) fn update_skins(&mut self, message: SkinsMessage) -> Task<LodestoneAppMessage> {
        match message {
            SkinsMessage::Use => match self.skins.chosen() {
                Some(chosen) => self.ask_upload(chosen.name, chosen.png, chosen.slim),
                None => Task::none(),
            },
            SkinsMessage::RemoveArchived(index) => self.remove_archived(index),
            SkinsMessage::SaveCape => self.save_cape(),
            SkinsMessage::OpenFolder => super::instances::reveal_folder(self.paths.skins()),
            SkinsMessage::Navigate(page) => self.navigate(page),
            message => {
                self.skins.update(message);
                self.previews_task()
            }
        }
    }

    /// Applies the cape the viewer shows, without a question. It needs a Microsoft account, and no other change under way.
    fn save_cape(&mut self) -> Task<LodestoneAppMessage> {
        if self.skins.busy() {
            return Task::none();
        }
        let Some(Account::Microsoft { session }) = self.account.as_ref() else {
            return Task::none();
        };
        let worn = session.profile.capes.iter().find(|cape| cape.active).map(|cape| cape.id.as_str());
        let Some(id) = self.skins.cape_change(worn) else {
            return Task::none();
        };
        self.start_profile_change(ProfileChange::Cape { id })
    }

    /// Asks the player to confirm a skin upload. The answer returns as `ProfileChangeConfirmed`.
    fn ask_upload(&mut self, name: String, png: Vec<u8>, slim: bool) -> Task<LodestoneAppMessage> {
        if self.skins.busy() {
            return Task::none();
        }
        if !self.signed_in_microsoft() {
            return self.alerts.show(Alert::warn(lang::t("skins.needs_microsoft"))).discard();
        }
        let message = lang::t_with("skins.upload_message", &[("name", name.as_str())]);
        self.pending_change = Some(ProfileChange::Skin { name, png, slim });
        self.alerts
            .show(Alert::warn(message).title(lang::t("skins.upload_title")).yes_no())
            .map(LodestoneAppMessage::ProfileChangeConfirmed)
    }

    /// The answer to the question about a change. A yes starts the change.
    pub(super) fn profile_change_confirmed(&mut self, confirmed: bool) -> Task<LodestoneAppMessage> {
        let Some(change) = self.pending_change.take() else {
            return Task::none();
        };
        if !confirmed {
            return Task::none();
        }
        self.start_profile_change(change)
    }

    /// Starts a change with the session the account holds. A session close to expiry is refreshed first, through the same
    /// single refresh a launch uses, and the change waits for it.
    fn start_profile_change(&mut self, change: ProfileChange) -> Task<LodestoneAppMessage> {
        let Some(account) = self.account.clone() else {
            return Task::none();
        };
        match prepare::session_check(&account, prepare::unix_now()) {
            SessionCheck::Fresh => self.send_profile_change(change),
            SessionCheck::SignInAgain => self.alerts.show(Alert::warn(lang::t("skins.sign_in_again"))).discard(),
            SessionCheck::Refresh(token) => {
                // A refresh already under way takes this change among the ones waiting for it.
                self.waiting_change = Some(change);
                self.start_session_refresh(token)
            }
        }
    }

    /// Carries on with the change that waited for a refresh. A refresh that did not apply, because the signed-in session has
    /// moved on, drops the change.
    pub(super) fn resume_profile_change(&mut self, applied: bool) -> Task<LodestoneAppMessage> {
        let Some(change) = self.waiting_change.take() else {
            return Task::none();
        };
        if applied {
            self.start_profile_change(change)
        } else {
            self.alerts.show(Alert::warn(lang::t("skins.sign_in_again"))).discard()
        }
    }

    /// Sends a change to Mojang with the account's token, and reads the profile the answer leaves. A skin's current skin is
    /// archived first, and an archive that cannot be written stops the upload.
    fn send_profile_change(&mut self, change: ProfileChange) -> Task<LodestoneAppMessage> {
        let Some(Account::Microsoft { session }) = self.account.as_ref() else {
            return Task::none();
        };
        let token = session.minecraft_token.clone();
        let auth = self.auth.clone();
        self.skins.set_busy(true);
        match change {
            ProfileChange::Skin { name, png, slim } => {
                info!("Uploading the skin {name}");
                let current = self.skins.appearance().map(|appearance| (appearance.skin_png.clone(), appearance.slim));
                let archive_dir = archive::dir(&self.paths.skins());
                let known = self.skins.known().to_vec();
                Task::perform(
                    async move {
                        if let Some((current_png, current_slim)) = current {
                            let kept = tokio::task::spawn_blocking(move || archive::keep(&archive_dir, &current_png, current_slim, &known))
                                .await
                                .map_err(|error| error.to_string())?;
                            if let Err(error) = kept {
                                let message = error.to_string();
                                return Err(lang::t_with("skins.archive.failed", &[("error", message.as_str())]));
                            }
                        }
                        auth.upload_skin(&token, slim, &png).await.map_err(|error| error.to_string())
                    },
                    LodestoneAppMessage::ProfileChanged,
                )
            }
            ProfileChange::Cape { id: Some(id) } => Task::perform(
                async move { auth.equip_cape(&token, &id).await.map_err(|error| error.to_string()) },
                LodestoneAppMessage::ProfileChanged,
            ),
            ProfileChange::Cape { id: None } => Task::perform(
                async move { auth.hide_cape(&token).await.map_err(|error| error.to_string()) },
                LodestoneAppMessage::ProfileChanged,
            ),
        }
    }

    /// A change to the profile finished. The profile it leaves replaces the signed-in one, and the appearance and capes
    /// follow it. A failure is shown to the player.
    pub(super) fn profile_changed(&mut self, result: Result<Profile, String>) -> Task<LodestoneAppMessage> {
        self.skins.set_busy(false);
        match result {
            Ok(profile) => {
                info!("The profile changed for {}", profile.name);
                self.replace_profile(profile);
                self.skins.reset_cape_pick();
                Task::batch([self.appearance_task(), self.capes_task()])
            }
            Err(error) => {
                warn!("Changing the skin or cape failed: {error}");
                self.alerts
                    .show(Alert::warn(lang::t_with("skins.change_failed", &[("error", error.as_str())])))
                    .discard()
            }
        }
    }

    /// Moves an archived skin to the trash, where it can be restored. Nothing is deleted outright.
    fn remove_archived(&mut self, index: usize) -> Task<LodestoneAppMessage> {
        let Some(path) = self.skins.archived_path(index) else {
            return Task::none();
        };
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || move_to_trash(&path))
                    .await
                    .unwrap_or_else(|error| Err(error.to_string()))
            },
            LodestoneAppMessage::SkinFileMoved,
        )
    }

    /// A skin file moved to the trash. The folders are read again, so the archive drops it.
    pub(super) fn skin_file_moved(&mut self, result: Result<(), String>) -> Task<LodestoneAppMessage> {
        match result {
            Ok(()) => self.scan_task(),
            Err(error) => {
                warn!("Moving the skin to the trash failed: {error}");
                self.alerts
                    .show(Alert::warn(lang::t_with("skins.remove_failed", &[("error", error.as_str())])))
                    .discard()
            }
        }
    }
}
