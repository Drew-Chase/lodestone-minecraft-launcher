use super::*;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// The active language is process-wide, so tests that switch it hold this lock
/// and put the default back when they finish.
static ACTIVE_LANGUAGE: Mutex<()> = Mutex::new(());

/// Runs `body` with `code` active, then restores the default.
fn with_language<R>(code: &str, body: impl FnOnce() -> R) -> R {
    let _guard = ACTIVE_LANGUAGE.lock().unwrap_or_else(|poison| poison.into_inner());
    assert!(set_active(code), "{code} is bundled");
    let result = body();
    set_active(DEFAULT);
    result
}

/// Every `.rs` file under `dir`, found recursively.
fn rust_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("reading the source directory") {
        let path = entry.expect("reading a directory entry").path();
        if path.is_dir() {
            rust_sources(&path, out);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            out.push(path);
        }
    }
}

/// Every `t("…")`, `t_with("…")`, `t_count("…")` and `rich_segments("…")` call in
/// `source`, as `(call, key)`.
fn literal_keys(source: &str) -> Vec<(&'static str, &str)> {
    const CALLS: [&str; 4] = ["t", "t_with", "t_count", "rich_segments"];
    let mut found = Vec::new();
    for call in CALLS {
        let marker = format!("{call}(\"");
        for (index, _) in source.match_indices(&marker) {
            // Skip a longer name that ends in the same letters, such as `format("`.
            if source[..index].chars().next_back().is_some_and(|c| c.is_alphanumeric() || c == '_') {
                continue;
            }
            let rest = &source[index + marker.len()..];
            if let Some(end) = rest.find('"') {
                found.push((call, &rest[..end]));
            }
        }
    }
    found
}

#[test]
fn en_us_leads_the_list_and_names_itself() {
    let default = &languages()[0];
    assert_eq!(default.code, DEFAULT);
    assert_eq!(default.name, "English");
}

#[test]
fn every_bundled_catalogue_parses() {
    for &(code, source) in LANGUAGE_FILES {
        if let Err(error) = Language::parse(code, source) {
            panic!("res/lang/{code}.toml does not parse: {error}");
        }
    }
}

#[test]
fn other_catalogues_hold_only_keys_en_us_knows() {
    let default = &languages()[0];
    for language in languages() {
        for key in language.entries.keys() {
            assert!(default.entries.contains_key(key), "{} has {key}, which en_us lacks", language.code);
        }
    }
}

#[test]
fn every_literal_key_in_the_source_exists_in_en_us() {
    let default = &languages()[0];
    let mut files = Vec::new();
    rust_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"), &mut files);
    assert!(!files.is_empty(), "the source tree is found");
    let mut found = 0;
    for file in files {
        let source = std::fs::read_to_string(&file).expect("reading a source file");
        for (call, key) in literal_keys(&source) {
            found += 1;
            let keys = if call == "t_count" {
                vec![format!("{key}.one"), format!("{key}.other")]
            } else {
                vec![key.to_owned()]
            };
            for key in keys {
                assert!(default.entries.contains_key(&key), "{} uses {key}, which en_us lacks", file.display());
            }
        }
    }
    // A scanner that matched nothing would pass every check above, so it must find real call sites.
    assert!(found > 100, "the scan finds the call sites (found {found})");
}

#[test]
fn a_key_missing_from_the_active_language_falls_back_to_en_us() {
    let default = &languages()[0];
    let active = Language {
        code: "xx_xx",
        name: String::from("Test"),
        entries: HashMap::from([(String::from("only.here"), Entry::new("Here"))]),
    };
    assert_eq!(resolve(&active, default, "only.here").map(|entry| entry.plain.as_str()), Some("Here"));
    assert_eq!(resolve(&active, default, "login.or").map(|entry| entry.plain.as_str()), Some("OR"));
    assert!(resolve(&active, default, "no.such.key").is_none());
}

#[test]
fn a_missing_key_shows_as_itself() {
    assert_eq!(t("no.such.key"), "no.such.key");
}

#[test]
fn plain_lookups_drop_the_markup() {
    with_language(DEFAULT, || assert_eq!(t("login.title"), "Welcome back, crafter"));
}

#[test]
fn rich_lookups_keep_the_primary_run() {
    with_language(DEFAULT, || {
        let runs: Vec<(&str, bool)> = rich_segments("login.title")
            .iter()
            .map(|segment| (segment.text, segment.primary))
            .collect();
        assert_eq!(runs, vec![("Welcome back, ", false), ("crafter", true)]);
    });
}

#[test]
fn placeholders_are_filled_in() {
    with_language(DEFAULT, || {
        assert_eq!(t_with("titlebar.tag", &[("VERSION", "1.2.3")]), "LODESTONE - v1.2.3")
    });
}

#[test]
fn an_unknown_language_is_refused_and_the_current_one_kept() {
    with_language(DEFAULT, || {
        assert!(!set_active("xx_xx"));
        assert_eq!(active_index(), 0);
    });
}
