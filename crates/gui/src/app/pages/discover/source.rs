//! The five platforms the Discover page searches, and what each one offers.
//! The page always has exactly one platform selected: there is no "all".

use crate::app::components::platform_logo::Logo;
use lodestone_platforms::{AtLauncher, ContentType, CurseForge, Ftb, Modrinth, PlatformKind, SearchSort, Technic};

/// A content platform the page can search.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Source {
    Modrinth,
    CurseForge,
    AtLauncher,
    Technic,
    Ftb,
}

impl Source {
    /// The sources, in the order the selector shows them.
    pub const ALL: [Source; 5] = [Source::Modrinth, Source::CurseForge, Source::AtLauncher, Source::Technic, Source::Ftb];

    /// The name the platform goes by. Brand names are not translated.
    pub fn name(self) -> &'static str {
        match self {
            Source::Modrinth => "Modrinth",
            Source::CurseForge => "CurseForge",
            Source::AtLauncher => "ATLauncher",
            Source::Technic => "Technic",
            Source::Ftb => "FTB",
        }
    }

    /// The key the preferences store the selection under.
    pub fn key(self) -> &'static str {
        match self {
            Source::Modrinth => "modrinth",
            Source::CurseForge => "curseforge",
            Source::AtLauncher => "atlauncher",
            Source::Technic => "technic",
            Source::Ftb => "ftb",
        }
    }

    /// The platform a stored key names, if any.
    pub fn from_key(key: &str) -> Option<Source> {
        Source::ALL.into_iter().find(|source| source.key() == key)
    }

    /// The platform's mark.
    pub fn logo(self) -> Logo {
        match self {
            Source::Modrinth => Logo::Modrinth,
            Source::CurseForge => Logo::CurseForge,
            Source::AtLauncher => Logo::AtLauncher,
            Source::Technic => Logo::Technic,
            Source::Ftb => Logo::Ftb,
        }
    }

    /// The content types the platform hosts, straight from its capability matrix.
    pub fn content_types(self) -> &'static [ContentType] {
        match self {
            Source::Modrinth => Modrinth::CONTENT_TYPES,
            Source::CurseForge => CurseForge::CONTENT_TYPES,
            Source::AtLauncher => AtLauncher::CONTENT_TYPES,
            Source::Technic => Technic::CONTENT_TYPES,
            Source::Ftb => Ftb::CONTENT_TYPES,
        }
    }

    /// Whether the platform hosts `content`.
    pub fn supports(self, content: ContentType) -> bool {
        self.content_types().contains(&content)
    }

    /// The sort orders the platform can express. The first one is the default.
    pub fn sorts(self) -> &'static [SearchSort] {
        match self {
            Source::Modrinth | Source::CurseForge => &[SearchSort::Relevance, SearchSort::Popular, SearchSort::Updated, SearchSort::Newest],
            Source::Ftb => &[SearchSort::Popular],
            Source::AtLauncher | Source::Technic => &[SearchSort::Relevance],
        }
    }

    /// Whether the platform offers category, loader and Minecraft version filters.
    pub fn has_filters(self) -> bool {
        matches!(self, Source::Modrinth | Source::CurseForge)
    }

    /// Whether the platform can be searched in this build. CurseForge needs its key.
    pub fn enabled(self) -> bool {
        match self {
            Source::CurseForge => crate::app::discover::curseforge_key().is_some(),
            Source::Modrinth | Source::AtLauncher | Source::Technic | Source::Ftb => true,
        }
    }

    /// The platform's home page, for the cases where it has nothing to show.
    pub fn home_url(self) -> &'static str {
        match self {
            Source::Modrinth => "https://modrinth.com",
            Source::CurseForge => "https://www.curseforge.com/minecraft",
            Source::AtLauncher => "https://atlauncher.com",
            Source::Technic => "https://www.technicpack.net",
            Source::Ftb => "https://www.feed-the-beast.com",
        }
    }
}

#[cfg(test)]
#[path = "../../../../tests/app/pages/discover/source.rs"]
mod tests;
