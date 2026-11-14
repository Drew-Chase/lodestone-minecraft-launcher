use super::*;

/// A forward step that refuses no page, which is what a plain forward step is.
fn forward(router: &mut Router) -> bool {
    router.go_forward_where(|_| true)
}

#[test]
fn push_moves_current_and_clears_forward() {
    let mut router = Router::new(Page::Library);
    router.push(Page::News);
    router.push(Page::Settings);
    assert_eq!(router.current(), Page::Settings);
    assert!(router.can_go_back());

    router.go_back();
    assert!(router.can_go_forward());

    router.push(Page::Library);
    assert!(!router.can_go_forward(), "push must discard the forward stack");
}

#[test]
fn pushing_the_current_page_is_a_noop() {
    let mut router = Router::new(Page::Library);
    assert!(!router.push(Page::Library));
    assert!(!router.can_go_back());
    assert_eq!(router.current(), Page::Library);
}

#[test]
fn back_and_forward_walk_the_stacks() {
    let mut router = Router::new(Page::Library);
    router.push(Page::News);
    router.push(Page::Settings);

    assert!(router.go_back());
    assert_eq!(router.current(), Page::News);
    assert!(router.go_back());
    assert_eq!(router.current(), Page::Library);
    assert!(!router.go_back(), "back stops at the first entry");

    assert!(forward(&mut router));
    assert_eq!(router.current(), Page::News);
    assert!(forward(&mut router));
    assert_eq!(router.current(), Page::Settings);
    assert!(!forward(&mut router), "forward stops at the last entry");
}

#[test]
fn replace_swaps_in_place() {
    let mut router = Router::new(Page::Library);
    router.push(Page::News);
    router.replace(Page::Settings);

    assert_eq!(router.current(), Page::Settings);
    assert!(router.can_go_back());
    assert!(router.go_back());
    assert_eq!(router.current(), Page::Library, "replace must keep the back stack");
    assert!(router.can_go_forward(), "the replaced entry stays reachable via forward");
    assert!(forward(&mut router));
    assert_eq!(router.current(), Page::Settings);
}

#[test]
fn can_go_flags_track_the_stacks() {
    let mut router = Router::new(Page::Library);
    assert!(!router.can_go_back());
    assert!(!router.can_go_forward());

    router.push(Page::News);
    assert!(router.can_go_back());
    assert!(!router.can_go_forward());
}

#[test]
fn the_back_page_is_the_entry_a_back_step_returns_to() {
    let mut router = Router::new(Page::Discover);
    assert_eq!(router.back_page(), None);
    router.push(Page::Resource);
    assert_eq!(router.back_page(), Some(Page::Discover));
    router.go_back();
    assert_eq!(router.back_page(), None);
}

#[test]
fn back_passes_over_the_pages_it_refuses_and_keeps_them_for_forward() {
    // Library, then the sign-in page, then Settings. A step back that refuses the sign-in page lands on Library, and a forward
    // step still reaches the sign-in page on its way to Settings.
    let mut router = Router::new(Page::Library);
    router.push(Page::Login);
    router.push(Page::Settings);

    assert!(router.go_back_where(|page| page != Page::Login));
    assert_eq!(router.current(), Page::Library);
    assert!(forward(&mut router));
    assert_eq!(router.current(), Page::Login, "a refused page stays in the history");
    assert!(forward(&mut router));
    assert_eq!(router.current(), Page::Settings);
}

#[test]
fn a_step_no_entry_is_accepted_for_leaves_the_history_as_it_was() {
    let mut router = Router::new(Page::Login);
    router.push(Page::Setup);

    assert!(!router.go_back_where(|page| page != Page::Login));
    assert_eq!(router.current(), Page::Setup);
    assert!(router.can_go_back());
}

#[test]
fn leave_returns_to_the_page_below_and_drops_the_sign_in_entry() {
    let mut router = Router::new(Page::Library);
    router.push(Page::Settings);
    router.push(Page::Login);

    router.leave(Page::Settings);
    assert_eq!(router.current(), Page::Settings);
    assert_eq!(router.back_page(), Some(Page::Library), "the sign-in page must not stay in the history");
    assert!(router.go_back());
    assert_eq!(router.current(), Page::Library);
}

#[test]
fn leave_to_another_page_keeps_the_entry_below_it() {
    let mut router = Router::new(Page::Library);
    router.push(Page::Login);

    router.leave(Page::Settings);
    assert_eq!(router.current(), Page::Settings);
    assert_eq!(router.back_page(), Some(Page::Library));
}
