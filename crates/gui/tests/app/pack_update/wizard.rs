use super::*;
use crate::app::install::modpack::Payload;
use crate::app::install::pack::Release;
use crate::app::install::update::merge::Side;
use crate::app::install::update::plan::{Change, Decision, Kind};
use crate::app::install::update::{Decisions, UpdateRequest};
use crate::app::pages::library::instance::{Loader, fixture};
use crate::app::paths::LauncherPaths;

/// An update of an instance named `w`, onto release 2.0, with no files of its own.
fn request() -> UpdateRequest {
    UpdateRequest {
        paths: LauncherPaths::new(std::env::temp_dir().join("lodestone-wizard-test")),
        instance: fixture("w", "Pack", Loader::Fabric),
        release: Release {
            id: "v2".to_string(),
            number: "2.0".to_string(),
        },
        mc_version: "1.20.1".to_string(),
        loader: Loader::Fabric,
        loader_version: None,
        payload: Payload {
            files: Vec::new(),
            unpacks: Vec::new(),
            sources: Vec::new(),
            packs: Vec::new(),
        },
        backup_worlds: false,
        decisions: Decisions::default(),
        record_only: false,
        reviewed: None,
    }
}

/// A change to `path` of `kind`.
fn change(path: &str, kind: Kind) -> Change {
    Change {
        path: path.to_string(),
        kind,
    }
}

/// A wizard on a review of `changes`.
fn wizard(changes: Vec<Change>) -> PackWizard {
    PackWizard::new(Review {
        request: request(),
        changes,
        conflicts: Vec::new(),
    })
}

#[test]
fn the_steps_move_forward_and_back_and_stop_at_either_end() {
    let mut wizard = wizard(Vec::new());
    assert_eq!(wizard.step(), Step::Summary);
    wizard.update(Message::Back);
    assert_eq!(wizard.step(), Step::Summary);
    wizard.update(Message::Next);
    assert_eq!(wizard.step(), Step::Changes);
    wizard.update(Message::Next);
    assert_eq!(wizard.step(), Step::Confirm);
    wizard.update(Message::Next);
    assert_eq!(wizard.step(), Step::Confirm);
    wizard.update(Message::Back);
    assert_eq!(wizard.step(), Step::Changes);
}

#[test]
fn the_counts_follow_the_decisions_and_leave_the_users_own_jars_out() {
    let mut wizard = wizard(vec![
        change("mods/a.jar", Kind::Add),
        change("mods/b.jar", Kind::Drop),
        change("mods/mine.jar", Kind::Yours),
    ]);
    assert_eq!(wizard.counts(), (2, 0), "a new jar is applied and a dropped mod is removed by default");
    wizard.update(Message::Decide {
        path: "mods/b.jar".to_string(),
        decision: Decision::Keep,
    });
    assert_eq!(wizard.counts(), (1, 1));
}

#[test]
fn applying_carries_the_decisions_the_backup_choice_and_the_reviewed_changes_into_the_request() {
    let mut wizard = wizard(vec![change("config/a.toml", Kind::Mine)]);
    wizard.update(Message::Decide {
        path: "config/a.toml".to_string(),
        decision: Decision::Take,
    });
    wizard.update(Message::SetBackup(true));

    let request = wizard.into_request();

    assert_eq!(request.decisions.decide(&change("config/a.toml", Kind::Mine)), Decision::Take);
    assert!(request.backup_worlds);
    assert_eq!(
        request.reviewed.map(|reviewed| reviewed.changes),
        Some(vec![change("config/a.toml", Kind::Mine)])
    );
}

#[test]
fn a_hunk_takes_the_side_its_choice_names_and_the_users_side_otherwise() {
    let mut wizard = wizard(Vec::new());
    assert_eq!(wizard.decisions().side("config/a.toml", 0), Side::Yours);
    wizard.update(Message::Choose {
        path: "config/a.toml".to_string(),
        index: 1,
        side: Side::Theirs,
    });
    assert_eq!(wizard.decisions().side("config/a.toml", 1), Side::Theirs);
    assert_eq!(wizard.decisions().side("config/a.toml", 0), Side::Yours);
}
