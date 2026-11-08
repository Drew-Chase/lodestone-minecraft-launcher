use super::*;

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

    assert!(router.go_forward());
    assert_eq!(router.current(), Page::News);
    assert!(router.go_forward());
    assert_eq!(router.current(), Page::Settings);
    assert!(!router.go_forward(), "forward stops at the last entry");
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
    assert!(router.go_forward());
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
