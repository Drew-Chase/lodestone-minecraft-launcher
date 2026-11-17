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

/// A fresh folder under the system temp folder, named for the test.
fn scratch(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("lodestone-install-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("scratch folder");
    path
}

/// Writes `contents` to `path`, creating the folders above it.
fn write(path: &Path, contents: &[u8]) {
    std::fs::create_dir_all(path.parent().expect("a file has a parent")).expect("parent folder");
    std::fs::write(path, contents).expect("file writes");
}

#[test]
fn a_file_name_held_switched_on_or_off_is_held_by_name() {
    let root = scratch("held-by-name");
    write(&root.join("fabric-api.jar"), b"");
    write(&root.join("sodium.jar.disabled"), b"");
    assert!(held_by_name(&root, "fabric-api.jar"));
    assert!(held_by_name(&root, "sodium.jar"), "a switched-off copy is held too");
    assert!(!held_by_name(&root, "lithium.jar"));
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn another_installed_version_is_a_conflict_and_a_matching_or_unknown_one_is_not() {
    assert_eq!(
        conflicting_version(&[Some("0.100.0")], Some("0.116.1")).as_deref(),
        Some("0.100.0"),
        "a copy at another version is a dependency version conflict"
    );
    assert_eq!(
        conflicting_version(&[Some("0.116.1")], Some("0.116.1")),
        None,
        "the same version is kept unasked"
    );
    assert_eq!(
        conflicting_version(&[Some("0.100.0"), Some("0.116.1")], Some("0.116.1")),
        None,
        "a copy at the offered version is already there"
    );
    assert_eq!(
        conflicting_version(&[None], Some("0.116.1")),
        None,
        "an installed copy with no version cannot be compared"
    );
    assert_eq!(
        conflicting_version(&[Some("0.100.0")], None),
        None,
        "an offered jar with no version cannot be compared"
    );
}

#[test]
fn a_conflict_takes_only_its_first_answer() {
    let (reply, mut answer) = tokio::sync::oneshot::channel();
    let conflict = VersionConflict {
        dependency: "Fabric API".to_string(),
        adding: "Sodium".to_string(),
        installed: "0.100.0".to_string(),
        offered: "0.116.1".to_string(),
        reply: Arc::new(Mutex::new(Some(reply))),
    };
    conflict.clone().answer(true);
    conflict.answer(false);
    assert_eq!(answer.try_recv(), Ok(true), "only the first answer counts");
}

#[test]
fn a_move_that_fails_takes_back_the_jars_already_moved() {
    let root = scratch("move-back");
    let staged = root.join("staged").join("alpha.jar");
    let target = root.join("mods").join("alpha.jar");
    write(&staged, b"alpha");
    let moves = [
        (staged, target.clone()),
        (root.join("staged").join("beta.jar"), root.join("mods").join("beta.jar")),
    ];
    assert!(move_into(&moves).is_err(), "the second jar is not staged");
    assert!(!target.exists(), "the jar moved before the failure is removed again");
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn a_staging_folder_is_held_while_it_lives_and_removed_when_it_ends() {
    let root = scratch("staging");
    let staging = Staging::within(&root).expect("a staging folder");
    write(&staging.dir(0).join("mod.jar"), b"jar");
    assert!(staging_in_use(), "a live staging folder keeps Clear cache from its root");
    drop(staging);
    assert!(!staging_in_use());
    assert_eq!(std::fs::read_dir(&root).expect("the root stays").count(), 0, "the add's folder is gone");
    std::fs::remove_dir_all(&root).ok();
}
