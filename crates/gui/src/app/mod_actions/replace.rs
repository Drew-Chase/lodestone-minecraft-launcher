//! Replacing a mod's jar with another release: the update, and the switch to any release that fits the instance.

use super::*;

impl LodestoneLauncher {
    /// Installs `release` in place of the mod `file_name`. Waits while a game runs on the instance, since it may have the
    /// jar open.
    pub(super) fn replace_mod(&mut self, file_name: String, release: ContentVersion) -> Task<LodestoneAppMessage> {
        let Some(context) = self.mods_context() else {
            return Task::none();
        };
        if self.games.is_active(&context.instance.id) {
            return self.alerts.show(Alert::warn(lang::t("instance.mods.running"))).discard();
        }
        let (Some(db), Some(record)) = (
            self.db.clone(),
            self.instance_page.mods().row(&file_name).and_then(|row| row.source.clone()),
        ) else {
            return Task::none();
        };
        let Some(source) = Source::from_key(&record.platform) else {
            return Task::none();
        };
        let request = AddRequest {
            project: Project {
                source,
                content: ContentType::Mod,
                project_id: record.project_id.clone(),
                name: record.name.clone(),
                web_url: None,
                icon_url: None,
                game_version: None,
                version_id: Some(release.id.clone()),
            },
            instance: context.instance.clone(),
        };
        let generation = self.instance_page.generation();
        self.instance_page.mods_mut().set_busy(&file_name, true);
        let keep_disabled = file_name.ends_with(".disabled");
        let answered = file_name.clone();
        Task::perform(replace_jar(db, request, context.mods_dir(), file_name, keep_disabled), move |result| {
            LodestoneAppMessage::ModActions(Message::Replaced {
                generation,
                file_name: answered.clone(),
                result,
            })
        })
    }

    /// A jar was replaced. Its record moves to the new jar, the folder is read again, and the new jar is checked for a
    /// newer release.
    pub(super) fn mod_replaced(&mut self, file_name: String, result: Result<Vec<ModSource>, String>) -> Task<LodestoneAppMessage> {
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
                self.forget_checked(&context.instance.id, &file_name);
                let mut tasks = vec![self.rescan_mods(), self.load_instances()];
                if let (Some(own), Some(db)) = (records.first().cloned(), self.db.clone()) {
                    let generation = self.instance_page.generation();
                    let check = check_mods(db, context.check(), Vec::new(), Some(vec![own]));
                    tasks.push(Task::perform(check, move |result| {
                        LodestoneAppMessage::ModActions(Message::Checked {
                            generation,
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
        self.replace_mod(file_name, release)
    }
}

/// Writes the release `request` names as a new jar, then moves the record of `old_file` to it and trashes the old jar.
/// The new jar is written first, so a failure leaves the old mod in place. A disabled jar stays disabled.
async fn replace_jar(db: Database, request: AddRequest, mods: PathBuf, old_file: String, keep_disabled: bool) -> Result<Vec<ModSource>, String> {
    let mut records = add_to_instance(request, mods.clone()).await?;
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

/// The releases of a project that run on the loader and the Minecraft version, newest first.
async fn releases_that_fit(source: Source, project_id: &str, loader: &str, minecraft: &str) -> Result<Vec<ContentVersion>, String> {
    let releases = versions_of(source, ContentType::Mod, project_id).await?;
    Ok(releases.into_iter().filter(|release| fits(release, loader, minecraft)).collect())
}
