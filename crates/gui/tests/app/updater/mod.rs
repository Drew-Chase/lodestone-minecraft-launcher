use lodestone_updater::Payload;

use super::*;

fn release() -> Release {
    Release {
        tag: "v0.2.0".to_string(),
        html_url: "https://example.test/releases/v0.2.0".to_string(),
        body: String::new(),
        assets: Vec::new(),
    }
}

fn verified() -> Verified {
    Verified {
        release: release(),
        payload: PathBuf::from("Lodestone-v0.2.0-setup.exe"),
    }
}

/// An updater in `status` on a copy that installs updates itself.
fn installing_copy(status: Status) -> Updater {
    Updater {
        status,
        dialog: None,
        etag: None,
        target: Target::Installer,
    }
}

#[test]
fn there_is_no_button_without_an_update() {
    assert_eq!(installing_copy(Status::Idle).button(), None);
    assert_eq!(installing_copy(Status::Failed("offline".to_string())).button(), None);
}

#[test]
fn an_available_update_is_a_button_that_can_be_pressed() {
    let button = installing_copy(Status::Available(release())).button().expect("a button");
    assert!(button.enabled);
}

#[test]
fn a_download_or_an_install_disables_the_button() {
    let downloading = Status::Downloading {
        release: release(),
        install: false,
    };
    assert!(!installing_copy(downloading).button().expect("a button").enabled);
    assert!(!installing_copy(Status::Installing(verified())).button().expect("a button").enabled);
}

#[test]
fn a_downloaded_update_can_be_pressed_while_it_waits_for_the_press() {
    assert!(installing_copy(Status::Ready(verified())).button().expect("a button").enabled);
}

#[test]
fn a_portable_launcher_never_checks_for_updates() {
    assert!(!enabled(true));
    assert_eq!(Target::detect(true), Target::Manual);
}

#[test]
fn the_check_asks_again_only_while_no_update_is_under_way() {
    assert!(installing_copy(Status::Idle).awaiting_check());
    assert!(installing_copy(Status::Available(release())).awaiting_check());
    let downloading = Status::Downloading {
        release: release(),
        install: true,
    };
    assert!(!installing_copy(downloading).awaiting_check());
    assert!(!installing_copy(Status::Ready(verified())).awaiting_check());
    assert!(!installing_copy(Status::Installing(verified())).awaiting_check());
}

#[test]
fn the_changelog_shows_only_for_an_available_release_whose_dialog_is_open() {
    let open = || Some(markdown::Content::parse("## Changes\n\n- A fix"));
    let with_dialog = |status| Updater {
        status,
        dialog: open(),
        etag: None,
        target: Target::Installer,
    };
    assert!(with_dialog(Status::Available(release())).modal().is_some());
    assert!(installing_copy(Status::Available(release())).modal().is_none());
    assert!(with_dialog(Status::Idle).modal().is_none());
}

#[test]
fn a_release_without_notes_still_shows_its_changelog() {
    let updater = Updater {
        dialog: Some(markdown::Content::parse("")),
        status: Status::Available(release()),
        etag: None,
        target: Target::Installer,
    };
    assert!(updater.modal().is_some());
}

#[test]
fn each_copy_takes_the_file_its_system_publishes() {
    assert_eq!(Target::Installer.payload(), Some(Payload::WindowsInstaller));
    assert_eq!(
        Target::Executable(PathBuf::from("lodestone_launcher")).payload(),
        Some(Payload::LinuxArchive)
    );
    assert_eq!(Target::Bundle(PathBuf::from("Lodestone.app")).payload(), Some(Payload::MacosArchive));
    assert_eq!(Target::Manual.payload(), None);
    assert!(!Target::Manual.installs());
}

#[test]
fn a_status_holds_only_the_release_it_is_working_on() {
    let downloading = installing_copy(Status::Downloading {
        release: release(),
        install: true,
    });
    assert!(downloading.holds("v0.2.0"));
    assert!(!downloading.holds("v0.3.0"));
    assert!(installing_copy(Status::Ready(verified())).holds("v0.2.0"));
    assert!(!installing_copy(Status::Available(release())).holds("v0.2.0"));
}
