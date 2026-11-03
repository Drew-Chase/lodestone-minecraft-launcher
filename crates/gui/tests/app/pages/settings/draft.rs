use super::*;

fn saved() -> SettingsValues {
    SettingsValues::default()
}

#[test]
fn a_new_draft_has_no_edits() {
    let draft = Draft::new(saved());
    assert!(!draft.dirty());
    assert_eq!(draft.current(), &saved());
}

#[test]
fn an_edit_is_dirty_until_it_is_committed() {
    let mut draft = Draft::new(saved());
    draft.current_mut().concurrent_downloads = 11;
    assert!(draft.dirty());
    let committed = draft.commit();
    assert_eq!(committed.concurrent_downloads, 11);
    assert!(!draft.dirty());
}

#[test]
fn reset_throws_the_edits_away() {
    let mut draft = Draft::new(saved());
    draft.current_mut().auto_update = false;
    draft.reset();
    assert!(!draft.dirty());
    assert!(draft.current().auto_update);
}

#[test]
fn a_reload_follows_the_saved_values_when_there_are_no_edits() {
    let mut draft = Draft::new(saved());
    let mut newer = saved();
    newer.beta_channel = true;
    draft.reload(&newer);
    assert!(draft.current().beta_channel);
    assert!(!draft.dirty());
}

#[test]
fn a_reload_keeps_the_edits_in_progress() {
    let mut draft = Draft::new(saved());
    draft.current_mut().beta_channel = true;
    draft.reload(&saved());
    assert!(draft.current().beta_channel);
    assert!(draft.dirty());
}
