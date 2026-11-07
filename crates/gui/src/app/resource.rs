//! The resource page's side effects: its details, releases, pictures and bundled projects, and the actions it asks
//! for. The page decides what it needs. This module turns those requests into tasks, and sends the answers back as
//! messages. An action runs through the Discover page's own methods, so a project's state is the same on both pages.

use iced::Task;

use super::discover::{bundled_of, details_of, versions_of};
use super::instances::open_source;
use super::pages::Page;
use super::pages::discover::{Effect as DiscoverEffect, Project, ProjectStatus, load_image};
use super::pages::resource::{Action, Effect, Listing, Message};
use super::{LodestoneAppMessage, LodestoneLauncher, lang};

impl LodestoneLauncher {
    /// Opens a project's resource page. The card's data shows at once, and the details and releases are asked for.
    pub(super) fn open_resource(&mut self, listing: Listing) -> Task<LodestoneAppMessage> {
        let source = listing.source;
        let content = listing.content;
        let project_id = listing.project_id.clone();
        let effects = self.resource.show(listing);
        let generation = self.resource.generation();
        let details_id = project_id.clone();
        let details = Task::perform(async move { details_of(source, content, &details_id).await }, move |result| {
            LodestoneAppMessage::Resource(Message::Details {
                generation,
                result: result.map(Box::new),
            })
        });
        let releases = Task::perform(async move { versions_of(source, content, &project_id).await }, move |result| {
            LodestoneAppMessage::Resource(Message::Versions { generation, result })
        });
        let mut tasks = vec![self.navigate(Page::Resource), details, releases];
        tasks.extend(effects.into_iter().map(|effect| self.resource_effect(effect)));
        Task::batch(tasks)
    }

    /// Routes one message of the resource page: the page updates, and its effects run.
    pub(super) fn resource_message(&mut self, message: Message) -> Task<LodestoneAppMessage> {
        let effects = self.resource.update(message);
        Task::batch(effects.into_iter().map(|effect| self.resource_effect(effect)))
    }

    /// The status of the project the page shows, as the Discover page knows it.
    pub(super) fn resource_status(&self) -> Option<ProjectStatus> {
        let listing = self.resource.listing()?;
        Some(self.discover.status_of(listing.source, listing.content, &listing.project_id))
    }

    /// Carries out one effect of the resource page.
    fn resource_effect(&mut self, effect: Effect) -> Task<LodestoneAppMessage> {
        match effect {
            Effect::Pictures(urls) => Task::batch(urls.into_iter().map(|url| {
                let answer = url.clone();
                Task::perform(load_image(url), move |handle| {
                    LodestoneAppMessage::Resource(Message::Picture { url: answer, handle })
                })
            })),
            Effect::Contents { generation, version } => {
                let version_id = version.id.clone();
                Task::perform(async move { bundled_of(&version).await }, move |result| {
                    LodestoneAppMessage::Resource(Message::Contents {
                        generation,
                        version_id,
                        result,
                    })
                })
            }
            Effect::Act(action) => self.resource_action(action),
            Effect::Open(url) => open_source(url),
        }
    }

    /// Carries out an action the page asked for on the project it shows, through the Discover page's methods.
    fn resource_action(&mut self, action: Action) -> Task<LodestoneAppMessage> {
        let Some(listing) = self.resource.listing().cloned() else {
            return Task::none();
        };
        let project = |version_id: Option<String>| Project {
            source: listing.source,
            content: listing.content,
            project_id: listing.project_id.clone(),
            name: listing.name.clone(),
            web_url: listing.web_url.clone(),
            icon_url: listing.icon_url.clone(),
            game_versions: Vec::new(),
            version_id,
        };
        let effects: Vec<DiscoverEffect> = match action {
            Action::Download(version) => self.discover.download(project(version)),
            Action::Install(version) => self.discover.install(project(version)),
            Action::Add(version) => self.discover.add_mod(project(version)),
            Action::Replace(version) => {
                // A game may have the jar open, so the swap is refused while one runs on the instance, as the Mods tab's switch is.
                let target = self.discover.status_of(listing.source, listing.content, &listing.project_id).target;
                if target.is_some_and(|target| self.games.is_active(&target.id)) {
                    vec![DiscoverEffect::Notify(lang::t("instance.mods.running").to_string())]
                } else {
                    self.discover.replace_mod(project(Some(version)))
                }
            }
            Action::OpenInstance(id) => vec![DiscoverEffect::OpenInstance(id)],
            Action::PickInstance(index) => self.discover.select_instance(index),
            Action::Place(version) => self.discover.place_with(project(version)),
            Action::PickWorld(index) => self.discover.select_world(index),
            Action::OpenWeb => listing.web_url.map(DiscoverEffect::Open).into_iter().collect(),
        };
        Task::batch(effects.into_iter().map(|effect| self.discover_effect(effect)))
    }
}
