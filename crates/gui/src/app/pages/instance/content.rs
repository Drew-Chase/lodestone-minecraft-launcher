//! The Content tab: the instance's mods, and its resource packs and shader packs. The switch at its top picks which of the three
//! the tab shows. The mods are the mod list, and the packs are the pack lists in [`super::packs`].

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
}

impl Section {
    /// Every section, in the order the switch lists them.
    pub const ALL: [Section; 3] = [Section::Mods, Section::ResourcePacks, Section::ShaderPacks];

    /// The switch's label for the section.
    pub fn label(self) -> &'static str {
        match self {
            Section::Mods => lang::t("instance.content.mods"),
            Section::ResourcePacks => lang::t("instance.content.resource_packs"),
            Section::ShaderPacks => lang::t("instance.content.shader_packs"),
        }
    }

    /// The kind of pack a pack section lists, or `None` for the mods section.
    pub fn pack_kind(self) -> Option<ContentType> {
        match self {
            Section::Mods => None,
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

/// The switch over the sections, with the active one lit.
pub fn switch<'a>(active: Section) -> Element<'a, Message> {
    let options = Section::ALL.map(|section| (section, section.label()));
    segmented::segmented(&options, active, Message::Section)
}
