//! The language catalogues: every piece of UI text, looked up by key.
//!
//! Each `res/lang/<code>.toml` is embedded at build time (see `build.rs`) and
//! parsed on first use. Keys are dotted (`login.title`), and the file's
//! `langname` names the language in the picker. A key missing from the active
//! language falls back to `en_us`, and a key missing from both shows as itself,
//! so a gap is visible rather than blank.
//!
//! The active language is one atomic index. `update` sets it when the user picks
//! a language and `view` reads it, so [`t`] works anywhere without threading
//! state through every view.
//!
//! Placeholders are `{NAME}`, filled by [`t_with`]. Counts use a `<stem>.one` and
//! `<stem>.other` pair, picked by [`t_count`]. [`rich_segments`] honours the `%P`
//! and `%C` markup (see [`markup`]); [`t`] returns the plain text only.

mod markup;

use std::collections::HashMap;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};

pub use markup::Segment;

include!(concat!(env!("OUT_DIR"), "/languages.rs"));

/// The fallback language. `build.rs` sorts it first, so it is `languages()[0]`.
pub const DEFAULT: &str = "en_us";

/// The index into [`languages`] of the active language.
static ACTIVE: AtomicUsize = AtomicUsize::new(0);

/// One loaded catalogue.
pub struct Language {
    /// The file stem, such as `en_us`. Preferences store this.
    pub code: &'static str,
    /// The language's own name, from `langname`.
    pub name: String,
    entries: HashMap<String, Entry>,
}

/// One translated string, kept in two forms.
struct Entry {
    /// The text with its markup resolved. [`t`] returns this.
    plain: String,
    /// The text as written. [`rich_segments`] parses this when it renders.
    marked: String,
}

/// Every loaded catalogue, `en_us` first. A file that fails to parse is skipped
/// with a warning, except `en_us`, which the app cannot run without.
pub fn languages() -> &'static [Language] {
    static LANGUAGES: OnceLock<Vec<Language>> = OnceLock::new();
    LANGUAGES.get_or_init(|| {
        LANGUAGE_FILES
            .iter()
            .filter_map(|&(code, source)| match Language::parse(code, source) {
                Ok(language) => Some(language),
                Err(error) if code == DEFAULT => panic!("res/lang/{code}.toml must load: {error}"),
                Err(error) => {
                    tracing::warn!("Skipping res/lang/{code}.toml: {error}");
                    None
                }
            })
            .collect()
    })
}

/// Makes `code` the active language. An unknown code leaves the current one in
/// place and returns `false`.
pub fn set_active(code: &str) -> bool {
    match languages().iter().position(|language| language.code == code) {
        Some(index) => {
            ACTIVE.store(index, Ordering::Relaxed);
            true
        }
        None => false,
    }
}

/// The index into [`languages`] of the active language, for the picker's selected row.
pub fn active_index() -> usize {
    ACTIVE.load(Ordering::Relaxed)
}

/// The text for `key` in the active language, with markup removed.
pub fn t(key: &'static str) -> &'static str {
    entry(key).map_or(key, |entry| entry.plain.as_str())
}

/// [`t`] with each `{NAME}` replaced by its value.
pub fn t_with(key: &'static str, args: &[(&str, &str)]) -> String {
    args.iter()
        .fold(t(key).to_owned(), |text, &(name, value)| text.replace(&format!("{{{name}}}"), value))
}

/// The count's form of `stem`: `<stem>.one` for exactly one, `<stem>.other`
/// otherwise, with `{count}` filled in.
pub fn t_count(stem: &str, count: usize) -> String {
    let form = if count == 1 { "one" } else { "other" };
    let key = format!("{stem}.{form}");
    let text: &str = match entry(&key) {
        Some(entry) => &entry.plain,
        None => &key,
    };
    text.replace("{count}", &count.to_string())
}

/// The runs of `key` with their `%P` and `%C` markup, for rich rendering. A key
/// missing from every language comes back as one plain run.
pub fn rich_segments(key: &'static str) -> Vec<Segment<'static>> {
    match entry(key) {
        Some(entry) => markup::segments(&entry.marked),
        None => vec![Segment { text: key, primary: false }],
    }
}

/// The entry for `key` in the active language, falling back to `en_us`.
fn entry(key: &str) -> Option<&'static Entry> {
    let languages = languages();
    let active = languages.get(active_index()).unwrap_or(&languages[0]);
    resolve(active, &languages[0], key)
}

/// The entry for `key`: `active` first, then `fallback`.
fn resolve<'a>(active: &'a Language, fallback: &'a Language, key: &str) -> Option<&'a Entry> {
    active.entries.get(key).or_else(|| fallback.entries.get(key))
}

impl Language {
    fn parse(code: &'static str, source: &str) -> Result<Self, String> {
        let table: toml::Table = source.parse().map_err(|error: toml::de::Error| error.to_string())?;
        let name = table
            .get("langname")
            .and_then(toml::Value::as_str)
            .ok_or_else(|| String::from("langname is missing"))?
            .to_owned();
        let mut entries = HashMap::new();
        for (key, value) in &table {
            if key != "langname" {
                flatten(key, value, &mut entries);
            }
        }
        Ok(Self { code, name, entries })
    }
}

/// Walks nested tables into dotted keys and keeps each text value.
fn flatten(path: &str, value: &toml::Value, out: &mut HashMap<String, Entry>) {
    match value {
        toml::Value::String(raw) => {
            out.insert(path.to_owned(), Entry::new(raw));
        }
        toml::Value::Table(table) => {
            for (key, inner) in table {
                flatten(&format!("{path}.{key}"), inner, out);
            }
        }
        other => tracing::warn!("Ignoring {path} in the catalogue: expected text, found {other}"),
    }
}

impl Entry {
    fn new(raw: &str) -> Self {
        Self {
            plain: markup::segments(raw).iter().map(|segment| segment.text).collect(),
            marked: raw.to_owned(),
        }
    }
}

#[cfg(test)]
#[path = "../../../tests/app/lang/mod.rs"]
mod tests;
