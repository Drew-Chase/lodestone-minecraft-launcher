//! Linking a mod to the project it is on its platform: the search, the pick of a project and its release, and the record.

use super::*;

impl LodestoneLauncher {
    /// Opens the search that links a mod to a project, prefilled with the mod's name, and starts it.
    pub(super) fn open_link(&mut self, file_name: String) -> Task<LodestoneAppMessage> {
        let Some(context) = self.mods_context() else {
            return Task::none();
        };
        let Some(row) = self.instance_page.mods().row(&file_name) else {
            return Task::none();
        };
        let query = row.file.name.clone();
        let source = Source::Modrinth;
        self.instance_page
            .mods_mut()
            .open_dialog(Dialog::Associate(Box::new(Associate::new(file_name, query.clone(), source))));
        self.search_link(&context, source, query)
    }

    /// Starts a search of `source` for mods named `query`, on the instance's loader and Minecraft version.
    pub(super) fn search_link(&self, context: &Context, source: Source, query: String) -> Task<LodestoneAppMessage> {
        let generation = self.instance_page.generation();
        let loader = context.instance.loader.to_db().to_string();
        let minecraft = context.instance.mc_version.clone();
        let asked = query.clone();
        Task::perform(search_mods(source, query, loader, minecraft), move |result| {
            LodestoneAppMessage::ModActions(Message::Searched {
                generation,
                source,
                query: asked.clone(),
                result,
            })
        })
    }

    /// The search the link dialog asked for, on its platform and text.
    pub(super) fn search_from_dialog(&mut self) -> Task<LodestoneAppMessage> {
        let Some(context) = self.mods_context() else {
            return Task::none();
        };
        let Some(Dialog::Associate(associate)) = self.instance_page.mods_mut().dialog_mut() else {
            return Task::none();
        };
        associate.hits = Load::Loading;
        associate.picked = None;
        let (source, query) = (associate.source, associate.query.clone());
        self.search_link(&context, source, query)
    }

    /// The link dialog switched platforms, so its text is searched again on the other one.
    pub(super) fn change_platform(&mut self, source: Source) -> Task<LodestoneAppMessage> {
        let Some(context) = self.mods_context() else {
            return Task::none();
        };
        let Some(Dialog::Associate(associate)) = self.instance_page.mods_mut().dialog_mut() else {
            return Task::none();
        };
        associate.source = source;
        associate.hits = Load::Loading;
        associate.picked = None;
        let query = associate.query.clone();
        self.search_link(&context, source, query)
    }

    /// The search result the link dialog picked. Its releases load, and the release that fits the jar is chosen.
    pub(super) fn pick_from_dialog(&mut self, index: usize) -> Task<LodestoneAppMessage> {
        let Some(context) = self.mods_context() else {
            return Task::none();
        };
        let generation = self.instance_page.generation();
        let Some(Dialog::Associate(associate)) = self.instance_page.mods_mut().dialog_mut() else {
            return Task::none();
        };
        let Load::Loaded(hits) = &associate.hits else {
            return Task::none();
        };
        let Some(project) = hits.get(index).cloned() else {
            return Task::none();
        };
        let source = associate.source;
        let jar = context.mods_dir().join(&associate.file_name);
        associate.picked = Some(Picked {
            project: project.clone(),
            releases: Load::Loading,
            chosen: None,
            jar_sha1: None,
        });
        let loader = context.instance.loader.to_db().to_string();
        let minecraft = context.instance.mc_version.clone();
        let project_id = project.id.clone();
        Task::perform(pick_releases(source, project_id, jar, loader, minecraft), move |result| {
            LodestoneAppMessage::ModActions(Message::Picked {
                generation,
                project: project.clone(),
                result,
            })
        })
    }

    /// The hits of a search for a link arrived. They fill the dialog that asked, if it is still open for the same platform
    /// and text. Each hit's icon is then loaded, and arrives as `Icon`.
    pub(super) fn mod_searched(
        &mut self,
        generation: u64,
        source: Source,
        query: String,
        result: Result<SearchPage, String>,
    ) -> Task<LodestoneAppMessage> {
        let mut icons = Vec::new();
        if let Some(Dialog::Associate(associate)) = self.instance_page.mods_mut().dialog_mut()
            && associate.source == source
            && associate.query == query
        {
            if let Ok(page) = &result {
                icons = page.hits.iter().filter_map(|hit| hit.icon_url.clone()).collect();
            }
            associate.hits = match result {
                Ok(page) => Load::Loaded(page.hits),
                Err(error) => Load::Failed(error),
            };
        }
        Task::batch(icons.into_iter().map(|url| {
            let answer = url.clone();
            // The dialog draws each hit's icon in a 32 px square.
            Task::perform(load_icon(url, crate::app::scale::device_px(32.0)), move |handle| {
                LodestoneAppMessage::ModActions(Message::Icon {
                    generation,
                    url: answer.clone(),
                    handle,
                })
            })
        }))
    }

    /// A hit's icon arrived. It is kept for the search that asked, if its dialog is still open.
    pub(super) fn mod_icon(&mut self, url: String, handle: Option<image::Handle>) {
        if let Some(handle) = handle
            && let Some(Dialog::Associate(associate)) = self.instance_page.mods_mut().dialog_mut()
        {
            associate.icons.insert(url, handle);
        }
    }

    /// The releases of a picked project arrived. They fill the pick that asked, if it is still the pick. The release the
    /// link picks is scrolled into view.
    pub(super) fn mod_picked(&mut self, project: SearchResult, result: Result<PickedReleases, String>) -> Task<LodestoneAppMessage> {
        if let Some(Dialog::Associate(associate)) = self.instance_page.mods_mut().dialog_mut()
            && let Some(picked) = associate.picked.as_mut()
            && picked.project.id == project.id
        {
            match result {
                Ok(found) => {
                    let chosen = found.chosen;
                    picked.releases = Load::Loaded(found.releases);
                    picked.chosen = chosen;
                    picked.jar_sha1 = found.jar_sha1;
                    if let Some(index) = chosen {
                        return scroll_to_release(index);
                    }
                }
                Err(error) => picked.releases = Load::Failed(error),
            }
        }
        Task::none()
    }

    /// Links the mod to the picked project and its chosen release, and writes the record.
    pub(super) fn confirm_link(&mut self) -> Task<LodestoneAppMessage> {
        let (Some(context), Some(db)) = (self.mods_context(), self.db.clone()) else {
            return Task::none();
        };
        let generation = self.instance_page.generation();
        let Some(Dialog::Associate(associate)) = self.instance_page.mods().dialog() else {
            return Task::none();
        };
        let (Some(release), Some(picked)) = (associate.chosen_release(), associate.picked.as_ref()) else {
            return Task::none();
        };
        let record = ModSource {
            instance: context.instance.id.clone(),
            platform: associate.source.key().to_string(),
            project_id: picked.project.id.clone(),
            version_id: release.id.clone(),
            name: picked.project.name.clone(),
            file_name: associate.file_name.clone(),
            sha1: picked.jar_sha1.clone(),
        };
        let file_name = associate.file_name.clone();
        self.instance_page.mods_mut().set_busy(&file_name, true);
        let answered = file_name.clone();
        Task::perform(
            async move { db.record_mod_sources(std::slice::from_ref(&record)).await.map(|()| record) },
            move |result| {
                LodestoneAppMessage::ModActions(Message::Linked {
                    generation,
                    file_name: answered.clone(),
                    result,
                })
            },
        )
    }

    /// A link was recorded. The mod takes its source, the dialog closes, and the mod is checked for a newer release.
    pub(super) fn mod_linked(&mut self, file_name: String, result: Result<ModSource, String>) -> Task<LodestoneAppMessage> {
        self.instance_page.mods_mut().set_busy(&file_name, false);
        match result {
            Ok(record) => {
                let (Some(context), Some(db)) = (self.mods_context(), self.db.clone()) else {
                    return Task::none();
                };
                let mods = self.instance_page.mods_mut();
                mods.close_dialog();
                mods.apply_source(record.clone());
                self.discover.note_sources(std::slice::from_ref(&record));
                let generation = self.instance_page.generation();
                let check = check_mods(db, context.check(), Vec::new(), Some(vec![record]));
                let instance = context.instance.id.clone();
                let check = Task::perform(check, move |result| {
                    LodestoneAppMessage::ModActions(Message::Checked {
                        generation,
                        instance,
                        full: false,
                        result,
                    })
                });
                Task::batch([check, self.project_icons(generation)])
            }
            Err(error) => {
                warn!("linking a mod failed: {error}");
                self.alerts
                    .show(Alert::error(lang::t_with("instance.mods.associate.failed", &[("error", error.as_str())])))
                    .discard()
            }
        }
    }
}

/// The first page of mods a platform has for `text`, on the loader and the Minecraft version.
async fn search_mods(source: Source, text: String, loader: String, minecraft: String) -> Result<SearchPage, String> {
    let query = SearchQuery {
        text,
        loaders: vec![loader],
        game_versions: vec![minecraft],
        limit: SEARCH_LIMIT,
        ..SearchQuery::new(ContentType::Mod)
    };
    match source {
        Source::CurseForge => curseforge()?.search(&query).await.map_err(|error| error.to_string()),
        _ => MODRINTH.search(&query).await.map_err(|error| error.to_string()),
    }
}

/// The picture a platform gives a project, or `None` when the project has none or the picture does not load.
pub(super) async fn project_picture(source: Source, project_id: String) -> Option<image::Handle> {
    let details = details_of(source, ContentType::Mod, &project_id).await.ok()?;
    // The Mods tab draws each project's icon in a 44 px tile.
    load_icon(details.icon_url?, crate::app::scale::device_px(44.0)).await
}

/// The releases of a project a link may name, the one that fits the jar best, and the jar's SHA-1. The release with a file
/// whose hash is the jar's wins. Failing that, the newest that fits the instance does.
async fn pick_releases(source: Source, project_id: String, jar: PathBuf, loader: String, minecraft: String) -> Result<PickedReleases, String> {
    let releases = versions_of(source, ContentType::Mod, &project_id).await?;
    let jar_sha1 = tokio::task::spawn_blocking(move || sha1_of_file(&jar))
        .await
        .map_err(|error| error.to_string())?
        .ok();
    let matching = releases.iter().position(|release| {
        jar_sha1
            .as_deref()
            .is_some_and(|sha1| release.files.iter().any(|file| file.sha1.as_deref() == Some(sha1)))
    });
    let chosen = matching.or_else(|| releases.iter().position(|release| fits(release, &loader, &minecraft)));
    Ok(PickedReleases { releases, chosen, jar_sha1 })
}
