//! The Mods tab's dialogs, each a modal on top of the tab: linking a mod to the project it is, switching the mod to another
//! release, and listing its dependencies. A dialog holds what its view needs. The root fetches what it shows and carries
//! out what its buttons ask for, and the answers come back into the dialog.

use std::collections::HashMap;

use aurora_leaf::Icon;
use iced::widget::{Space, Text, column, container, image, row, text_input};
use iced::{Element, Fill, Padding, alignment};
use iced_color_extension::rgb;
use lodestone_platforms::{ContentVersion, DependencyKind, SearchResult};

use crate::app::components::badge::Badge;
use crate::app::components::button::Variant;
use crate::app::components::modal::Modal;
use crate::app::components::{TextButton, input};
use crate::app::lang;
use crate::app::pages::discover::Source;
use crate::app::pages::resource::Load;
use crate::app::theme::{color, fonts, utility};

/// The width of a dialog's card.
const WIDTH: f32 = 620.0;

/// A dialog on top of the Mods tab.
#[derive(Debug)]
pub enum Dialog {
    Associate(Box<Associate>),
    Switch(Switch),
    Dependencies(Dependencies),
}

/// A dialog's messages. The tab handles the local ones, and the root carries out the rest.
#[derive(Debug, Clone)]
pub enum Message {
    /// Closes the dialog.
    Close,
    /// The search text changed.
    Query(String),
    /// A release of the picked project was chosen, by index.
    Choose(usize),
    /// Back from the picked project to the search results.
    Back,
    /// Searches the platform being searched for the text.
    Search,
    /// Searches another platform for the text.
    Platform(Source),
    /// A search result was picked, by index.
    Pick(usize),
    /// Links the mod to the picked project and its chosen release.
    Confirm,
    /// Installs the release at this index in place of the mod's current one.
    Use(usize),
    /// Opens the resource page of the dependency at this index.
    Open(usize),
}

impl Message {
    /// Whether the tab handles the message itself, without the root.
    pub fn is_local(&self) -> bool {
        matches!(self, Message::Close | Message::Query(_) | Message::Choose(_) | Message::Back)
    }
}

/// Linking a mod to a project: a search of a platform, then the releases of the project picked from its hits.
#[derive(Debug)]
pub struct Associate {
    /// The jar the link is for.
    pub file_name: String,
    /// The text searched for.
    pub query: String,
    /// The platform searched.
    pub source: Source,
    /// The hits for the text on `source`.
    pub hits: Load<Vec<SearchResult>>,
    /// The project picked from the hits, with its releases once they load.
    pub picked: Option<Picked>,
    /// The icon of each hit's project, by the url it has, as the pictures load.
    pub icons: HashMap<String, image::Handle>,
}

/// The project picked for a link, with its releases and the one chosen. `jar_sha1` is the SHA-1 of the jar the link is for,
/// which the link records, once the jar could be read.
#[derive(Debug)]
pub struct Picked {
    pub project: SearchResult,
    pub releases: Load<Vec<ContentVersion>>,
    pub chosen: Option<usize>,
    pub jar_sha1: Option<String>,
}

impl Associate {
    /// A search for `query` on `source`, for the jar `file_name`, with nothing to show until the hits load.
    pub fn new(file_name: String, query: String, source: Source) -> Self {
        Self {
            file_name,
            query,
            source,
            hits: Load::Loading,
            picked: None,
            icons: HashMap::new(),
        }
    }

    /// The release the link is for: the one chosen from the picked project, once its releases have loaded.
    pub fn chosen_release(&self) -> Option<&ContentVersion> {
        let picked = self.picked.as_ref()?;
        let Load::Loaded(releases) = &picked.releases else {
            return None;
        };
        releases.get(picked.chosen?)
    }
}

/// Switching a mod to another release that fits the instance.
#[derive(Debug)]
pub struct Switch {
    /// The jar being switched.
    pub file_name: String,
    /// The mod's name.
    pub name: String,
    /// The releases that fit the instance, newest first, once they load.
    pub releases: Load<Vec<ContentVersion>>,
    /// The release the jar is, so its row can say so.
    pub installed: Option<String>,
}

/// The dependencies of a mod's current release, with whether the instance has each one.
#[derive(Debug)]
pub struct Dependencies {
    /// The jar the mod is.
    pub file_name: String,
    /// The platform the mod came from, which its dependencies are read from and opened on.
    pub source: Source,
    /// The mod's name.
    pub name: String,
    /// The dependencies, once they load.
    pub lines: Load<Vec<Line>>,
}

/// One dependency: its project, its name, how the release relates to it, and the jar the instance has for it.
#[derive(Debug, Clone)]
pub struct Line {
    pub project_id: String,
    pub name: String,
    pub kind: DependencyKind,
    pub installed: Option<String>,
}

impl Dialog {
    /// Applies a local message to the dialog.
    pub fn local(&mut self, message: &Message) {
        match (self, message) {
            (Dialog::Associate(associate), Message::Query(query)) => associate.query = query.clone(),
            (Dialog::Associate(associate), Message::Choose(index)) => {
                if let Some(picked) = associate.picked.as_mut() {
                    picked.chosen = Some(*index);
                }
            }
            (Dialog::Associate(associate), Message::Back) => associate.picked = None,
            _ => {}
        }
    }

    /// The dialog's card.
    pub fn view(&self) -> Element<'_, Message> {
        match self {
            Dialog::Associate(associate) => associate_view(associate),
            Dialog::Switch(switch) => switch_view(switch),
            Dialog::Dependencies(dependencies) => dependencies_view(dependencies),
        }
    }
}

fn associate_view(dialog: &Associate) -> Element<'_, Message> {
    let (body, ready) = match &dialog.picked {
        Some(picked) => (picked_body(picked), dialog.chosen_release().is_some()),
        None => (results_body(dialog), false),
    };
    let confirm = TextButton::primary(lang::t("instance.mods.associate.confirm"));
    let confirm = if ready {
        confirm.on_press(Message::Confirm)
    } else {
        confirm.disabled(true)
    };
    let footer = row![Space::new().width(Fill), cancel_button(), confirm]
        .spacing(12)
        .align_y(alignment::Vertical::Center);
    Modal::new(
        lang::t_with("instance.mods.associate.title", &[("name", dialog.file_name.as_str())]),
        body,
    )
    .subtitle(lang::t("instance.mods.associate.subtitle"))
    .icon(Icon::lucide().link(), color::Primary::DEFAULT, color::Primary::TINT)
    .footer(footer)
    .on_close(Message::Close)
    .close_on_scrim(true)
    .width(WIDTH)
    .into()
}

/// The search: the platform switch, the text field, and the hits.
fn results_body(dialog: &Associate) -> Element<'_, Message> {
    let platforms = row![
        platform_button(Source::Modrinth, dialog.source),
        platform_button(Source::CurseForge, dialog.source),
    ]
    .spacing(8);
    let search = text_input(lang::t("instance.mods.associate.search_placeholder"), &dialog.query)
        .on_input(Message::Query)
        .on_submit(Message::Search)
        .style(input::style)
        .padding(Padding::new(0.0).left(14).right(14).top(10).bottom(10))
        .size(13)
        .width(Fill);
    let hits: Element<'_, Message> = match &dialog.hits {
        Load::Loading => muted(lang::t("instance.mods.associate.searching")),
        Load::Failed(error) => muted(lang::t_with("instance.mods.associate.failed", &[("error", error.as_str())])),
        Load::Loaded(hits) if hits.is_empty() => muted(lang::t("instance.mods.associate.no_results")),
        Load::Loaded(hits) => column(
            hits.iter()
                .enumerate()
                .map(|(index, hit)| hit_row(index, hit, hit.icon_url.as_ref().and_then(|url| dialog.icons.get(url))))
                .collect::<Vec<_>>(),
        )
        .spacing(6)
        .width(Fill)
        .into(),
    };
    column![platforms, search, hits].spacing(12).width(Fill).into()
}

/// The picked project's releases, with the one chosen marked, and a way back to the search.
fn picked_body(picked: &Picked) -> Element<'_, Message> {
    let back = TextButton::ghost(lang::t("instance.mods.associate.back")).on_press(Message::Back);
    let header = row![
        back,
        Text::new(picked.project.name.as_str())
            .size(14)
            .font(fonts::Inter::SEMI_BOLD)
            .color(color::Text::HEADLINE),
    ]
    .spacing(12)
    .align_y(alignment::Vertical::Center);
    let releases: Element<'_, Message> = match &picked.releases {
        Load::Loading => muted(lang::t("instance.mods.associate.loading_releases")),
        Load::Failed(error) => muted(lang::t_with("instance.mods.associate.failed", &[("error", error.as_str())])),
        Load::Loaded(releases) if releases.is_empty() => muted(lang::t("instance.mods.associate.no_releases")),
        Load::Loaded(releases) => column(
            releases
                .iter()
                .enumerate()
                .map(|(index, release)| release_choice(index, release, picked.chosen == Some(index)))
                .collect::<Vec<_>>(),
        )
        .spacing(6)
        .width(Fill)
        .into(),
    };
    column![header, releases].spacing(12).width(Fill).into()
}

fn switch_view(dialog: &Switch) -> Element<'_, Message> {
    let body: Element<'_, Message> = match &dialog.releases {
        Load::Loading => muted(lang::t("instance.mods.switch.loading")),
        Load::Failed(error) => muted(lang::t_with("instance.mods.switch.failed", &[("error", error.as_str())])),
        Load::Loaded(releases) if releases.is_empty() => muted(lang::t("instance.mods.switch.empty")),
        Load::Loaded(releases) => column(
            releases
                .iter()
                .enumerate()
                .map(|(index, release)| switch_choice(index, release, dialog.installed.as_deref() == Some(release.id.as_str())))
                .collect::<Vec<_>>(),
        )
        .spacing(6)
        .width(Fill)
        .into(),
    };
    Modal::new(lang::t_with("instance.mods.switch.title", &[("name", dialog.name.as_str())]), body)
        .subtitle(lang::t("instance.mods.switch.subtitle"))
        .icon(Icon::lucide().repeat(), color::Primary::DEFAULT, color::Primary::TINT)
        .footer(close_footer())
        .on_close(Message::Close)
        .close_on_scrim(true)
        .width(WIDTH)
        .into()
}

fn dependencies_view(dialog: &Dependencies) -> Element<'_, Message> {
    let body: Element<'_, Message> = match &dialog.lines {
        Load::Loading => muted(lang::t("instance.mods.deps.loading")),
        Load::Failed(error) => muted(lang::t_with("instance.mods.deps.failed", &[("error", error.as_str())])),
        Load::Loaded(lines) if lines.is_empty() => muted(lang::t("instance.mods.deps.empty")),
        Load::Loaded(lines) => column(
            lines
                .iter()
                .enumerate()
                .map(|(index, line)| dependency_line(index, line))
                .collect::<Vec<_>>(),
        )
        .spacing(6)
        .width(Fill)
        .into(),
    };
    Modal::new(lang::t_with("instance.mods.deps.title", &[("name", dialog.name.as_str())]), body)
        .subtitle(lang::t("instance.mods.deps.subtitle"))
        .icon(Icon::lucide().layers(), color::Primary::DEFAULT, color::Primary::TINT)
        .footer(close_footer())
        .on_close(Message::Close)
        .close_on_scrim(true)
        .width(WIDTH)
        .into()
}

/// A platform's button in the search. The platform being searched is filled.
fn platform_button(source: Source, selected: Source) -> Element<'static, Message> {
    let button = if source == selected {
        TextButton::primary(source.name())
    } else {
        TextButton::ghost(source.name())
    };
    button.on_press(Message::Platform(source)).into()
}

/// One search hit: its icon, its name, its description and its author. A press picks it.
fn hit_row<'a>(index: usize, hit: &'a SearchResult, icon: Option<&'a image::Handle>) -> Element<'a, Message> {
    let text = column![
        Text::new(hit.name.as_str())
            .size(13)
            .font(fonts::Inter::SEMI_BOLD)
            .color(color::Text::HEADLINE),
        Text::new(hit.description.as_str())
            .size(12)
            .font(fonts::Inter::REGULAR)
            .color(color::Text::MUTED),
    ]
    .spacing(2)
    .width(Fill);
    let author = Text::new(hit.author.clone().unwrap_or_default())
        .size(11)
        .font(fonts::JetBrainsMono::REGULAR)
        .color(color::Text::PLACEHOLDER);
    let content = row![hit_icon(icon), text, author].spacing(14).align_y(alignment::Vertical::Center);
    TextButton::content(content)
        .variant(Variant::Ghost)
        .width(Fill)
        .on_press(Message::Pick(index))
        .into()
}

/// A hit's icon, or the package glyph while the picture loads and when the platform has none.
fn hit_icon(icon: Option<&image::Handle>) -> Element<'_, Message> {
    match icon {
        Some(handle) => image(handle.clone()).width(32).height(32).into(),
        None => container(utility::icon(
            Icon::lucide().package(),
            15.0,
            Some((color::Text::MUTED, color::Text::MUTED)),
        ))
        .width(32)
        .height(32)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .into(),
    }
}

/// One release of the picked project. The chosen one is filled.
fn release_choice(index: usize, release: &ContentVersion, chosen: bool) -> Element<'_, Message> {
    let variant = if chosen { Variant::Primary } else { Variant::Ghost };
    let label = release.name.clone().unwrap_or_else(|| release.version_number.clone());
    let versions = release.minecraft_versions.join(", ");
    let content = row![
        Text::new(label).size(13).font(fonts::Inter::SEMI_BOLD).color(color::Text::HEADLINE),
        Space::new().width(Fill),
        Text::new(versions)
            .size(11)
            .font(fonts::JetBrainsMono::REGULAR)
            .color(color::Text::PLACEHOLDER),
    ]
    .spacing(14)
    .align_y(alignment::Vertical::Center);
    TextButton::content(content)
        .variant(variant)
        .width(Fill)
        .on_press(Message::Choose(index))
        .into()
}

/// One release a mod can switch to. The release the jar is shows as installed, and cannot be picked again.
fn switch_choice(index: usize, release: &ContentVersion, installed: bool) -> Element<'_, Message> {
    let label = release.name.clone().unwrap_or_else(|| release.version_number.clone());
    let mut content = row![
        Text::new(label).size(13).font(fonts::Inter::SEMI_BOLD).color(color::Text::HEADLINE),
        Space::new().width(Fill),
    ]
    .spacing(14)
    .align_y(alignment::Vertical::Center);
    if installed {
        content = content.push(Badge::new(lang::t("instance.mods.switch.installed"), color::Primary::DEFAULT, color::Primary::TINT).compact());
    }
    let button = TextButton::content(content).variant(Variant::Ghost).width(Fill).disabled(installed);
    if installed {
        button.into()
    } else {
        button.on_press(Message::Use(index)).into()
    }
}

/// One dependency: its name, how the release relates to it, whether the instance has it, and a way to its page.
fn dependency_line(index: usize, line: &Line) -> Element<'_, Message> {
    let (ink, fill) = match line.kind {
        DependencyKind::Incompatible => (rgb!(255, 110, 110, 100%), rgb!(255, 110, 110, 12%)),
        _ => (color::Text::MUTED, rgb!(255, 255, 255, 6%)),
    };
    let mut content = row![
        Text::new(line.name.as_str())
            .size(13)
            .font(fonts::Inter::SEMI_BOLD)
            .color(color::Text::HEADLINE),
        Badge::new(kind_label(line.kind), ink, fill).compact(),
        Space::new().width(Fill),
    ]
    .spacing(10)
    .align_y(alignment::Vertical::Center);
    content = content.push(match line.installed {
        Some(_) => Badge::new(lang::t("instance.mods.deps.installed"), color::Primary::DEFAULT, color::Primary::TINT).compact(),
        None => Badge::new(lang::t("instance.mods.deps.missing"), color::Text::MUTED, rgb!(255, 255, 255, 6%)).compact(),
    });
    let open = TextButton::ghost(lang::t("instance.mods.deps.open")).on_press(Message::Open(index));
    row![content, open].spacing(12).align_y(alignment::Vertical::Center).width(Fill).into()
}

/// The label a dependency's kind shows as.
fn kind_label(kind: DependencyKind) -> &'static str {
    match kind {
        DependencyKind::Required => lang::t("instance.mods.deps.required"),
        DependencyKind::Optional => lang::t("instance.mods.deps.optional"),
        DependencyKind::Incompatible => lang::t("instance.mods.deps.incompatible"),
        DependencyKind::Embedded => lang::t("instance.mods.deps.embedded"),
    }
}

fn cancel_button() -> Element<'static, Message> {
    TextButton::ghost(lang::t("common.cancel")).on_press(Message::Close).into()
}

fn close_footer() -> Element<'static, Message> {
    row![
        Space::new().width(Fill),
        TextButton::ghost(lang::t("common.close")).on_press(Message::Close),
    ]
    .into()
}

/// A line of muted text that owns its words, for the states a list shows while it waits or is empty.
fn muted(text: impl Into<String>) -> Element<'static, Message> {
    Text::new(text.into())
        .size(12)
        .font(fonts::Inter::REGULAR)
        .color(color::Text::MUTED)
        .into()
}

#[cfg(test)]
#[path = "../../../../../tests/app/pages/instance/mods/dialog.rs"]
mod tests;
