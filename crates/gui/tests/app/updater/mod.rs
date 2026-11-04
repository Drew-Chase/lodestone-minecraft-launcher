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
        installer: PathBuf::from("Lodestone-v0.2.0-setup.exe"),
    }
}

#[test]
fn there_is_no_button_without_an_update() {
    assert_eq!(
        Updater {
            dialog: None,
            status: Status::Idle
        }
        .button(),
        None
    );
    assert_eq!(
        Updater {
            dialog: None,
            status: Status::Failed("offline".to_string())
        }
        .button(),
        None
    );
}

#[test]
fn an_available_update_is_a_button_that_can_be_pressed() {
    let button = Updater {
        dialog: None,
        status: Status::Available(release()),
    }
    .button()
    .expect("a button");
    assert!(button.enabled);
}

#[test]
fn a_download_or_an_install_disables_the_button() {
    assert!(
        !Updater {
            dialog: None,
            status: Status::Downloading(release())
        }
        .button()
        .expect("a button")
        .enabled
    );
    assert!(
        !Updater {
            dialog: None,
            status: Status::Installing(verified())
        }
        .button()
        .expect("a button")
        .enabled
    );
}

#[test]
fn a_verified_update_can_be_pressed_while_it_waits() {
    assert!(
        Updater {
            dialog: None,
            status: Status::Held(verified())
        }
        .button()
        .expect("a button")
        .enabled
    );
    assert!(
        Updater {
            dialog: None,
            status: Status::Ready(verified())
        }
        .button()
        .expect("a button")
        .enabled
    );
}

#[test]
fn a_portable_launcher_never_checks_for_updates() {
    assert!(!enabled(true));
}

#[test]
fn the_check_asks_again_only_while_no_update_is_under_way() {
    assert!(
        Updater {
            dialog: None,
            status: Status::Idle
        }
        .awaiting_check()
    );
    assert!(
        Updater {
            dialog: None,
            status: Status::Available(release())
        }
        .awaiting_check()
    );
    assert!(
        !Updater {
            dialog: None,
            status: Status::Downloading(release())
        }
        .awaiting_check()
    );
    assert!(
        !Updater {
            dialog: None,
            status: Status::Installing(verified())
        }
        .awaiting_check()
    );
}

#[test]
fn the_changelog_shows_only_for_an_available_release_whose_dialog_is_open() {
    let open = || Some(markdown::Content::parse("## Changes\n\n- A fix"));
    assert!(
        Updater {
            dialog: open(),
            status: Status::Available(release())
        }
        .modal()
        .is_some()
    );
    assert!(
        Updater {
            dialog: None,
            status: Status::Available(release())
        }
        .modal()
        .is_none()
    );
    assert!(
        Updater {
            dialog: open(),
            status: Status::Idle
        }
        .modal()
        .is_none()
    );
}

#[test]
fn a_release_without_notes_still_shows_its_changelog() {
    let mut release = release();
    release.body = String::new();
    let updater = Updater {
        dialog: Some(markdown::Content::parse("")),
        status: Status::Available(release),
    };
    assert!(updater.modal().is_some());
}
