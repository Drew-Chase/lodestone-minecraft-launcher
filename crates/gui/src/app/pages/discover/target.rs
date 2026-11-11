//! The target instance: the instance a tab's projects are added to, the filters that follow it, and the projects already
//! added to it. Mods are written into the instance's `mods/` folder, and packs into its own pack folders, so the page
//! keeps the instances it can add to and what it has added there.

use lodestone_platforms::ContentType;

use super::{Discover, Effect, Project, Source, placeholder};
use crate::app::pages::library::instance::{Instance, Loader};
use crate::app::shared_library::ModSource;

/// Whether a tab adds its projects to an instance, so it shows the instance selector. Datapacks also name a world in it.
pub fn takes_target(content: ContentType) -> bool {
    matches!(
        content,
        ContentType::Mod | ContentType::ResourcePack | ContentType::Shader | ContentType::DataPack | ContentType::World
    )
}

/// An instance a mod can be added to, with what the Mods tab takes from it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceChoice {
    /// The instance's id.
    pub id: String,
    /// The instance's name, as the selector shows it.
    pub name: String,
    /// The loader the instance runs through.
    pub loader: Loader,
    /// The Minecraft version the instance runs.
    pub mc_version: String,
    /// Whether the instance is locked to its modpack, which keeps mods from being added to or switched in it until it is
    /// unlocked.
    pub locked: bool,
}

impl InstanceChoice {
    /// The choice an instance makes for the Mods tab.
    pub fn of(instance: &Instance) -> Self {
        Self {
            id: instance.id.clone(),
            name: instance.name.clone(),
            loader: instance.loader,
            mc_version: instance.mc_version.clone(),
            locked: instance.locked,
        }
    }

    /// Whether mods can run in the instance at all. A vanilla instance has no loader for them.
    pub fn takes_mods(&self) -> bool {
        self.loader != Loader::Vanilla
    }

    /// Whether mods can be added to the instance, or one it has switched to another release: it takes mods, and it is not
    /// locked to its modpack.
    pub fn accepts_mods(&self) -> bool {
        self.takes_mods() && !self.locked
    }

    /// The loader the search filters by, named as the platforms name loaders. A vanilla instance filters by none.
    fn loader_filter(&self) -> Option<String> {
        self.takes_mods().then(|| self.loader.to_db().to_string())
    }
}

/// A mod to add to an instance: the project and the release to take, and the instance it goes into.
#[derive(Debug, Clone)]
pub struct AddRequest {
    /// The project, with the release the user picked when they picked one.
    pub project: Project,
    /// The instance the mod goes into.
    pub instance: InstanceChoice,
}

/// The key a mod is kept under: the instance it is in, its platform and its project.
fn key_of(record: &ModSource) -> (String, String, String) {
    (record.instance.clone(), record.platform.clone(), record.project_id.clone())
}

impl Discover {
    /// Replaces the instances mods can be added to. The target is forgotten when its instance is no longer listed.
    pub fn set_instances(&mut self, instances: Vec<InstanceChoice>) {
        if self.target.as_ref().is_some_and(|id| !instances.iter().any(|choice| &choice.id == id)) {
            self.target = None;
        }
        self.instances = instances;
    }

    /// Replaces the mods the launcher knows came from a platform, from the database when it opens. Each is kept under its
    /// instance, platform and project, with the record of the release it has.
    pub fn set_added(&mut self, records: &[ModSource]) {
        self.added = records.iter().map(|record| (key_of(record), record.clone())).collect();
    }

    /// The instance the mods go to, once one is picked.
    pub(super) fn target(&self) -> Option<&InstanceChoice> {
        let id = self.target.as_ref()?;
        self.instances.iter().find(|choice| &choice.id == id)
    }

    /// The key the project is kept under in the target instance, when one is picked.
    pub(super) fn target_key(&self, project_id: &str) -> Option<(String, String, String)> {
        self.target
            .as_ref()
            .map(|id| (id.clone(), self.source.key().to_string(), project_id.to_string()))
    }

    /// Whether the project is already in the target instance, at any release.
    pub(super) fn added_to_target(&self, project_id: &str) -> bool {
        self.target_key(project_id).is_some_and(|key| self.added.contains_key(&key))
    }

    /// Whether the project is being added to the target instance now.
    pub(super) fn adding_to_target(&self, project_id: &str) -> bool {
        self.target_key(project_id).is_some_and(|key| self.adding.contains(&key))
    }

    /// Picks the instance the projects go to, from the tab or a resource page. The instance's Minecraft version becomes
    /// a filter, and on the Mods tab its loader too, and the search starts again.
    pub fn select_instance(&mut self, index: usize) -> Vec<Effect> {
        self.target = self.instances.get(index).map(|choice| choice.id.clone());
        self.target_world = None;
        self.worlds.clear();
        self.apply_target_filters();
        let mut effects = self.start(false);
        // A datapack goes into a world, so the page reads the worlds of the instance it was given.
        if self.content == ContentType::DataPack
            && let Some(instance) = self.target.clone()
        {
            effects.push(Effect::LoadWorlds(instance));
        }
        effects
    }

    /// Starts adding a mod, or one of its releases, to the target instance. Nothing starts without a target
    /// instance that takes mods, or when the mod is already in that instance or being added.
    pub fn add_mod(&mut self, project: Project) -> Vec<Effect> {
        let Some(instance) = self.target().filter(|choice| choice.accepts_mods()).cloned() else {
            return Vec::new();
        };
        let key = (instance.id.clone(), project.source.key().to_string(), project.project_id.clone());
        if self.added.contains_key(&key) || self.adding.contains(&key) {
            return Vec::new();
        }
        self.adding.insert(key);
        vec![Effect::AddMod(Box::new(AddRequest { project, instance }))]
    }

    /// Starts swapping the mod's installed jar for the release `project` names, when the target instance has the mod at
    /// another release. Nothing starts when the release is installed already, or a change to the mod is in flight.
    pub fn replace_mod(&mut self, project: Project) -> Vec<Effect> {
        let Some(instance) = self.target().filter(|choice| choice.accepts_mods()).cloned() else {
            return Vec::new();
        };
        let key = (instance.id.clone(), project.source.key().to_string(), project.project_id.clone());
        if self.adding.contains(&key) {
            return Vec::new();
        }
        let Some(installed) = self.added.get(&key).cloned() else {
            return Vec::new();
        };
        if project.version_id.as_deref() == Some(installed.version_id.as_str()) {
            return Vec::new();
        }
        self.adding.insert(key);
        vec![Effect::ReplaceMod {
            request: Box::new(AddRequest { project, instance }),
            old_file: installed.file_name,
        }]
    }

    /// Records the outcome of an add or a replacement. The mods that landed, the dependencies among them, are remembered
    /// and saved, so their projects show as added too, at the release they landed at. A failure is reported.
    pub(super) fn mod_added(
        &mut self,
        instance: String,
        platform: String,
        project_id: String,
        result: Result<Vec<ModSource>, String>,
    ) -> Vec<Effect> {
        self.adding.remove(&(instance, platform, project_id));
        match result {
            Ok(records) => {
                self.added.extend(records.iter().map(|record| (key_of(record), record.clone())));
                vec![Effect::RecordModSources(records)]
            }
            Err(error) => vec![Effect::Notify(error)],
        }
    }

    /// Keeps the mods the launcher knows came from a platform in step with the database. Each record replaces the one its
    /// instance, platform and project already had.
    pub fn note_sources(&mut self, records: &[ModSource]) {
        self.added.extend(records.iter().map(|record| (key_of(record), record.clone())));
    }

    /// Forgets the record of the jar `file_name` in `instance`, once the jar is gone.
    pub fn forget_source(&mut self, instance: &str, file_name: &str) {
        self.added
            .retain(|_, record| record.instance != instance || record.file_name != file_name);
    }

    /// Moves the record of the jar `from` in `instance` to its new name, once the jar is renamed.
    pub fn renamed(&mut self, instance: &str, from: &str, to: &str) {
        for record in self.added.values_mut() {
            if record.instance == instance && record.file_name == from {
                record.file_name = to.to_string();
            }
        }
    }

    /// The filters follow the target instance: its Minecraft version, and on the Mods tab its loader too. The other picks
    /// stay as the user made them.
    pub(super) fn apply_target_filters(&mut self) {
        if !self.takes_target_version() {
            return;
        }
        let Some(choice) = self.target().cloned() else {
            return;
        };
        if self.content == ContentType::Mod {
            self.filters.loaders = choice.loader_filter().into_iter().collect();
        }
        self.filters.game_versions = vec![choice.mc_version];
        self.filters.tidy();
    }

    /// Points the page at the Mods tab for an instance, as the instance page's Add button does.
    pub fn prepare_mods(&mut self, choice: InstanceChoice) {
        self.prepare_content(choice, ContentType::Mod);
    }

    /// Points the page at a tab for an instance, as the instance page's Add buttons do. The platform moves to one that hosts
    /// the tab when the current one does not, and the filters take the instance's version (and loader, on mods). The search
    /// starts when the page opens. A tab no platform in this build hosts is left as it is.
    pub fn prepare_content(&mut self, choice: InstanceChoice, content: ContentType) {
        let Some(source) = Source::ALL
            .into_iter()
            .find(|candidate| candidate.enabled() && candidate.supports(content))
        else {
            return;
        };
        match self.instances.iter_mut().find(|known| known.id == choice.id) {
            Some(known) => *known = choice.clone(),
            None => self.instances.push(choice.clone()),
        }
        self.target = Some(choice.id);
        self.target_world = None;
        self.worlds.clear();
        if source != self.source {
            self.source = source;
            self.placeholder = placeholder(source);
        }
        self.content = content;
        self.reset_filters();
    }
}

/// A world of an instance, as the world picker of the datapack section lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldChoice {
    /// The save's folder name in the instance's `saves/`.
    pub folder: String,
    /// The world's name, as the game shows it.
    pub name: String,
}

#[cfg(test)]
#[path = "../../../../tests/app/pages/discover/target.rs"]
mod tests;
