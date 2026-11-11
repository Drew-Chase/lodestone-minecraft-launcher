//! Listing a mod's dependencies and its dependents, and opening one of the dependencies on its platform.

use super::*;

impl LodestoneLauncher {
    /// Opens the dependencies dialog for a mod, and lists the dependencies of its current release.
    pub(super) fn open_dependencies(&mut self, file_name: String) -> Task<LodestoneAppMessage> {
        let (Some(context), Some(db)) = (self.mods_context(), self.db.clone()) else {
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
        self.instance_page.mods_mut().open_dialog(Dialog::Dependencies(Dependencies {
            file_name: file_name.clone(),
            source,
            name,
            lines: Load::Loading,
        }));
        let instance = context.instance.id.clone();
        let answered = file_name.clone();
        Task::perform(dependencies_of(db, instance, record, source), move |result| {
            LodestoneAppMessage::ModActions(Message::Dependencies {
                generation,
                file_name: answered.clone(),
                result,
            })
        })
    }

    /// Opens the dialog that lists the mods in the folder which name a library as a dependency. The folder's jars declare
    /// them, so the dialog is complete at once and needs no platform.
    pub(super) fn open_dependents(&mut self, file_name: String) -> Task<LodestoneAppMessage> {
        if let Some(dependents) = self.instance_page.mods().dependents_of(&file_name) {
            self.instance_page.mods_mut().open_dialog(Dialog::Dependents(dependents));
        }
        Task::none()
    }

    /// The dependencies arrived. They fill the dialog that asked, if it is still open for the same mod.
    pub(super) fn mod_dependencies(&mut self, file_name: String, result: Result<Vec<Line>, String>) {
        if let Some(Dialog::Dependencies(dependencies)) = self.instance_page.mods_mut().dialog_mut()
            && dependencies.file_name == file_name
        {
            dependencies.lines = match result {
                Ok(lines) => Load::Loaded(lines),
                Err(error) => Load::Failed(error),
            };
        }
    }

    /// Opens the resource page of a dependency the dependencies dialog listed.
    pub(super) fn open_dependency(&mut self, index: usize) -> Task<LodestoneAppMessage> {
        let Some(Dialog::Dependencies(dependencies)) = self.instance_page.mods().dialog() else {
            return Task::none();
        };
        let Load::Loaded(lines) = &dependencies.lines else {
            return Task::none();
        };
        let Some(line) = lines.get(index).cloned() else {
            return Task::none();
        };
        let source = dependencies.source;
        self.instance_page.mods_mut().close_dialog();
        self.open_resource(Listing {
            source,
            content: ContentType::Mod,
            project_id: line.project_id,
            name: line.name,
            author: None,
            summary: String::new(),
            downloads: 0,
            icon_url: None,
            web_url: None,
            updated: None,
        })
    }
}

/// The dependencies of a mod's current release, each named, and marked with the jar the instance has for it.
async fn dependencies_of(db: Database, instance: String, record: ModSource, source: Source) -> Result<Vec<Line>, String> {
    let releases = versions_of(source, ContentType::Mod, &record.project_id).await?;
    let release = releases
        .into_iter()
        .find(|release| release.id == record.version_id)
        .ok_or_else(|| lang::t("instance.mods.deps.no_release").to_string())?;
    let dependencies: Vec<VersionDependency> = release
        .dependencies
        .into_iter()
        .filter(|dependency| dependency.project_id.is_some())
        .collect();
    let ids: Vec<String> = dependencies
        .iter()
        .filter_map(|dependency| dependency.project_id.clone())
        .collect::<HashSet<String>>()
        .into_iter()
        .collect();
    let names = named(source, &ids).await;
    let installed = db.mod_sources_of(&instance).await?;
    Ok(dependencies
        .into_iter()
        .filter_map(|dependency| {
            let project_id = dependency.project_id?;
            Some(Line {
                name: names.get(&project_id).cloned().unwrap_or_else(|| project_id.clone()),
                installed: installed
                    .iter()
                    .find(|record| record.project_id == project_id)
                    .map(|record| record.file_name.clone()),
                kind: dependency.kind,
                project_id,
            })
        })
        .collect())
}

/// The names of projects on a platform, keyed by project id. A name that cannot be read is left out, and the project's id
/// stands in for it.
async fn named(source: Source, ids: &[String]) -> HashMap<String, String> {
    match source {
        Source::Modrinth => match MODRINTH.project_names(ids).await {
            Ok(names) => names,
            Err(error) => {
                warn!("reading the names of a mod's dependencies failed: {error}");
                HashMap::new()
            }
        },
        _ => {
            let mut names = HashMap::new();
            for id in ids {
                if let Ok(details) = details_of(source, ContentType::Mod, id).await {
                    names.insert(id.clone(), details.name);
                }
            }
            names
        }
    }
}
