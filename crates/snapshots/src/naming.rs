//! The file names of snapshots: when each was taken, its kind, and a counter when another snapshot took the same second.

use chrono::NaiveDateTime;

use crate::model::Kind;

/// The layout of the time in a name. Every field has a fixed width, so names sort by time as text does, and no colon
/// appears in them, since Windows does not allow one.
pub const STAMP: &str = "%Y-%m-%d_%H-%M-%S";
/// The length of a formatted time, such as `2026-11-20_14-05-33`.
const STAMP_LEN: usize = 19;
/// The extension of a snapshot's JSON file, without its leading dot.
pub const JSON_EXTENSION: &str = "json.xz";
/// The extension of a snapshot's zip of local jars, without its leading dot.
pub const ZIP_EXTENSION: &str = "zip";

/// A snapshot's name without its extension: `2026-11-20_14-05-33_update-all`. The second snapshot taken in the same second
/// gets `-2` after its kind, the third `-3`, and so on.
pub fn stem(when: NaiveDateTime, kind: Kind, copy: u32) -> String {
    let base = format!("{}_{}", when.format(STAMP), kind.slug());
    if copy <= 1 { base } else { format!("{base}-{copy}") }
}

/// A snapshot's name, as its file name gives it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Name {
    /// When the snapshot was taken, to the second.
    pub when: NaiveDateTime,
    /// The copy within the second: 1 for the first snapshot taken then, 2 for the next, and so on.
    pub copy: u32,
    /// Why the snapshot was taken.
    pub kind: Kind,
}

impl Name {
    /// The stem this name formats to, without its extension.
    pub fn stem(&self) -> String {
        stem(self.when, self.kind, self.copy)
    }
}

/// The name a stem gives, or `None` when the stem is not a snapshot's.
pub fn parse_stem(stem: &str) -> Option<Name> {
    let when = NaiveDateTime::parse_from_str(stem.get(..STAMP_LEN)?, STAMP).ok()?;
    let rest = stem.get(STAMP_LEN..)?.strip_prefix('_')?;
    Kind::ALL.into_iter().find_map(|kind| {
        let slug = kind.slug();
        if rest == slug {
            Some(Name { when, copy: 1, kind })
        } else {
            let copy = rest.strip_prefix(slug)?.strip_prefix('-')?.parse::<u32>().ok()?;
            (copy >= 2).then_some(Name { when, copy, kind })
        }
    })
}

/// The name of a snapshot's file, which ends in `.json.xz` or `.zip`. `None` for any other file, including a `.partial` one.
pub fn parse_file(file_name: &str) -> Option<Name> {
    let stem = file_name
        .strip_suffix(JSON_EXTENSION)
        .and_then(|rest| rest.strip_suffix('.'))
        .or_else(|| file_name.strip_suffix(ZIP_EXTENSION).and_then(|rest| rest.strip_suffix('.')))?;
    parse_stem(stem)
}

#[cfg(test)]
#[path = "../tests/naming.rs"]
mod tests;
