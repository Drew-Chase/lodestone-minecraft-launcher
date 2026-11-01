use super::*;
use lodestone_platforms::ContentType;

/// A jar a release ships, marked primary or not.
fn jar(name: &str, primary: bool) -> ContentFile {
    ContentFile {
        name: name.to_string(),
        url: format!("https://cdn.example/{name}"),
        sha1: None,
        size: None,
        primary,
    }
}

/// A release for the given loaders and Minecraft versions, shipping the given files.
fn release(id: &str, loaders: &[&str], minecraft: &[&str], files: Vec<ContentFile>) -> ContentVersion {
    ContentVersion {
        id: id.to_string(),
        project_id: Some("sodium".to_string()),
        name: None,
        version_number: id.to_string(),
        content_type: ContentType::Mod,
        minecraft_versions: minecraft.iter().map(|version| version.to_string()).collect(),
        loaders: loaders.iter().map(|loader| loader.to_string()).collect(),
        downloads: 0,
        date: None,
        changelog_url: None,
        files,
        changelog: None,
        channel: None,
        dependencies: Vec::new(),
    }
}

#[test]
fn the_newest_release_for_the_loader_and_the_version_is_the_one_taken() {
    let releases = vec![
        release("forge-1.21.1", &["forge"], &["1.21.1"], vec![jar("a.jar", true)]),
        release("fabric-1.21.1", &["fabric"], &["1.21.1"], vec![jar("b.jar", true)]),
        release("fabric-1.20.1", &["fabric"], &["1.20.1"], vec![jar("c.jar", true)]),
        release("fabric-1.21.1-old", &["fabric"], &["1.21.1"], vec![jar("d.jar", true)]),
    ];
    let chosen = fitting(releases, "fabric", "1.21.1").expect("a release fits");
    assert_eq!(chosen.id, "fabric-1.21.1", "the first listed, which the platform lists newest first");
}

#[test]
fn a_release_for_another_loader_or_version_does_not_fit() {
    let releases = vec![release("neoforge-1.21.1", &["neoforge"], &["1.21.1"], vec![jar("a.jar", true)])];
    assert!(fitting(releases, "fabric", "1.21.1").is_none());
}

#[test]
fn the_primary_jar_is_taken_or_the_first_file_when_none_is_primary() {
    let marked = release("v", &["fabric"], &["1.21.1"], vec![jar("sources.jar", false), jar("mod.jar", true)]);
    assert_eq!(primary(&marked).map(|file| file.name).as_deref(), Some("mod.jar"));
    let unmarked = release("v", &["fabric"], &["1.21.1"], vec![jar("only.jar", false)]);
    assert_eq!(primary(&unmarked).map(|file| file.name).as_deref(), Some("only.jar"));
    let empty = release("v", &["fabric"], &["1.21.1"], Vec::new());
    assert!(primary(&empty).is_none());
}

#[test]
fn only_required_dependencies_that_name_a_project_are_walked() {
    let mut chosen = release("v", &["fabric"], &["1.21.1"], vec![jar("mod.jar", true)]);
    chosen.dependencies = vec![
        VersionDependency {
            kind: DependencyKind::Required,
            project_id: Some("fabric-api".to_string()),
            version_id: None,
        },
        VersionDependency {
            kind: DependencyKind::Optional,
            project_id: Some("modmenu".to_string()),
            version_id: None,
        },
        VersionDependency {
            kind: DependencyKind::Required,
            project_id: None,
            version_id: Some("unnamed".to_string()),
        },
    ];
    let walked: Vec<String> = required(&chosen).into_iter().filter_map(|dependency| dependency.project_id).collect();
    assert_eq!(walked, ["fabric-api"]);
}
