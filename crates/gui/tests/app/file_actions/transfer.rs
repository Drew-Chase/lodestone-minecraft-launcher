use super::*;

fn paths(names: &[&str]) -> Vec<PathBuf> {
    names.iter().map(PathBuf::from).collect()
}

fn clip(names: &[&str], cut: bool) -> Clip {
    Clip { paths: paths(names), cut }
}

#[test]
fn nothing_on_either_clipboard_pastes_nothing() {
    assert_eq!(paste_plan(None, None), None);
    assert_eq!(paste_plan(Some(Vec::new()), None), None, "an empty list is nothing to paste");
}

#[test]
fn files_copied_in_another_app_are_copied_in() {
    let system = paths(&["/home/me/Downloads/pack.zip"]);
    assert_eq!(paste_plan(Some(system.clone()), None), Some((system, Mode::Copy)));
}

#[test]
fn files_cut_here_are_moved_when_the_system_clipboard_still_holds_them() {
    let cut = clip(&["/inst/a.toml", "/inst/b.toml"], true);
    // The system may list them in another order.
    let system = paths(&["/inst/b.toml", "/inst/a.toml"]);
    assert_eq!(paste_plan(Some(system), Some(&cut)), Some((cut.paths.clone(), Mode::Move)));
}

#[test]
fn files_copied_here_are_copied() {
    let copied = clip(&["/inst/a.toml"], false);
    assert_eq!(
        paste_plan(Some(copied.paths.clone()), Some(&copied)),
        Some((copied.paths.clone(), Mode::Copy))
    );
}

#[test]
fn a_newer_copy_in_another_app_wins_over_an_older_cut_here() {
    let cut = clip(&["/inst/a.toml"], true);
    let system = paths(&["/home/me/other.txt"]);
    assert_eq!(
        paste_plan(Some(system.clone()), Some(&cut)),
        Some((system, Mode::Copy)),
        "the cut entries are not what the clipboard holds now, so they stay where they are"
    );
}

#[test]
fn the_launchers_own_list_is_used_when_the_system_clipboard_cannot_be_read() {
    let cut = clip(&["/inst/a.toml"], true);
    assert_eq!(paste_plan(None, Some(&cut)), Some((cut.paths.clone(), Mode::Move)));
    let copied = clip(&["/inst/a.toml"], false);
    assert_eq!(paste_plan(Some(Vec::new()), Some(&copied)), Some((copied.paths.clone(), Mode::Copy)));
}
