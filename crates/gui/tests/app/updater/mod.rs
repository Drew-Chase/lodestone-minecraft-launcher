use super::*;

fn release() -> Release {
    Release {
        tag: "v0.2.0".to_string(),
        html_url: "https://example.test/releases/v0.2.0".to_string(),
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
    assert_eq!(Updater { status: Status::Idle }.button(), None);
    assert_eq!(
        Updater {
            status: Status::Failed("offline".to_string())
        }
        .button(),
        None
    );
}

#[test]
fn an_available_update_is_a_button_that_can_be_pressed() {
    let button = Updater {
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
            status: Status::Downloading(release())
        }
        .button()
        .expect("a button")
        .enabled
    );
    assert!(
        !Updater {
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
            status: Status::Held(verified())
        }
        .button()
        .expect("a button")
        .enabled
    );
    assert!(
        Updater {
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
