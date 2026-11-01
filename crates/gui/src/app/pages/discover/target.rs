//! The Mods tab: the instance mods are added to, the filters that follow it, and the mods already added to it.
//! Adding a mod is a download into the instance's own folder, so the page keeps the instances it can add to and
//! the mods it has added there.

use lodestone_platforms::ContentType;

use super::{Discover, Effect, Project, Source, placeholder};
use crate::app::pages::library::instance::{Instance, Loader};
use crate::app::shared_library::ModSource;

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
}

impl InstanceChoice {
    /// The choice an instance makes for the Mods tab.
    pub fn of(instance: &Instance) -> Self {
        Self {
            id: instance.id.clone(),
            name: instance.name.clone(),
            loader: instance.loader,
            mc_version: instance.mc_version.clone(),
        }
    }

    /// Whether mods can run in the instance at all. A vanilla instance has no loader for them.
    pub fn takes_mods(&self) -> bool {
        self.loader != Loader::Vanilla
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

    /// Picks the instance the mods go to, from the Mods tab or a resource page. On the Mods tab the instance's loader
    /// and Minecraft version become the filters, and the search starts again.
    pub fn select_instance(&mut self, index: usize) -> Vec<Effect> {
        self.target = self.instances.get(index).map(|choice| choice.id.clone());
        if self.content != ContentType::Mod {
            return Vec::new();
        }
        self.apply_target_filters();
        self.start(false)
    }

    /// Starts adding a mod, or one of its releases, to the target instance. Nothing starts without a target
    /// instance that takes mods, or when the mod is already in that instance or being added.
    pub fn add_mod(&mut self, project: Project) -> Vec<Effect> {
        let Some(instance) = self.target().filter(|choice| choice.takes_mods()).cloned() else {
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
        let Some(instance) = self.target().filter(|choice| choice.takes_mods()).cloned() else {
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

    /// On the Mods tab, the filters follow the target instance: its loader and Minecraft version. Other tabs keep
    /// the filters the user picked.
    pub(super) fn apply_target_filters(&mut self) {
        if self.content != ContentType::Mod {
            return;
        }
        let Some(choice) = self.target().cloned() else {
            return;
        };
        self.loader = choice.loader_filter();
        self.game_version = Some(choice.mc_version);
    }

    /// Points the page at the Mods tab for an instance, as the instance page's Add button does. The platform moves
    /// to one that hosts mods when the current one does not, and the filters take the instance's loader and version.
    /// The search starts when the page opens.
    pub fn prepare_mods(&mut self, choice: InstanceChoice) {
        match self.instances.iter_mut().find(|known| known.id == choice.id) {
            Some(known) => *known = choice.clone(),
            None => self.instances.push(choice.clone()),
        }
        self.target = Some(choice.id);
        if !self.source.supports(ContentType::Mod) {
            self.source = Source::Modrinth;
            self.placeholder = placeholder(self.source);
        }
        self.content = ContentType::Mod;
        self.reset_filters();
    }
}

#[cfg(test)]
#[path = "../../../../tests/app/pages/discover/target.rs"]
mod tests;
