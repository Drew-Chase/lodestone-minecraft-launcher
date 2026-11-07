//! The resource page: one project from a platform, with its overview, gallery, changelog and versions, and a page for
//! each version. A Discover card or the hero opens it. The page keeps what it has been told, and asks for what it
//! needs through [`Effect`]s. The root runs them and sends the answers back tagged with the page's generation, so an
//! answer for a project the page has since left is dropped.

mod changelog;
mod gallery;
mod header;
mod overview;
mod parts;
mod readme;
mod sidebar;
mod version;
mod versions;

use std::collections::{HashMap, HashSet};

use iced::widget::{column, container, image, markdown};
use iced::{Element, Fill, Padding, Size};
use lodestone_platforms::{BundledProject, Channel, ContentDetails, ContentType, ContentVersion, DescriptionFormat};

use crate::app::components::{Tabs, scrollbar};
use crate::app::lang;
use crate::app::pages::discover::{ProjectStatus, Source};

/// The page's side margin, shared by its header and its body.
const GUTTER: f32 = 28.0;
/// The versions, changelogs and gallery pictures a page of a list shows, at most.
const PER_PAGE: usize = 20;
/// The gallery pictures a page of the gallery shows.
const GALLERY_PER_PAGE: usize = 12;
/// The most readme pictures the page loads. The rest keep their alt text.
const README_PICTURES: usize = 12;
/// The channels the Versions tab filters by, in the order its selector lists them. The first is any.
const CHANNELS: [Option<Channel>; 4] = [None, Some(Channel::Release), Some(Channel::Beta), Some(Channel::Alpha)];

/// The project a Discover card or the hero showed. The page shows it while the details load.
#[derive(Debug, Clone)]
pub struct Listing {
    /// The platform the project is on.
    pub source: Source,
    /// The content type the project is, which decides its actions.
    pub content: ContentType,
    /// The project's id on that platform.
    pub project_id: String,
    /// The project's name.
    pub name: String,
    /// The author's name, when the card showed one.
    pub author: Option<String>,
    /// The short description the card showed.
    pub summary: String,
    /// The download count the card showed.
    pub downloads: u64,
    /// The project's icon, when it has one.
    pub icon_url: Option<String>,
    /// The project's page on the platform, when it has one.
    pub web_url: Option<String>,
    /// When the project was last updated, as the platform writes it.
    pub updated: Option<String>,
}

/// An answer the page is waiting for, or has.
#[derive(Debug, Default)]
pub enum Load<T> {
    /// The request is out.
    #[default]
    Loading,
    /// The answer came back.
    Loaded(T),
    /// The request failed, with the reason.
    Failed(String),
}

/// The tabs of the page. A tab shows only when the platform has what it shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    /// The readme, with the facts beside it.
    #[default]
    Overview,
    /// The project's pictures.
    Gallery,
    /// Each version's changes.
    Changelog,
    /// The versions, and a page for each.
    Versions,
}

/// What the header or a row asks the launcher to do with the project, or with one of its releases. `None` takes the
/// newest release that fits the project's filters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Download the release into the shared folder.
    Download(Option<String>),
    /// Install the modpack from the release.
    Install(Option<String>),
    /// Add the mod to the instance the Mods tab picked.
    Add(Option<String>),
    /// Swap the mod the instance has for the release picked, which is another release of it.
    Replace(String),
    /// Open the instance a modpack was installed as.
    OpenInstance(String),
    /// Pick the instance the mods go to, by its index in the selector.
    PickInstance(usize),
    /// Place the release in the instance or world the page picked: a pack, a datapack, or a world imported as a save.
    Place(Option<String>),
    /// Pick the world the datapacks go to, by its index in the world selector.
    PickWorld(usize),
    /// Open the project's page on its platform.
    OpenWeb,
}

/// Messages the resource page handles.
#[derive(Debug, Clone)]
pub enum Message {
    /// A tab was picked.
    Tab(Tab),
    /// The details came back for the project of `generation`.
    Details {
        generation: u64,
        result: Result<Box<ContentDetails>, String>,
    },
    /// The versions came back for the project of `generation`.
    Versions {
        generation: u64,
        result: Result<Vec<ContentVersion>, String>,
    },
    /// The projects a modpack release bundles came back.
    Contents {
        generation: u64,
        version_id: String,
        result: Result<Vec<BundledProject>, String>,
    },
    /// A picture came back, or could not be had (`None`).
    Picture { url: String, handle: Option<image::Handle> },
    /// An action was pressed.
    Act(Action),
    /// A link was pressed in a readme, a changelog or the sidebar.
    Link(String),
    /// A page of the gallery was picked, by its index.
    GalleryPage(usize),
    /// A gallery picture was pressed, by its index in the gallery.
    OpenPicture(usize),
    /// The viewer moved to the picture before or after the one it shows.
    StepPicture(isize),
    /// The viewer was closed.
    ClosePicture,
    /// A page of the changelog was picked, by its index.
    ChangelogPage(usize),
    /// A page of the versions was picked, by its index.
    VersionsPage(usize),
    /// A channel was picked in the versions filter, by its index in [`CHANNELS`].
    ChannelSelected(usize),
    /// A game version was picked in the versions filter. Index 0 means any.
    GameVersionSelected(usize),
    /// A version was opened, by its index in the versions the platform listed.
    OpenVersion(usize),
    /// The open version was closed.
    CloseVersion,
}

/// What the root does for the page after an update.
#[derive(Debug, Clone)]
pub enum Effect {
    /// Load these pictures.
    Pictures(Vec<String>),
    /// Load the projects a modpack release bundles, for the answer of `generation`.
    Contents { generation: u64, version: Box<ContentVersion> },
    /// Carry out an action on the project.
    Act(Action),
    /// Open a link in the browser.
    Open(String),
}

/// The resource page's state.
#[derive(Default)]
pub struct ResourcePage {
    listing: Option<Listing>,
    /// Bumped by every [`ResourcePage::show`]. An answer for an older generation is for a project the page no
    /// longer shows, so it is dropped.
    generation: u64,
    details: Load<Box<ContentDetails>>,
    versions: Load<Vec<ContentVersion>>,
    /// The Minecraft versions the releases name, in the order the platform lists them.
    game_versions: Vec<String>,
    /// The overview's readme, parsed once when the details land.
    readme: Option<markdown::Content>,
    /// The changelog of each version, parsed when its page is shown, by the version's index.
    changelogs: HashMap<usize, markdown::Content>,
    /// The pictures that have loaded, by address.
    pictures: HashMap<String, image::Handle>,
    /// The addresses already asked for, loaded or not, so none is asked for twice.
    requested: HashSet<String>,
    /// The projects the open modpack version bundles.
    contents: Load<Vec<BundledProject>>,
    /// The version the contents are for, by id.
    contents_for: Option<String>,
    tab: Tab,
    gallery_page: usize,
    /// The gallery picture open in the viewer, by its index in the gallery.
    gallery_open: Option<usize>,
    changelog_page: usize,
    versions_page: usize,
    channel: Option<Channel>,
    game_version: Option<String>,
    /// The version whose page is open, by its index in the versions.
    version_open: Option<usize>,
}

impl ResourcePage {
    /// Shows a project, forgetting what the page held for the one before. The requests the page needs come back as
    /// effects, and the answers carry [`ResourcePage::generation`].
    pub fn show(&mut self, listing: Listing) -> Vec<Effect> {
        let generation = self.generation + 1;
        *self = Self {
            generation,
            listing: Some(listing),
            ..Self::default()
        };
        let icon = self.listing.as_ref().and_then(|listing| listing.icon_url.clone());
        self.wanted_all(icon)
    }

    /// The generation the answers for the project shown must carry.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// The project the page shows, once one has been opened.
    pub fn listing(&self) -> Option<&Listing> {
        self.listing.as_ref()
    }

    /// Carries out one message, and returns the requests it starts.
    pub fn update(&mut self, message: Message) -> Vec<Effect> {
        match message {
            Message::Tab(tab) => {
                self.tab = tab;
                match tab {
                    Tab::Changelog => self.changelog_effects(),
                    Tab::Gallery => self.gallery_effects(),
                    Tab::Overview | Tab::Versions => Vec::new(),
                }
            }
            Message::Details { generation, result } if generation == self.generation => match result {
                Ok(details) => {
                    self.readme = readme_text(&details).map(|text| markdown::Content::parse(&readme::to_markdown(text)));
                    let mut urls: Vec<String> = details.info.creators.iter().filter_map(|creator| creator.avatar_url.clone()).collect();
                    urls.extend(self.readme.as_ref().map(readme_pictures).unwrap_or_default());
                    self.details = Load::Loaded(details);
                    let mut effects = self.gallery_effects();
                    effects.extend(self.wanted_all(urls));
                    effects
                }
                Err(error) => {
                    self.details = Load::Failed(error);
                    Vec::new()
                }
            },
            Message::Versions { generation, result } if generation == self.generation => match result {
                Ok(versions) => {
                    self.game_versions = game_versions_of(&versions);
                    self.versions = Load::Loaded(versions);
                    if self.tab == Tab::Changelog {
                        self.changelog_effects()
                    } else {
                        Vec::new()
                    }
                }
                Err(error) => {
                    self.versions = Load::Failed(error);
                    Vec::new()
                }
            },
            Message::Contents {
                generation,
                version_id,
                result,
            } if generation == self.generation && self.contents_for.as_deref() == Some(version_id.as_str()) => match result {
                Ok(bundled) => {
                    let urls: Vec<String> = bundled.iter().filter_map(|project| project.icon_url.clone()).collect();
                    self.contents = Load::Loaded(bundled);
                    self.wanted_all(urls)
                }
                Err(error) => {
                    self.contents = Load::Failed(error);
                    Vec::new()
                }
            },
            Message::Picture { url, handle } => {
                if let Some(handle) = handle {
                    self.pictures.insert(url, handle);
                }
                Vec::new()
            }
            Message::Act(action) => vec![Effect::Act(action)],
            Message::Link(url) => vec![Effect::Open(url)],
            Message::GalleryPage(page) => {
                self.gallery_page = page;
                self.gallery_effects()
            }
            Message::OpenPicture(index) => self.open_picture(index),
            Message::StepPicture(step) => {
                let count = self.gallery().len();
                match self
                    .gallery_open
                    .and_then(|index| index.checked_add_signed(step))
                    .filter(|index| *index < count)
                {
                    Some(index) => self.open_picture(index),
                    None => Vec::new(),
                }
            }
            Message::ClosePicture => {
                self.gallery_open = None;
                Vec::new()
            }
            Message::ChangelogPage(page) => {
                self.changelog_page = page;
                self.changelog_effects()
            }
            Message::VersionsPage(page) => {
                self.versions_page = page;
                Vec::new()
            }
            Message::ChannelSelected(index) => {
                self.channel = CHANNELS.get(index).copied().flatten();
                self.versions_page = 0;
                Vec::new()
            }
            Message::GameVersionSelected(index) => {
                self.game_version = index.checked_sub(1).and_then(|index| self.game_versions.get(index)).cloned();
                self.versions_page = 0;
                Vec::new()
            }
            Message::OpenVersion(index) => self.open_version(index),
            Message::CloseVersion => {
                self.version_open = None;
                Vec::new()
            }
            // An answer for a project the page no longer shows, or for a version it has closed.
            Message::Details { .. } | Message::Versions { .. } | Message::Contents { .. } => Vec::new(),
        }
    }

    /// The tabs the page shows: the ones the platform has data for. The overview is always there.
    pub fn tabs(&self) -> Vec<Tab> {
        let mut tabs = vec![Tab::Overview];
        if self.details().is_some_and(|details| !details.info.gallery.is_empty()) {
            tabs.push(Tab::Gallery);
        }
        match &self.versions {
            Load::Loaded(versions) => {
                if versions.iter().any(|version| version.changelog.is_some()) {
                    tabs.push(Tab::Changelog);
                }
                if !versions.is_empty() {
                    tabs.push(Tab::Versions);
                }
            }
            Load::Loading => tabs.push(Tab::Versions),
            Load::Failed(_) => {}
        }
        tabs
    }

    /// The page. The header and the tabs stay in place, and the tab's content scrolls beneath them.
    pub fn view(&self, window: Size, status: &ProjectStatus) -> Element<'_, Message> {
        let tabs = self.tabs();
        let shown = if tabs.contains(&self.tab) { self.tab } else { Tab::Overview };
        let body: Element<'_, Message> = match shown {
            Tab::Overview => overview::view(self, window),
            Tab::Gallery => gallery::view(self, window),
            Tab::Changelog => changelog::view(self),
            Tab::Versions => match self
                .version_open
                .and_then(|index| self.loaded_versions().get(index).map(|version| (index, version)))
            {
                Some((index, version)) => version::view(self, index, version, status),
                None => versions::view(self, status),
            },
        };
        let active = tabs.iter().position(|tab| *tab == shown).unwrap_or_default();
        column![
            container(header::view(self, status)).padding(Padding::new(0.0).top(22).left(GUTTER).right(GUTTER)),
            tab_bar(&tabs, active),
            scrollbar::scroll(
                container(body)
                    .padding(Padding::new(0.0).top(22).left(GUTTER).right(GUTTER).bottom(40))
                    .width(Fill)
            )
            .width(Fill)
            .height(Fill),
        ]
        .width(Fill)
        .height(Fill)
        .into()
    }

    /// The gallery picture open in the viewer, over the window, when one is open.
    pub fn viewer(&self) -> Option<Element<'_, Message>> {
        let index = self.gallery_open?;
        let picture = self.gallery().get(index)?;
        Some(gallery::viewer(self, index, picture))
    }

    /// The details, once they have loaded.
    fn details(&self) -> Option<&ContentDetails> {
        match &self.details {
            Load::Loaded(details) => Some(details),
            Load::Loading | Load::Failed(_) => None,
        }
    }

    /// The gallery pictures, in the platform's order. Empty until the details have loaded.
    fn gallery(&self) -> &[lodestone_platforms::GalleryImage] {
        self.details().map_or(&[], |details| details.info.gallery.as_slice())
    }

    /// The versions the platform listed, once they have loaded.
    fn loaded_versions(&self) -> &[ContentVersion] {
        match &self.versions {
            Load::Loaded(versions) => versions,
            Load::Loading | Load::Failed(_) => &[],
        }
    }

    /// The versions the Versions tab lists, by their index, after its channel and game-version filters.
    fn filtered_versions(&self) -> Vec<usize> {
        self.loaded_versions()
            .iter()
            .enumerate()
            .filter(|(_, version)| self.channel.is_none_or(|channel| version.channel == Some(channel)))
            .filter(|(_, version)| {
                self.game_version
                    .as_ref()
                    .is_none_or(|wanted| version.minecraft_versions.contains(wanted))
            })
            .map(|(index, _)| index)
            .collect()
    }

    /// The picture at `url`, once it has loaded.
    fn picture(&self, url: &str) -> Option<&image::Handle> {
        self.pictures.get(url)
    }

    /// The addresses not yet asked for, each returned once, as one picture request. Nothing is asked for when all
    /// of them were asked for already.
    fn wanted_all(&mut self, urls: impl IntoIterator<Item = String>) -> Vec<Effect> {
        let fresh: Vec<String> = urls.into_iter().filter(|url| self.requested.insert(url.clone())).collect();
        if fresh.is_empty() { Vec::new() } else { vec![Effect::Pictures(fresh)] }
    }

    /// The thumbnails of the gallery page on screen.
    fn gallery_effects(&mut self) -> Vec<Effect> {
        let thumbnails: Vec<String> = self
            .gallery()
            .iter()
            .skip(self.gallery_page * GALLERY_PER_PAGE)
            .take(GALLERY_PER_PAGE)
            .map(|picture| picture.thumbnail_url.clone())
            .collect();
        self.wanted_all(thumbnails)
    }

    /// Opens a gallery picture in the viewer, and asks for its full-size copy.
    fn open_picture(&mut self, index: usize) -> Vec<Effect> {
        let Some(picture) = self.gallery().get(index) else {
            return Vec::new();
        };
        let full = picture.full_url.clone();
        self.gallery_open = Some(index);
        self.wanted_all(Some(full))
    }

    /// Parses the changelogs of the changelog page on screen, each once, and asks for the pictures they show.
    fn changelog_effects(&mut self) -> Vec<Effect> {
        let start = self.changelog_page * PER_PAGE;
        let pending: Vec<(usize, String)> = self
            .loaded_versions()
            .iter()
            .enumerate()
            .skip(start)
            .take(PER_PAGE)
            .filter(|(index, _)| !self.changelogs.contains_key(index))
            .filter_map(|(index, version)| version.changelog.clone().map(|changelog| (index, changelog)))
            .collect();
        let mut urls = Vec::new();
        for (index, changelog) in pending {
            let content = markdown::Content::parse(&readme::to_markdown(&changelog));
            urls.extend(readme_pictures(&content));
            self.changelogs.insert(index, content);
        }
        self.wanted_all(urls)
    }

    /// Opens a version's page: its changelog is parsed, and a Modrinth modpack's bundled projects are asked for.
    fn open_version(&mut self, index: usize) -> Vec<Effect> {
        let Some(version) = self.loaded_versions().get(index).cloned() else {
            return Vec::new();
        };
        self.version_open = Some(index);
        let mut effects = Vec::new();
        let changelog = if self.changelogs.contains_key(&index) {
            None
        } else {
            version.changelog.clone()
        };
        if let Some(changelog) = changelog {
            let content = markdown::Content::parse(&readme::to_markdown(&changelog));
            effects.extend(self.wanted_all(readme_pictures(&content)));
            self.changelogs.insert(index, content);
        }
        let bundles = self
            .listing
            .as_ref()
            .is_some_and(|listing| listing.source == Source::Modrinth && listing.content == ContentType::Modpack);
        if bundles {
            self.contents = Load::Loading;
            self.contents_for = Some(version.id.clone());
            effects.push(Effect::Contents {
                generation: self.generation,
                version: Box::new(version),
            });
        }
        effects
    }
}

/// The readme a project's details carry, when the platform writes it as markdown.
fn readme_text(details: &ContentDetails) -> Option<&str> {
    match details.description_format {
        DescriptionFormat::Markdown => details.description.as_deref(),
        DescriptionFormat::Html => None,
    }
}

/// The pictures of a parsed readme or changelog, sorted so the same readme always asks in the same order, up to
/// the most the page loads.
fn readme_pictures(content: &markdown::Content) -> Vec<String> {
    let mut urls: Vec<String> = content.images().iter().cloned().collect();
    urls.sort();
    urls.truncate(README_PICTURES);
    urls
}

/// The Minecraft versions the releases name, once each, in the order they first appear.
fn game_versions_of(versions: &[ContentVersion]) -> Vec<String> {
    let mut seen = HashSet::new();
    versions
        .iter()
        .flat_map(|version| version.minecraft_versions.iter())
        .filter(|version| seen.insert(version.as_str()))
        .cloned()
        .collect()
}

/// The number of pages a list of `count` items fills, at least one.
fn pages_of(count: usize) -> usize {
    count.div_ceil(PER_PAGE).max(1)
}

/// The label of a tab.
fn tab_label(tab: Tab) -> &'static str {
    match tab {
        Tab::Overview => lang::t("resource.tab.overview"),
        Tab::Gallery => lang::t("resource.tab.gallery"),
        Tab::Changelog => lang::t("resource.tab.changelog"),
        Tab::Versions => lang::t("resource.tab.versions"),
    }
}

/// The tab bar: the underlined tabs the page shows, with the active one lit.
fn tab_bar(tabs: &[Tab], active: usize) -> Element<'static, Message> {
    let order = tabs.to_vec();
    container(
        Tabs::new(tabs.iter().map(|tab| tab_label(*tab)))
            .underline()
            .inset(GUTTER)
            .width(Fill)
            .active(active)
            .on_select(move |index| Message::Tab(order[index])),
    )
    .width(Fill)
    .padding(Padding::new(0.0).top(12))
    .into()
}

#[cfg(test)]
#[path = "../../../../tests/app/pages/resource/mod.rs"]
mod tests;
