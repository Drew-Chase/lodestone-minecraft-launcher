//! The Content tab's resource packs and shader packs: the packs in the instance's `resourcepacks/` or `shaderpacks/` folder as
//! one filtered list, with a way to add packs from the shared folders and one to find them on Discover. The folders are read
//! by `files`. The root carries out the copies and removals, and the tab shows what comes back.

mod shared;

use std::collections::HashMap;

use aurora_leaf::Icon;
use iced::widget::{Text, column, container, row, text_input};
use iced::{Element, Fill, Padding, alignment};
use lodestone_platforms::ContentType;

use super::GUTTER;
use super::files::PackFile;
use super::format::format_bytes;
use crate::app::components::button::Variant;
use crate::app::components::hint::hint;
use crate::app::components::icon_button::ghost_button_style;
use crate::app::components::panel::panel;
use crate::app::components::platform_logo;
use crate::app::components::{IconButton, TextButton, input, scrollbar};
use crate::app::lang;
use crate::app::pages::discover::Source;
use crate::app::shared_library::{self, AddedContent, SharedEntry};
use crate::app::theme::{color, fonts};

pub use shared::Dialog;

/// Messages the packs emit, or receive from the root.
#[derive(Debug, Clone)]
pub enum Message {
    /// The filter text changed.
    Query(String),
    /// Add from shared files was pressed for a kind of pack. The root reads the shared folder for the dialog.
    OpenShared(ContentType),
    /// The shared folder of a kind was read: each pack in it, and whether the instance has it already. The root sends it.
    SharedLoaded { kind: ContentType, entries: Vec<(SharedEntry, bool)> },
    /// The dialog was closed.
    CloseShared,
    /// Add was pressed for the pack at `index` of the dialog's list. The root copies it into the instance.
    Add { kind: ContentType, index: usize },
    /// Delete was pressed for the shared pack at `index` of the dialog's list. The root asks first, then moves it to the trash.
    DeleteShared { kind: ContentType, index: usize },
    /// Find on Discover was pressed for a kind of pack. The root opens Discover on it, for this instance.
    Find(ContentType),
    /// Show in file browser was pressed for the pack with this file name.
    Reveal { kind: ContentType, file_name: String },
    /// Remove was pressed for the pack with this file name. The root asks first, then moves it to the trash.
    Remove { kind: ContentType, file_name: String },
    /// The folders were read again, after a pack was added or removed, or the page came back to the instance. The root
    /// sends it with the generation of the page it was read for.
    Rescanned {
        generation: u64,
        resource: Vec<PackFile>,
        shader: Vec<PackFile>,
    },
    /// The platforms the packs were taken from, as the database holds them for the shown instance, or the error that stopped
    /// the read. The root sends it with the generation of the page it was read for.
    Sources {
        generation: u64,
        result: Result<Vec<AddedContent>, String>,
    },
}

/// The packs the Content tab lists, the platform each was taken from, the filter they are listed under, and the dialog that
/// adds packs.
#[derive(Debug, Default)]
pub struct State {
    resource: Vec<PackFile>,
    shader: Vec<PackFile>,
    /// The platform each resource pack was taken from, by file name. A pack with no entry was not taken from a platform.
    resource_sources: HashMap<String, Source>,
    /// The platform each shader pack was taken from, by file name.
    shader_sources: HashMap<String, Source>,
    query: String,
    dialog: Option<Dialog>,
}

impl State {
    /// Replaces the packs with the ones the folders hold.
    pub fn set_packs(&mut self, resource: Vec<PackFile>, shader: Vec<PackFile>) {
        self.resource = resource;
        self.shader = shader;
    }

    /// Keeps the platform each pack in the instance's own folders was taken from. A record of another scope, such as a
    /// world's datapacks, is not one of these packs.
    fn set_sources(&mut self, records: &[AddedContent]) {
        self.resource_sources.clear();
        self.shader_sources.clear();
        for record in records.iter().filter(|record| record.scope.is_empty()) {
            let Some(source) = Source::from_key(&record.platform) else {
                continue;
            };
            if record.kind == shared_library::folder_name(ContentType::ResourcePack) {
                self.resource_sources.insert(record.file_name.clone(), source);
            } else if record.kind == shared_library::folder_name(ContentType::Shader) {
                self.shader_sources.insert(record.file_name.clone(), source);
            }
        }
    }

    /// The platform the pack `file_name` of `kind` was taken from, when a record names one.
    fn source_of(&self, kind: ContentType, file_name: &str) -> Option<Source> {
        let sources = match kind {
            ContentType::ResourcePack => &self.resource_sources,
            _ => &self.shader_sources,
        };
        sources.get(file_name).copied()
    }

    /// Carries out the messages the tab keeps for itself: the filter, the dialog's list and its closing, the packs the
    /// folders hold, and the platforms they were taken from. The rest are the root's.
    pub fn update(&mut self, message: &Message) {
        match message {
            Message::Query(query) => self.query.clone_from(query),
            Message::SharedLoaded { kind, entries } => {
                self.dialog = Some(Dialog {
                    kind: *kind,
                    entries: entries.clone(),
                });
            }
            Message::CloseShared => self.dialog = None,
            Message::Rescanned { resource, shader, .. } => self.set_packs(resource.clone(), shader.clone()),
            // A failed read keeps the platforms the packs had, rather than dropping every chip for one bad read.
            Message::Sources { result, .. } => {
                if let Ok(records) = result {
                    self.set_sources(records);
                }
            }
            Message::OpenShared(_)
            | Message::Add { .. }
            | Message::DeleteShared { .. }
            | Message::Find(_)
            | Message::Reveal { .. }
            | Message::Remove { .. } => {}
        }
    }

    /// The packs of a kind, in the order the folder lists them.
    fn packs(&self, kind: ContentType) -> &[PackFile] {
        match kind {
            ContentType::ResourcePack => &self.resource,
            _ => &self.shader,
        }
    }

    /// The dialog that adds packs, when one is open.
    pub fn dialog(&self) -> Option<&Dialog> {
        self.dialog.as_ref()
    }

    /// The dialog over the tab, when one is open.
    pub fn dialog_view(&self) -> Option<Element<'_, Message>> {
        self.dialog.as_ref().map(shared::view)
    }

    /// The packs of `kind` under the filter, with the toolbar that adds packs.
    pub fn view(&self, kind: ContentType) -> Element<'_, Message> {
        let toolbar = row![
            text_input(lang::t("instance.packs.filter_placeholder"), &self.query)
                .on_input(Message::Query)
                .style(input::style)
                .padding(Padding::new(0.0).left(14).right(14).top(10).bottom(10))
                .size(13)
                .width(Fill),
            TextButton::labeled_icon(Icon::lucide().download(), lang::t("instance.packs.from_shared"), Variant::Ghost)
                .on_press(Message::OpenShared(kind)),
            hint(
                TextButton::labeled_icon(Icon::lucide().search(), lang::t("instance.packs.find"), Variant::Primary).on_press(Message::Find(kind)),
                lang::t("instance.packs.find_hint"),
            ),
        ]
        .spacing(10)
        .align_y(alignment::Vertical::Center);

        let packs = self.packs(kind);
        let needle = self.query.to_lowercase();
        let shown: Vec<&PackFile> = packs.iter().filter(|pack| pack.name.to_lowercase().contains(&needle)).collect();
        let list: Element<'_, Message> = if shown.is_empty() {
            let text = if packs.is_empty() {
                lang::t("instance.packs.empty")
            } else {
                lang::t("instance.packs.no_match")
            };
            panel(Text::new(text).size(12).font(fonts::Inter::REGULAR).color(color::text_muted()))
        } else {
            let mut rows = column![].spacing(6).width(Fill);
            for pack in shown {
                rows = rows.push(pack_row(kind, pack, self.source_of(kind, &pack.file_name)));
            }
            panel(rows)
        };
        let scrolled = scrollbar::scroll(
            container(list)
                .width(Fill)
                .padding(Padding::new(0.0).left(GUTTER).right(GUTTER).bottom(40.0)),
        )
        .width(Fill)
        .height(Fill);
        // The toolbar and the list keep the page's gutters, as the mod list does, so the list's scrollbar sits in the gutter.
        let toolbar = container(toolbar).width(Fill).padding(Padding::new(0.0).left(GUTTER).right(GUTTER));
        column![toolbar, scrolled].spacing(18).width(Fill).height(Fill).into()
    }
}

/// One pack: its name, with the platform it was taken from as a chip beside it when a record names one, its size, and buttons
/// that show it in the file browser and move it to the trash.
fn pack_row<'a>(kind: ContentType, pack: &PackFile, source: Option<Source>) -> Element<'a, Message> {
    let size = match pack.size {
        Some(bytes) => format_bytes(bytes),
        None => lang::t("instance.packs.folder").to_string(),
    };
    let mut title = row![
        Text::new(pack.name.clone())
            .size(13)
            .font(fonts::Inter::MEDIUM)
            .color(color::text_default()),
    ]
    .spacing(8)
    .align_y(alignment::Vertical::Center);
    if let Some(source) = source {
        title = title.push(platform_logo::chip(source.logo()));
    }
    let details = column![
        title,
        Text::new(size).size(11).font(fonts::JetBrainsMono::REGULAR).color(color::text_muted()),
    ]
    .spacing(2)
    .width(Fill);
    let reveal = hint(
        IconButton::new(Icon::lucide().folder())
            .size(32.0)
            .radius(8.0)
            .icon_size(14.0)
            .style(ghost_button_style())
            .on_press(Message::Reveal {
                kind,
                file_name: pack.file_name.clone(),
            }),
        lang::t("instance.packs.reveal_hint"),
    );
    let remove = hint(
        IconButton::new(Icon::lucide().trash())
            .size(32.0)
            .radius(8.0)
            .icon_size(14.0)
            .style(ghost_button_style())
            .on_press(Message::Remove {
                kind,
                file_name: pack.file_name.clone(),
            }),
        lang::t("instance.packs.remove_hint"),
    );
    row![details, reveal, remove]
        .spacing(8)
        .align_y(alignment::Vertical::Center)
        .width(Fill)
        .into()
}

#[cfg(test)]
#[path = "../../../../../tests/app/pages/instance/packs/mod.rs"]
mod tests;
