//! Placing a project from the Discover page into the target instance, or into the target world for a datapack. The page
//! asks for a placement, keeps what it placed, and tells the card and the resource page whether the project is there
//! already or on its way.

use lodestone_platforms::ContentType;

use super::target::WorldChoice;
use super::{Destination, Discover, Effect, PlaceOutcome, PlaceRequest, Placement, Project, Source};
use crate::app::shared_library::{self, AddedContent, SharedRecord};

impl Discover {
    /// Starts placing a card's project of the tab in the place the tab names.
    pub(super) fn place(&mut self, project_id: &str) -> Vec<Effect> {
        let project = self.project(project_id, None);
        self.place_with(project)
    }

    /// Starts placing `project` in the place its content names: the target instance, or for a datapack the target world in
    /// it. Nothing starts without a target (and a world, for a datapack), or while the project is already there or on its
    /// way.
    pub fn place_with(&mut self, project: Project) -> Vec<Effect> {
        let Some(destination) = self.destination_for(project.content) else {
            return Vec::new();
        };
        let Some(minecraft) = self.target().map(|choice| choice.mc_version.clone()) else {
            return Vec::new();
        };
        let shared = self.shared_record_of(&project);
        let request = PlaceRequest {
            project,
            minecraft,
            destination,
            shared,
        };
        let key = request.placement();
        if self.placed.contains_key(&key) || self.placing.contains(&key) {
            return Vec::new();
        }
        self.placing.insert(key);
        vec![Effect::Place(Box::new(request))]
    }

    /// Where a project of `content` goes: the target instance, and for a datapack the target world in it. `None` until the
    /// target is picked.
    pub(super) fn destination_for(&self, content: ContentType) -> Option<Destination> {
        let instance = self.target.clone()?;
        Some(match content {
            ContentType::DataPack => Destination::Datapacks {
                instance,
                world: self.target_world.clone()?,
            },
            ContentType::World => Destination::World { instance },
            _ => Destination::Pack { instance },
        })
    }

    /// Where the tab places its projects. `None` until the target is picked.
    pub(super) fn destination(&self) -> Option<Destination> {
        self.destination_for(self.content)
    }

    /// The place a project of `content` on `source` is placed in, when the target names one. A datapack's place is its
    /// world, and every other content's is the instance alone.
    pub(super) fn placement_for(&self, source: Source, content: ContentType, project_id: &str) -> Option<Placement> {
        let instance = self.target.clone()?;
        let scope = match content {
            ContentType::DataPack => self.target_world.clone()?,
            _ => String::new(),
        };
        Some((
            instance,
            scope,
            shared_library::folder_name(content).to_string(),
            source.key().to_string(),
            project_id.to_string(),
        ))
    }

    /// The place a project of the tab would be placed in, when the tab has one.
    pub(super) fn placement_of(&self, project_id: &str) -> Option<Placement> {
        self.placement_for(self.source, self.content, project_id)
    }

    /// The record of a project placed in the target place, when it is there.
    pub(super) fn placed_in_target(&self, project_id: &str) -> Option<&AddedContent> {
        self.placement_of(project_id).and_then(|key| self.placed.get(&key))
    }

    /// Whether a project is being placed in the target place now.
    pub(super) fn placing_in_target(&self, project_id: &str) -> bool {
        self.placement_of(project_id).is_some_and(|key| self.placing.contains(&key))
    }

    /// The download the shared folder holds for `project`, when it has one.
    fn shared_record_of(&self, project: &Project) -> Option<SharedRecord> {
        self.library
            .get(&(
                shared_library::folder_name(project.content).to_string(),
                project.source.key().to_string(),
                project.project_id.clone(),
            ))
            .cloned()
    }

    /// The name of the target world, for the buttons that name the place they add to.
    pub(super) fn target_world_name(&self) -> Option<&str> {
        let folder = self.target_world.as_deref()?;
        self.worlds.iter().find(|world| world.folder == folder).map(|world| world.name.as_str())
    }

    /// Records what the worlds of an instance are, for the world picker. The list is kept only while that instance is the
    /// target, and a world picked that is no longer there is forgotten.
    pub(super) fn set_worlds(&mut self, instance: &str, worlds: Vec<WorldChoice>) -> Vec<Effect> {
        if self.target.as_deref() != Some(instance) {
            return Vec::new();
        }
        if self
            .target_world
            .as_ref()
            .is_some_and(|folder| !worlds.iter().any(|world| &world.folder == folder))
        {
            self.target_world = None;
        }
        self.worlds = worlds;
        Vec::new()
    }

    /// Picks the world the datapacks go to, from the world picker, by its index in the picker's list.
    pub fn select_world(&mut self, index: usize) -> Vec<Effect> {
        self.target_world = self.worlds.get(index).map(|world| world.folder.clone());
        Vec::new()
    }

    /// Records a placement's outcome: the project is kept as placed, its download is kept in the shared folder, and both are
    /// saved. A placement that failed is reported, and the card offers it again.
    pub(super) fn placed_result(&mut self, key: Placement, result: Result<Box<PlaceOutcome>, String>) -> Vec<Effect> {
        self.placing.remove(&key);
        match result {
            Ok(outcome) => {
                let PlaceOutcome { added, shared } = *outcome;
                self.placed.insert(key, added.clone());
                let mut effects = vec![Effect::RecordAdded(added)];
                if let Some(record) = shared {
                    self.library
                        .insert((record.kind.clone(), record.platform.clone(), record.project_id.clone()), record.clone());
                    effects.push(Effect::Record(record));
                }
                effects
            }
            Err(error) => vec![Effect::Notify(error)],
        }
    }

    /// Replaces the placements the database holds, once it has loaded.
    pub fn set_placed(&mut self, records: &[AddedContent]) {
        self.placed = records
            .iter()
            .map(|record| {
                (
                    (
                        record.instance.clone(),
                        record.scope.clone(),
                        record.kind.clone(),
                        record.platform.clone(),
                        record.project_id.clone(),
                    ),
                    record.clone(),
                )
            })
            .collect();
    }
}

#[cfg(test)]
#[path = "../../../../tests/app/pages/discover/place.rs"]
mod tests;
