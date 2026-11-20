//! The Content tab: the instance's mods, its resource packs and shader packs, and its mod snapshots. The switch at its top picks
//! which of them the tab shows. The mods are the mod list, the packs are the pack lists in [`super::packs`], and the snapshots are
//! listed by [`super::snapshots`].

use iced::Element;
use lodestone_platforms::ContentType;

use crate::app::components::segmented;
use crate::app::lang;

/// The sections the Content tab shows, in the order the switch lists them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Section {
    #[default]
    Mods,
    ResourcePacks,
    ShaderPacks,
    /// The mod snapshots the instance keeps. The switch offers it only while there is one.
    Snapshots,
}

impl Section {
    /// Every section, in the order the switch lists them.
    pub const ALL: [Section; 4] = [Section::Mods, Section::ResourcePacks, Section::ShaderPacks, Section::Snapshots];

    /// The switch's label for the section.
    pub fn label(self) -> &'static str {
        match self {
            Section::Mods => lang::t("instance.content.mods"),
            Section::ResourcePacks => lang::t("instance.content.resource_packs"),
            Section::ShaderPacks => lang::t("instance.content.shader_packs"),
            Section::Snapshots => lang::t("instance.content.snapshots"),
        }
    }

    /// The kind of pack a pack section lists, or `None` for the mods section and the snapshots section.
    pub fn pack_kind(self) -> Option<ContentType> {
        match self {
            Section::Mods | Section::Snapshots => None,
            Section::ResourcePacks => Some(ContentType::ResourcePack),
            Section::ShaderPacks => Some(ContentType::Shader),
        }
    }
}

/// Messages of the Content tab's own switch.
#[derive(Debug, Clone)]
pub enum Message {
    /// The switch picked a section.
    Section(Section),
}

/// The switch over the sections, with the active one lit. The snapshots section is offered only when `offer_snapshots` is set.
pub fn switch<'a>(active: Section, offer_snapshots: bool) -> Element<'a, Message> {
    let options: Vec<(Section, &'static str)> = Section::ALL
        .into_iter()
        .filter(|section| offer_snapshots || *section != Section::Snapshots)
        .map(|section| (section, section.label()))
        .collect();
    segmented::segmented(&options, active, Message::Section)
}
