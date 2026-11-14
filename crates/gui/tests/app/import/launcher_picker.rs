//! Tests for `import/launcher_picker.rs`: the launchers the picker lists, the instances the player checks, and the import they make.

use super::*;

/// An instance a launcher listed, which imports when `summary` is `Ok`.
fn listed(launcher: Launcher, source: &str, summary: Result<String, String>) -> crate::app::import::launcher::Instance {
    crate::app::import::launcher::Instance {
        launcher,
        source: source.to_string(),
        name: source.to_string(),
        summary,
    }
}

/// A scan of `launcher` that lists `instances`.
fn scan_of(launcher: Launcher, instances: Vec<crate::app::import::launcher::Instance>) -> Scan {
    Scan {
        launcher,
        folder: Some(PathBuf::from("/games/launcher")),
        instances,
        error: None,
    }
}

#[test]
fn the_look_for_launchers_answers_with_what_each_folder_held() {
    let mut picker = LauncherPicker::default();
    let _ = picker.update(Message::Scan);
    assert!(picker.scanning, "asking for the look starts it");
    let _ = picker.update(Message::Scanned(vec![scan_of(
        Launcher::Prism,
        vec![listed(Launcher::Prism, "/prism/Alpha", Ok("1.21.1 · VANILLA".to_string()))],
    )]));
    assert!(!picker.scanning);
    assert_eq!(picker.scans.len(), 1);
}

#[test]
fn only_instances_that_can_be_imported_can_be_checked_and_import_follows_the_checks() {
    let mut picker = LauncherPicker::default();
    let _ = picker.update(Message::Scanned(vec![scan_of(
        Launcher::Prism,
        vec![
            listed(Launcher::Prism, "/prism/Alpha", Ok("1.21.1 · VANILLA".to_string())),
            listed(Launcher::Prism, "/prism/Broken", Err("no version".to_string())),
        ],
    )]));
    assert!(!picker.has_chosen(), "nothing is checked yet");
    assert!(picker.work().is_none());

    let broken = Pick {
        launcher: Launcher::Prism,
        source: "/prism/Broken".to_string(),
    };
    let _ = picker.update(Message::ToggleInstance(broken));
    assert!(picker.chosen.is_empty(), "an instance that cannot be imported stays unchecked");

    let alpha = Pick {
        launcher: Launcher::Prism,
        source: "/prism/Alpha".to_string(),
    };
    let _ = picker.update(Message::ToggleInstance(alpha.clone()));
    assert!(picker.has_chosen());
    assert!(matches!(picker.work(), Some(Work::Launchers(picks)) if picks == vec![alpha.clone()]));
    let _ = picker.update(Message::ToggleInstance(alpha.clone()));
    assert!(!picker.has_chosen(), "a second press unchecks it");
}

#[test]
fn select_all_checks_every_importable_instance_of_one_launcher_and_clears_them_again() {
    let mut picker = LauncherPicker::default();
    let _ = picker.update(Message::Scanned(vec![scan_of(
        Launcher::AtLauncher,
        vec![
            listed(Launcher::AtLauncher, "/at/One", Ok("1.20.1 · VANILLA".to_string())),
            listed(Launcher::AtLauncher, "/at/Two", Ok("1.21.1 · FABRIC 0.16.10".to_string())),
            listed(Launcher::AtLauncher, "/at/Broken", Err("no version".to_string())),
        ],
    )]));
    let _ = picker.update(Message::ToggleLauncher(Launcher::AtLauncher));
    assert_eq!(picker.chosen_count(), 2);
    let _ = picker.update(Message::ToggleLauncher(Launcher::AtLauncher));
    assert!(!picker.has_chosen());
}

#[test]
fn locating_a_launcher_replaces_its_scan_and_a_failed_locate_shows_its_reason() {
    let mut picker = LauncherPicker::default();
    let _ = picker.update(Message::Scanned(vec![scan_of(Launcher::Modrinth, Vec::new())]));
    let _ = picker.update(Message::ToggleInstance(Pick {
        launcher: Launcher::Modrinth,
        source: "/nowhere".to_string(),
    }));
    let _ = picker.update(Message::Located(
        Launcher::Modrinth,
        Err("No Modrinth app instances were found in that folder.".to_string()),
    ));
    assert_eq!(
        picker.scans[0].error.as_deref(),
        Some("No Modrinth app instances were found in that folder.")
    );
    let found = scan_of(
        Launcher::Modrinth,
        vec![listed(Launcher::Modrinth, "/modrinth/Pack", Ok("26.3 · FABRIC 0.20.1".to_string()))],
    );
    let _ = picker.update(Message::Located(Launcher::Modrinth, Ok(found)));
    assert_eq!(picker.scans[0].instances.len(), 1);
    assert!(!picker.has_chosen(), "a relocated launcher's old checks are dropped");
}

#[test]
fn clearing_the_checks_forgets_what_was_imported() {
    let mut picker = LauncherPicker::default();
    let _ = picker.update(Message::Scanned(vec![scan_of(
        Launcher::Prism,
        vec![listed(Launcher::Prism, "/prism/Alpha", Ok("1.21.1 · VANILLA".to_string()))],
    )]));
    let _ = picker.update(Message::ToggleLauncher(Launcher::Prism));
    assert_eq!(picker.chosen_count(), 1);
    picker.clear_chosen();
    assert!(!picker.has_chosen());
}
