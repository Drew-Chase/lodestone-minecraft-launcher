//! Reads Mojang's version JSON and a loader's profile, and folds the profile over the vanilla
//! version it inherits from. Every field is optional here, because a loader profile omits
//! most of what the vanilla version lists.

use std::collections::{BTreeMap, HashSet};

use serde::Deserialize;

use super::rules::Rule;

/// One version JSON, vanilla or loader, as far as the launcher reads it.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub main_class: Option<String>,
    #[serde(default)]
    pub arguments: Option<Arguments>,
    #[serde(default)]
    pub minecraft_arguments: Option<String>,
    #[serde(default)]
    pub libraries: Vec<Library>,
    #[serde(default)]
    pub downloads: Option<Downloads>,
    #[serde(default)]
    pub asset_index: Option<AssetIndex>,
    #[serde(default)]
    pub assets: Option<String>,
    #[serde(default)]
    pub logging: Option<Logging>,
    #[serde(default, rename = "type")]
    pub kind: Option<String>,
    #[serde(default)]
    pub java_version: Option<JavaVersion>,
}

/// The game and JVM argument lists a version gives.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Arguments {
    #[serde(default)]
    pub game: Vec<Argument>,
    #[serde(default)]
    pub jvm: Vec<Argument>,
}

/// One argument: a plain string, or a value that applies only when its rules allow it.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum Argument {
    Plain(String),
    Conditional { rules: Vec<Rule>, value: Value },
}

/// The value of a conditional argument: one string, or several.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum Value {
    One(String),
    Many(Vec<String>),
}

impl Value {
    /// The strings this value stands for, in order.
    pub fn strings(&self) -> Vec<String> {
        match self {
            Value::One(text) => vec![text.clone()],
            Value::Many(texts) => texts.clone(),
        }
    }
}

/// A library a version needs, with the rules that say on which platforms.
#[derive(Debug, Clone, Deserialize)]
pub struct Library {
    /// The Maven coordinates, such as `org.lwjgl:lwjgl:3.3.3`.
    pub name: String,
    /// The repository a loader library is fetched from. Mojang's libraries declare their
    /// downloads instead.
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub downloads: Option<LibraryDownloads>,
    #[serde(default)]
    pub rules: Option<Vec<Rule>>,
    /// Maps an operating system to the classifier of its native archive, when the library is
    /// a native archive to extract.
    #[serde(default)]
    pub natives: Option<BTreeMap<String, String>>,
    #[serde(default)]
    pub extract: Option<Extract>,
}

/// The files a library declares: its main artifact, and native classifiers.
#[derive(Debug, Clone, Deserialize)]
pub struct LibraryDownloads {
    #[serde(default)]
    pub artifact: Option<File>,
    #[serde(default)]
    pub classifiers: Option<BTreeMap<String, File>>,
}

/// What to leave out when a native archive is extracted.
#[derive(Debug, Clone, Deserialize)]
pub struct Extract {
    #[serde(default)]
    pub exclude: Vec<String>,
}

/// A file a version downloads: where it goes, where it comes from and how to check it.
#[derive(Debug, Clone, Deserialize)]
pub struct File {
    /// The path under the library directory. The client jar declares none.
    #[serde(default)]
    pub path: Option<String>,
    /// Empty when the file has no URL of its own.
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub sha1: Option<String>,
    #[serde(default)]
    pub size: Option<u64>,
}

/// The files a version downloads itself, which is only the client jar.
#[derive(Debug, Clone, Deserialize)]
pub struct Downloads {
    #[serde(default)]
    pub client: Option<File>,
}

/// The asset index a version names.
#[derive(Debug, Clone, Deserialize)]
pub struct AssetIndex {
    pub id: String,
    pub url: String,
}

/// The logging setup the game starts with.
#[derive(Debug, Clone, Deserialize)]
pub struct Logging {
    pub client: LoggingClient,
}

/// The logging argument, with `${path}` standing for the config file, and the config file.
#[derive(Debug, Clone, Deserialize)]
pub struct LoggingClient {
    pub argument: String,
    pub file: LogFile,
}

/// The log4j config a version names.
#[derive(Debug, Clone, Deserialize)]
pub struct LogFile {
    pub id: String,
    pub url: String,
    #[serde(default)]
    pub sha1: Option<String>,
    #[serde(default)]
    pub size: Option<u64>,
}

/// The Java runtime a version asks for.
#[derive(Debug, Clone, Deserialize)]
pub struct JavaVersion {
    pub component: String,
}

impl Manifest {
    /// Reads one version JSON.
    pub fn parse(text: &str) -> Result<Self, String> {
        serde_json::from_str(text).map_err(|error| error.to_string())
    }

    /// Folds this loader profile over the vanilla `parent` it inherits from. The profile's own
    /// entries win where one is set, and its libraries come first, so one it overrides wins.
    pub fn over(self, parent: Manifest) -> Manifest {
        let arguments = match (self.arguments, parent.arguments) {
            (None, None) => None,
            (child, parent) => {
                let (child, parent) = (child.unwrap_or_default(), parent.unwrap_or_default());
                Some(Arguments {
                    game: parent.game.into_iter().chain(child.game).collect(),
                    jvm: parent.jvm.into_iter().chain(child.jvm).collect(),
                })
            }
        };
        Manifest {
            id: self.id,
            main_class: self.main_class.or(parent.main_class),
            arguments,
            minecraft_arguments: self.minecraft_arguments.or(parent.minecraft_arguments),
            libraries: unique_libraries(self.libraries.into_iter().chain(parent.libraries)),
            downloads: self.downloads.or(parent.downloads),
            asset_index: self.asset_index.or(parent.asset_index),
            assets: self.assets.or(parent.assets),
            logging: self.logging.or(parent.logging),
            kind: self.kind.or(parent.kind),
            java_version: self.java_version.or(parent.java_version),
        }
    }
}

/// Keeps the first library of each `group:artifact[:classifier]`, so a loader's version of a
/// library replaces the vanilla one.
fn unique_libraries(libraries: impl Iterator<Item = Library>) -> Vec<Library> {
    let mut seen = HashSet::new();
    libraries.filter(|library| seen.insert(library_key(&library.name))).collect()
}

/// A library's identity without its version: `group:artifact`, plus `:classifier` when it has one.
pub fn library_key(name: &str) -> String {
    let parts: Vec<&str> = name.split(':').collect();
    match parts.as_slice() {
        [group, artifact, _version, classifier, ..] => format!("{group}:{artifact}:{classifier}"),
        [group, artifact, ..] => format!("{group}:{artifact}"),
        _ => name.to_string(),
    }
}

#[cfg(test)]
#[path = "../../../tests/app/launch/manifest.rs"]
mod tests;
