//! The files a version needs on disk: every library the rules allow, the native archives to
//! extract, and the jars the classpath lists.

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use super::manifest::{Library, Manifest};
use super::rules::{self, Platform};

/// The host Mojang's libraries download from when a library names no repository of its own.
const LIBRARIES_HOST: &str = "https://libraries.minecraft.net/";

/// One file to have on disk. Without a `url` it can only be checked, not fetched: a loader
/// installer writes those itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub url: Option<String>,
    pub path: PathBuf,
    pub sha1: Option<String>,
    pub size: Option<u64>,
}

/// A native archive to extract, and the path prefixes it leaves out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Native {
    pub archive: PathBuf,
    pub exclude: Vec<String>,
}

/// What a version's libraries need on this platform.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Plan {
    /// Every file, native archives among them, each path once.
    pub items: Vec<Item>,
    /// The jars the classpath lists, in order. Native archives are not among them.
    pub classpath: Vec<PathBuf>,
    /// The native archives to extract into the natives directory before launch.
    pub natives: Vec<Native>,
}

/// Plans the libraries of `manifest` for `platform`, with paths under `libraries_dir`.
pub fn plan(manifest: &Manifest, libraries_dir: &Path, platform: Platform) -> Plan {
    let mut plan = Plan::default();
    let mut on_classpath = HashSet::new();
    for library in &manifest.libraries {
        if !rules::allowed(library.rules.as_deref(), platform) {
            continue;
        }
        match &library.natives {
            Some(natives) => plan_native(library, natives, libraries_dir, platform, &mut plan),
            None => plan_jar(library, libraries_dir, &mut on_classpath, &mut plan),
        }
    }
    let mut seen = HashSet::new();
    plan.items.retain(|item| seen.insert(item.path.clone()));
    plan
}

/// A native library: its archive is downloaded and extracted, and it stays off the classpath.
fn plan_native(library: &Library, natives: &BTreeMap<String, String>, dir: &Path, platform: Platform, plan: &mut Plan) {
    let Some(template) = natives.get(platform.os) else {
        return;
    };
    let classifier = template.replace("${arch}", platform.pointer_width());
    let Some(item) = item_for(library, Some(&classifier), dir) else {
        return;
    };
    let exclude = library.extract.as_ref().map(|extract| extract.exclude.clone()).unwrap_or_default();
    plan.natives.push(Native {
        archive: item.path.clone(),
        exclude,
    });
    plan.items.push(item);
}

/// An ordinary library: its jar goes on the classpath once, however many entries name it.
fn plan_jar(library: &Library, dir: &Path, on_classpath: &mut HashSet<PathBuf>, plan: &mut Plan) {
    let Some(item) = item_for(library, None, dir) else {
        return;
    };
    if on_classpath.insert(item.path.clone()) {
        plan.classpath.push(item.path.clone());
    }
    plan.items.push(item);
}

/// The file a library names. Its declared download is used when it has one. Otherwise the
/// file is the Maven path of its coordinates, fetched from the library's repository.
fn item_for(library: &Library, classifier: Option<&str>, dir: &Path) -> Option<Item> {
    let declared = library.downloads.as_ref().and_then(|downloads| match classifier {
        Some(key) => downloads.classifiers.as_ref().and_then(|map| map.get(key)),
        None => downloads.artifact.as_ref(),
    });
    if let Some(file) = declared
        && let Some(relative) = file.path.as_deref()
    {
        let url = if file.url.is_empty() {
            hosted(library, relative)
        } else {
            Some(file.url.clone())
        };
        return Some(Item {
            url,
            path: dir.join(relative),
            sha1: file.sha1.clone(),
            size: file.size,
        });
    }
    let relative = maven_path(&library.name, classifier)?;
    Some(Item {
        url: hosted(library, &relative),
        path: dir.join(&relative),
        sha1: None,
        size: None,
    })
}

/// The URL a library's `relative` path downloads from. A library with its own repository
/// uses it. A Mojang library with none falls back to Mojang's host. A loader library with none
/// has no URL, so it must already be on disk.
fn hosted(library: &Library, relative: &str) -> Option<String> {
    let host = library
        .url
        .as_deref()
        .filter(|url| !url.is_empty())
        .or_else(|| library.downloads.as_ref().map(|_| LIBRARIES_HOST))?;
    Some(format!("{}/{relative}", host.trim_end_matches('/')))
}

/// The repository path of a Maven coordinate, `group:artifact:version[:classifier][@extension]`,
/// with `classifier` standing in for the coordinate's own when given.
pub fn maven_path(name: &str, classifier: Option<&str>) -> Option<String> {
    let (coordinate, extension) = name.split_once('@').unwrap_or((name, "jar"));
    let mut parts = coordinate.split(':');
    let (group, artifact, version) = (parts.next()?, parts.next()?, parts.next()?);
    let classifier = classifier.or(parts.next()).filter(|classifier| !classifier.is_empty());
    let file = match classifier {
        Some(classifier) => format!("{artifact}-{version}-{classifier}.{extension}"),
        None => format!("{artifact}-{version}.{extension}"),
    };
    Some(format!("{}/{artifact}/{version}/{file}", group.replace('.', "/")))
}

#[cfg(test)]
#[path = "../../../tests/app/launch/libraries.rs"]
mod tests;
