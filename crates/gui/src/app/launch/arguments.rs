//! The JVM and game arguments of one launch: the version's own lists with their rules applied,
//! every placeholder replaced, and the entries the launcher supplies itself taken out.

use std::path::Path;

use super::identity::Identity;
use super::manifest::{Argument, Manifest};
use super::rules::{self, Platform};

/// The values the placeholders in a version's arguments stand for on one launch.
#[derive(Debug, Clone, Copy)]
pub struct Values<'a> {
    pub version_name: &'a str,
    pub version_type: &'a str,
    pub game_directory: &'a Path,
    pub assets_root: &'a Path,
    pub assets_index_name: &'a str,
    pub library_directory: &'a Path,
    pub natives_directory: &'a Path,
    /// The classpath as one string, joined with the platform separator.
    pub classpath: &'a str,
    pub launcher_name: &'a str,
    pub launcher_version: &'a str,
    pub client_id: &'a str,
}

/// The JVM arguments a version lists, ready to pass to the launcher. The launcher adds the
/// classpath and the natives path itself, so the version's own `-cp` pair and
/// `-Djava.library.path` are left out here.
pub fn jvm(manifest: &Manifest, values: &Values, identity: &Identity, platform: Platform) -> Vec<String> {
    let raw = manifest
        .arguments
        .as_ref()
        .map(|arguments| flatten(&arguments.jvm, platform))
        .unwrap_or_default();
    finish(raw, &replacements(values, identity, platform), true)
}

/// The game arguments a version lists. A version from before the arguments list carries one
/// string of them, which is split on spaces.
pub fn game(manifest: &Manifest, values: &Values, identity: &Identity, platform: Platform) -> Vec<String> {
    let raw = match (&manifest.arguments, &manifest.minecraft_arguments) {
        (Some(arguments), _) if !arguments.game.is_empty() => flatten(&arguments.game, platform),
        (_, Some(legacy)) => legacy.split_whitespace().map(str::to_string).collect(),
        _ => Vec::new(),
    };
    finish(raw, &replacements(values, identity, platform), false)
}

/// The logging argument with its config file in place, or `None` when the version has none.
pub fn logging(manifest: &Manifest, config: &Path) -> Option<String> {
    manifest
        .logging
        .as_ref()
        .map(|logging| logging.client.argument.replace("${path}", &config.display().to_string()))
}

/// Every argument that applies on `platform`, in order.
fn flatten(arguments: &[Argument], platform: Platform) -> Vec<String> {
    arguments
        .iter()
        .flat_map(|argument| match argument {
            Argument::Plain(text) => vec![text.clone()],
            Argument::Conditional { rules, value } => {
                if rules::allowed(Some(rules), platform) {
                    value.strings()
                } else {
                    Vec::new()
                }
            }
        })
        .collect()
}

/// Replaces the placeholders in `tokens` and drops what the launcher supplies itself. In JVM
/// arguments that is the classpath pair and the natives path. A token still holding a
/// placeholder after replacement is dropped too, rather than passed to the game as text.
fn finish(tokens: Vec<String>, pairs: &[(&'static str, String)], jvm: bool) -> Vec<String> {
    let mut out = Vec::with_capacity(tokens.len());
    let mut skip_next = false;
    for token in tokens {
        if std::mem::take(&mut skip_next) {
            continue;
        }
        if jvm && (token == "-cp" || token == "-classpath") {
            skip_next = true;
            continue;
        }
        if jvm && token.starts_with("-Djava.library.path=") {
            continue;
        }
        let token = substitute(&token, pairs);
        if !token.contains("${") {
            out.push(token);
        }
    }
    out
}

/// Replaces every known `${name}` in `token`.
fn substitute(token: &str, pairs: &[(&'static str, String)]) -> String {
    pairs
        .iter()
        .fold(token.to_string(), |token, (name, value)| token.replace(&format!("${{{name}}}"), value))
}

/// The placeholder values for one launch, by name.
fn replacements(values: &Values, identity: &Identity, platform: Platform) -> Vec<(&'static str, String)> {
    let token = identity.access_token.as_str();
    vec![
        ("auth_player_name", identity.name.clone()),
        ("auth_uuid", identity.uuid.clone()),
        ("uuid", identity.uuid.clone()),
        ("auth_access_token", token.to_string()),
        ("accessToken", token.to_string()),
        ("auth_session", token.to_string()),
        ("auth_xuid", identity.xuid.clone()),
        ("user_type", identity.user_type.to_string()),
        ("user_properties", "{}".to_string()),
        ("clientid", values.client_id.to_string()),
        ("version_name", values.version_name.to_string()),
        ("version_type", values.version_type.to_string()),
        ("game_directory", values.game_directory.display().to_string()),
        ("assets_root", values.assets_root.display().to_string()),
        ("assets_index_name", values.assets_index_name.to_string()),
        ("library_directory", values.library_directory.display().to_string()),
        ("natives_directory", values.natives_directory.display().to_string()),
        ("classpath", values.classpath.to_string()),
        ("classpath_separator", platform.classpath_separator().to_string()),
        ("launcher_name", values.launcher_name.to_string()),
        ("launcher_version", values.launcher_version.to_string()),
    ]
}

#[cfg(test)]
#[path = "../../../tests/app/launch/arguments.rs"]
mod tests;
