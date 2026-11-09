//! The Versions tab: the releases of the modpack the instance came from, the release the instance is on, and the actions that move it
//! between them. The page lists the releases and sends the actions. The root asks the user, prepares the release, and queues the
//! update, so the page holds no files and starts no work.

mod view;

use iced::Task;
use lodestone_platforms::{ContentType, ContentVersion};

use crate::app::discover::versions_of;
use crate::app::pages::discover::Source;
use crate::app::pages::library::instance::Instance;

/// How many releases the list shows at a time. Each press of its button adds this many more.
pub const PAGE: usize = 25;

/// The messages the Versions tab sends and handles.
#[derive(Debug, Clone)]
pub enum Message {
    /// The releases arrived, or the listing failed. A listing of an earlier open is dropped.
    Loaded {
        generation: u64,
        result: Result<Vec<ContentVersion>, String>,
    },
    /// Lists the releases again, after a failure.
    Retry,
    /// Shows the next page of releases.
    More,
    /// Moves the instance onto the release with this id. The root asks first, then prepares the release and queues the update.
    Move(String),
    /// Records the release with this id as the one the instance is on. No file changes.
    Record(String),
}

/// What the tab knows of the releases.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
enum Listing {
    /// Nothing is listed: the instance did not come from a platform, or the tab was not opened.
    #[default]
    Idle,
    Loading,
    Loaded(Vec<ContentVersion>),
    Failed(String),
}

/// How a release stands against the release the instance is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Standing {
    /// The instance is on this release.
    Installed,
    /// A release the platform lists before the one the instance is on, so a newer one.
    Newer,
    /// A release the platform lists after the one the instance is on, so an older one.
    Older,
    /// The instance is on a release the platform does not list, so the order between them is not known.
    Other,
    /// The release the instance is on is not known.
    Unknown,
}

/// The standing of the release `id` in `versions`, which the platform lists newest first, against the release `installed`.
pub(crate) fn standing(versions: &[ContentVersion], installed: Option<&str>, id: &str) -> Standing {
    let Some(installed) = installed else {
        return Standing::Unknown;
    };
    if installed == id {
        return Standing::Installed;
    }
    match (
        versions.iter().position(|version| version.id == id),
        versions.iter().position(|version| version.id == installed),
    ) {
        (Some(at), Some(current)) if at < current => Standing::Newer,
        (Some(_), Some(_)) => Standing::Older,
        _ => Standing::Other,
    }
}

/// The Versions tab's state.
#[derive(Debug, Default)]
pub struct State {
    /// The instance the listing belongs to, by id. Empty before the tab opens.
    instance: String,
    /// The platform the instance came from, when it came from one.
    source: Option<Source>,
    /// The project's id on that platform.
    project: String,
    /// The release the instance is on, by its id there, and its number. Both are `None` when the release is not known.
    installed_id: Option<String>,
    installed_number: Option<String>,
    /// The Minecraft version and loader the instance runs, to mark the releases that would change them.
    minecraft: String,
    loader: String,
    listing: Listing,
    /// How many releases the list shows.
    shown: usize,
    /// Bumped by every open, so a listing that arrives after the tab moved to another instance is dropped.
    generation: u64,
    /// The release whose update is being prepared, while the root reads it.
    preparing: Option<String>,
}

impl State {
    /// Shows the tab for `instance`, and lists the releases of its platform. An instance that did not come from a platform lists none.
    pub fn open(&mut self, instance: &Instance) -> Task<Message> {
        self.generation += 1;
        self.instance = instance.id.clone();
        self.source = instance.source_platform.as_deref().and_then(Source::from_key);
        self.project = instance.source_project_id.clone().unwrap_or_default();
        self.minecraft = instance.mc_version.clone();
        self.loader = instance.loader.to_db().to_string();
        self.installed_id = instance.source_version_id.clone();
        self.installed_number = instance.source_version.clone();
        self.shown = PAGE;
        self.preparing = None;
        self.list()
    }

    /// Reads the instance's release again, after an update or a record changed it. The releases listed stay as they are.
    pub fn refresh(&mut self, instance: &Instance) {
        if instance.id != self.instance {
            return;
        }
        self.installed_id = instance.source_version_id.clone();
        self.installed_number = instance.source_version.clone();
        self.minecraft = instance.mc_version.clone();
        self.loader = instance.loader.to_db().to_string();
    }

    /// Starts the listing of the releases. A tab for an instance with no platform has nothing to list.
    fn list(&mut self) -> Task<Message> {
        let Some(source) = self.source else {
            self.listing = Listing::Idle;
            return Task::none();
        };
        self.listing = Listing::Loading;
        let (generation, project) = (self.generation, self.project.clone());
        Task::perform(async move { versions_of(source, ContentType::Modpack, &project).await }, move |result| {
            Message::Loaded { generation, result }
        })
    }

    /// Handles a message. The moves and records are the root's to carry out, so the page does nothing with them.
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Loaded { generation, result } if generation == self.generation => {
                self.listing = match result {
                    Ok(versions) => Listing::Loaded(versions),
                    Err(error) => Listing::Failed(error),
                };
                Task::none()
            }
            Message::Loaded { .. } | Message::Move(_) | Message::Record(_) => Task::none(),
            Message::Retry => self.list(),
            Message::More => {
                self.shown += PAGE;
                Task::none()
            }
        }
    }

    /// The release with `id`, from the listing the tab shows.
    pub fn release(&self, id: &str) -> Option<&ContentVersion> {
        match &self.listing {
            Listing::Loaded(versions) => versions.iter().find(|version| version.id == id),
            _ => None,
        }
    }

    /// Marks the release whose update the root is preparing, or clears the mark.
    pub fn set_preparing(&mut self, id: Option<String>) {
        self.preparing = id;
    }

    /// The release whose update is being prepared, while one is.
    pub fn preparing(&self) -> Option<&str> {
        self.preparing.as_deref()
    }
}

#[cfg(test)]
#[path = "../../../../../tests/app/pages/instance/versions/mod.rs"]
mod tests;
