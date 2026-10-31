use super::*;
use crate::app::install::catalog::VersionInfo;

/// A dialog whose folders live in a scratch directory, named after the test.
fn dialog(name: &str) -> NewInstance {
    let root = std::env::temp_dir().join(format!("lodestone-new-instance-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    NewInstance::new(LauncherPaths::new(root))
}

/// A dialog with a name, a Minecraft version and its Java runtime chosen.
fn filled(name: &str) -> NewInstance {
    let mut dialog = dialog(name);
    dialog.name = "  Dragon Survival v2 ".to_string();
    dialog.mc_version = Some("1.21.4".to_string());
    dialog.version_info = Some(VersionInfo {
        component: "java-runtime-delta".to_string(),
        major: Some(21),
    });
    dialog
}

#[test]
fn a_new_dialog_needs_a_name_before_it_can_install() {
    let dialog = dialog("needs-name");
    assert!(!dialog.ready());
    assert_eq!(dialog.request(&[]).expect_err("no name yet"), "Give the instance a name.");
}

#[test]
fn a_vanilla_form_builds_a_request_with_a_cleaned_folder_name() {
    let mut dialog = filled("vanilla-request");
    dialog.loader = Loader::Vanilla;
    assert!(dialog.ready());

    let request = dialog.request(&[]).expect("the request builds");
    assert_eq!(request.name, "Dragon Survival v2");
    assert_eq!(request.directory_name, "Dragon Survival v2");
    assert_eq!(request.java_component.as_deref(), Some("java-runtime-delta"));
    assert!(request.loader_version.is_none());
}

#[test]
fn a_modded_form_needs_a_loader_version() {
    let mut dialog = filled("modded-request");
    dialog.loader = Loader::Fabric;
    assert!(!dialog.ready());
    assert_eq!(dialog.request(&[]).expect_err("no loader version yet"), "Choose a loader version.");

    dialog.loader_version = Some("0.16.10".to_string());
    assert!(dialog.ready());
    assert_eq!(
        dialog.request(&[]).expect("the request builds").loader_version.as_deref(),
        Some("0.16.10")
    );
}

#[test]
fn a_java_override_replaces_the_version_runtime() {
    let mut dialog = filled("java-override");
    dialog.loader = Loader::Vanilla;
    dialog.java_override = Some("java-runtime-gamma".to_string());
    assert_eq!(
        dialog.request(&[]).expect("the request builds").java_component.as_deref(),
        Some("java-runtime-gamma")
    );
}

#[test]
fn an_uploaded_banner_keeps_a_scene_as_its_fallback() {
    let mut dialog = filled("uploaded-banner");
    dialog.loader = Loader::Vanilla;
    dialog.banner = BannerChoice::Uploaded(Upload::new(vec![1, 2, 3]));

    let request = dialog.request(&[]).expect("the request builds");
    assert_eq!(request.banner, Some(vec![1, 2, 3]));
    assert_eq!((request.biome, request.art_seed), banner::scene(0));
}
