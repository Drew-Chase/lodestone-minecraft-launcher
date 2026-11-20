//! Replacing a mod's jar with another release: the update, and the switch to any release that fits the instance.

use super::*;

impl LodestoneLauncher {
    /// Installs the release `version_id` in place of the mod `file_name`, of the project its record names.
    pub(super) fn replace_mod(&mut self, file_name: String, version_id: String) -> Task<LodestoneAppMessage> {
        let Some(record) = self.instance_page.mods().row(&file_name).and_then(|row| row.source.clone()) else {
            return Task::none();
        };
        let Some(source) = Source::from_key(&record.platform) else {
            return Task::none();
        };
        self.replace_with(file_name, source, record.project_id, record.name, version_id)
    }

    /// Installs the release `version_id` of the project `project_id` in place of the mod `file_name`. Waits while a game runs on
    /// the instance, since it may have the jar open. The answer comes back as [`Message::Replaced`].
    pub(super) fn replace_with(
        &mut self,
        file_name: String,
        source: Source,
        project_id: String,
        name: String,
        version_id: String,
    ) -> Task<LodestoneAppMessage> {
        if self.shown_locked() {
            return Task::none();
        }
        let Some(context) = self.mods_context() else {
            return Task::none();
        };
        if self.games.is_active(&context.instance.id) {
            return self.alerts.show(Alert::warn(lang::t("instance.mods.running"))).discard();
        }
        let Some(db) = self.db.clone() else {
            return Task::none();
        };
        let request = AddRequest {
            project: Project {
                source,
                content: ContentType::Mod,
                project_id,
                name,
                web_url: None,
                icon_url: None,
                game_versions: Vec::new(),
                version_id: Some(version_id),
            },
            instance: context.instance.clone(),
        };
        let generation = self.instance_page.generation();
        self.instance_page.mods_mut().set_busy(&file_name, true);
        let keep_disabled = file_name.ends_with(".disabled");
        let answered = file_name.clone();
        let mods = context.mods_dir();
        let replace = sipper(move |mut ask| async move { replace_jar(db, request, mods, file_name, keep_disabled, &mut ask).await });
        Task::sip(replace, LodestoneAppMessage::DependencyConflict, move |result| {
            LodestoneAppMessage::ModActions(Message::Replaced {
                generation,
                file_name: answered.clone(),
                result,
            })
        })
    }

    /// Installs the release `version_id` of the project `project_id` as a new mod of the shown instance, for a mod the instance no
    /// longer has. The answer comes back as [`Message::Installed`].
    pub(super) fn install_version(&mut self, source: Source, project_id: String, name: String, version_id: String) -> Task<LodestoneAppMessage> {
        let Some(context) = self.mods_context() else {
            return Task::none();
        };
        let Some(db) = self.db.clone() else {
            return Task::none();
        };
        let request = AddRequest {
            project: Project {
                source,
                content: ContentType::Mod,
                project_id,
                name: name.clone(),
                web_url: None,
                icon_url: None,
                game_versions: Vec::new(),
                version_id: Some(version_id),
            },
            instance: context.instance.clone(),
        };
        let generation = self.instance_page.generation();
        let mods = context.mods_dir();
        let install = sipper(move |mut ask| async move { install_jar(db, request, mods, &mut ask).await });
        Task::sip(install, LodestoneAppMessage::DependencyConflict, move |result| {
            LodestoneAppMessage::ModActions(Message::Installed {
                generation,
                name: name.clone(),
                result,
            })
        })
    }

    /// A release was installed for a mod the instance no longer had. Its records are applied, the folder is read again, and the
    /// next step of a queue starts. A failure is kept for the queue's summary, or shown when no queue is running.
    pub(super) fn mod_installed(&mut self, name: String, result: Result<Vec<ModSource>, String>) -> Task<LodestoneAppMessage> {
        match result {
            Ok(records) => {
                let mods = self.instance_page.mods_mut();
                for record in &records {
                    mods.apply_source(record.clone());
                }
                self.discover.note_sources(&records);
                let rescan = self.rescan_mods();
                Task::batch([rescan, self.load_instances(), self.next_mod_update()])
            }
            Err(error) => {
                warn!("installing {name} failed: {error}");
                if self.keep_install_failure(&name, &error) {
                    return self.next_mod_update();
                }
                self.alerts
                    .show(Alert::error(lang::t_with("instance.mods.replace.failed", &[("error", error.as_str())])))
                    .discard()
            }
        }
    }

    /// A jar was replaced, or could not be. While Update all runs, the next mod it queued starts once this one has been
    /// applied, and a failure is kept for the summary that ends the queue.
    pub(super) fn mod_replaced(&mut self, file_name: String, result: Result<Vec<ModSource>, String>) -> Task<LodestoneAppMessage> {
        if let Err(error) = &result
            && self.keep_update_failure(&file_name, error)
        {
            warn!("updating the mod {file_name} failed: {error}");
            self.instance_page.mods_mut().set_busy(&file_name, false);
            return self.next_mod_update();
        }
        let applied = self.apply_replaced(file_name, result);
        Task::batch([applied, self.next_mod_update()])
    }

    /// Applies a replaced jar. Its record moves to the new jar, the folder is read again, and the new jar is checked for a
    /// newer release.
    fn apply_replaced(&mut self, file_name: String, result: Result<Vec<ModSource>, String>) -> Task<LodestoneAppMessage> {
        self.instance_page.mods_mut().set_busy(&file_name, false);
        match result {
            Ok(records) => {
                let Some(context) = self.mods_context() else {
                    return Task::none();
                };
                let mods = self.instance_page.mods_mut();
                mods.forget_source(&file_name);
                for record in &records {
                    mods.apply_source(record.clone());
                }
                // The Discover page's cache is what the Versions tab marks as installed, so it takes the new release too.
                self.discover.note_sources(&records);
                let forget = self.forget_checked(&context.instance.id, &file_name);
                let mut tasks = vec![forget, self.rescan_mods(), self.load_instances()];
                if let (Some(own), Some(db)) = (records.first().cloned(), self.db.clone()) {
                    let generation = self.instance_page.generation();
                    let check = check_mods(db, context.check(), Vec::new(), Some(vec![own]));
                    let instance = context.instance.id.clone();
                    tasks.push(Task::perform(check, move |result| {
                        LodestoneAppMessage::ModActions(Message::Checked {
                            generation,
                            instance,
                            full: false,
                            result,
                        })
                    }));
                }
                Task::batch(tasks)
            }
            Err(error) => {
                warn!("replacing a mod failed: {error}");
                self.alerts
                    .show(Alert::error(lang::t_with("instance.mods.replace.failed", &[("error", error.as_str())])))
                    .discard()
            }
        }
    }

    /// Opens the switch dialog for a mod, and lists the releases of its project that fit the instance.
    pub(super) fn open_switch(&mut self, file_name: String) -> Task<LodestoneAppMessage> {
        if self.shown_locked() {
            return Task::none();
        }
        let Some(context) = self.mods_context() else {
            return Task::none();
        };
        let Some(row) = self.instance_page.mods().row(&file_name) else {
            return Task::none();
        };
        let Some(record) = row.source.clone() else {
            return Task::none();
        };
        let name = row.file.name.clone();
        let Some(source) = Source::from_key(&record.platform) else {
            return Task::none();
        };
        let generation = self.instance_page.generation();
        self.instance_page.mods_mut().open_dialog(Dialog::Switch(Switch {
            file_name: file_name.clone(),
            name,
            releases: Load::Loading,
            installed: Some(record.version_id.clone()),
        }));
        let loader = context.instance.loader.to_db().to_string();
        let minecraft = context.instance.mc_version.clone();
        let project_id = record.project_id.clone();
        let answered = file_name.clone();
        Task::perform(
            async move { releases_that_fit(source, &project_id, &loader, &minecraft).await },
            move |result| {
                LodestoneAppMessage::ModActions(Message::Releases {
                    generation,
                    file_name: answered.clone(),
                    result,
                })
            },
        )
    }

    /// The releases a switch can install arrived. They fill the dialog that asked, if it is still open for the same mod.
    pub(super) fn mod_releases(&mut self, file_name: String, result: Result<Vec<ContentVersion>, String>) {
        if let Some(Dialog::Switch(switch)) = self.instance_page.mods_mut().dialog_mut()
            && switch.file_name == file_name
        {
            switch.releases = match result {
                Ok(releases) => Load::Loaded(releases),
                Err(error) => Load::Failed(error),
            };
        }
    }

    /// The release a switch dialog picked is installed in place of the mod.
    pub(super) fn use_release(&mut self, index: usize) -> Task<LodestoneAppMessage> {
        let Some(Dialog::Switch(switch)) = self.instance_page.mods().dialog() else {
            return Task::none();
        };
        let Load::Loaded(releases) = &switch.releases else {
            return Task::none();
        };
        let Some(release) = releases.get(index).cloned() else {
            return Task::none();
        };
        let file_name = switch.file_name.clone();
        self.instance_page.mods_mut().close_dialog();
        self.replace_mod(file_name, release.id)
    }
}

/// Writes the release `request` names as a new jar, then moves the record of `old_file` to it and trashes the old jar.
/// The new jar is written first, so a failure leaves the old mod in place. A disabled jar stays disabled.
pub(crate) async fn replace_jar(
    db: Database,
    request: AddRequest,
    mods: PathBuf,
    old_file: String,
    keep_disabled: bool,
    ask: &mut ProgressSender<VersionConflict>,
) -> Result<Vec<ModSource>, String> {
    let mut records = add_to_instance(request, mods.clone(), Some(db.clone()), ask).await?;
    let Some(own) = records.first_mut() else {
        return Err(lang::t("discover.mods.no_release").to_string());
    };
    if keep_disabled && !own.file_name.ends_with(".disabled") {
        let renamed = format!("{}.disabled", own.file_name);
        let (from, to) = (mods.join(&own.file_name), mods.join(&renamed));
        tokio::task::spawn_blocking(move || std::fs::rename(&from, &to))
            .await
            .map_err(|error| error.to_string())?
            .map_err(|error| error.to_string())?;
        own.file_name = renamed;
    }
    let own = own.clone();
    db.replace_mod_source(&old_file, &own).await?;
    db.record_mod_sources(&records[1..]).await?;
    if own.file_name != old_file {
        let old = mods.join(&old_file);
        if old.is_file() {
            tokio::task::spawn_blocking(move || files::move_to_trash(&old))
                .await
                .map_err(|error| error.to_string())??;
        }
    }
    Ok(records)
}

/// Writes the release `request` names as a new jar in `mods`, and records every mod it writes against the instance. Unlike
/// [`replace_jar`] nothing is replaced, so no record moves and no jar is trashed.
pub(crate) async fn install_jar(
    db: Database,
    request: AddRequest,
    mods: PathBuf,
    ask: &mut ProgressSender<VersionConflict>,
) -> Result<Vec<ModSource>, String> {
    let records = add_to_instance(request, mods, Some(db.clone()), ask).await?;
    db.record_mod_sources(&records).await?;
    Ok(records)
}

/// The releases of a project that run on the loader and the Minecraft version, newest first.
async fn releases_that_fit(source: Source, project_id: &str, loader: &str, minecraft: &str) -> Result<Vec<ContentVersion>, String> {
    let releases = versions_of(source, ContentType::Mod, project_id).await?;
    Ok(releases.into_iter().filter(|release| fits(release, loader, minecraft)).collect())
}
