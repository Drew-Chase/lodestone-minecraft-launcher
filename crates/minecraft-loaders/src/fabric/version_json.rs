use anyhow::Result;
use piston_mc::manifest_v2::ReleaseType;
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Deserialize, Debug)]
pub struct VersionJson {
    pub id: String,
    pub time: chrono::DateTime<chrono::offset::Utc>,
    #[serde(rename = "type")]
    pub release_type: ReleaseType,
    #[serde(rename = "inheritsFrom")]
    pub minecraft_version: String,
    #[serde(rename = "releaseTime")]
    pub release_time: chrono::DateTime<chrono::offset::Utc>,
    #[serde(rename = "mainClass")]
    pub main_class: String,
    pub libraries: Vec<LibraryItem>,
    pub arguments: Arguments,
}

#[derive(Deserialize, Debug)]
pub struct LibraryItem {
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub sha1: Option<String>,
    #[serde(default)]
    pub sha256: Option<String>,
    #[serde(default)]
    pub sha512: Option<String>,
    #[serde(default)]
    pub md5: Option<String>,
    #[serde(default)]
    pub size: Option<usize>,
}
#[derive(Deserialize, Debug)]
pub struct Arguments {
    pub jvm: Vec<String>,
    pub game: Vec<String>,
}

impl VersionJson {
    pub fn load<S: AsRef<str>, P: AsRef<Path>>(
        install_path: P,
        loader_version: S,
        mc_version: S,
    ) -> Result<Self> {
        let loader_version = loader_version.as_ref();
        let mc_version = mc_version.as_ref();
        let name = format!("fabric-loader-{loader_version}-{mc_version}");
        let path = install_path
            .as_ref()
            .join("versions")
            .join(&name)
            .join(format!("{name}.json"));
        let content = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&content)?)
    }

    pub fn get_library_files(&self, install_path: impl AsRef<Path>) -> Vec<PathBuf> {
        let install_path = install_path.as_ref();
        let libraries_path = install_path.join("libraries");
        self.libraries
            .iter()
            .map(|l| libraries_path.join(Self::path_from_maven(l.name.as_ref())))
            .collect::<Vec<_>>()
    }

    fn path_from_maven(maven: &str) -> String {
        let parts: Vec<&str> = maven.split(':').collect();
        if parts.len() != 3 {
            return maven.to_string();
        }

        let group = parts[0].replace('.', "/");
        let artifact = parts[1];
        let version = parts[2];

        format!(
            "{}/{}/{}/{}-{}.jar",
            group, artifact, version, artifact, version
        )
    }
}
