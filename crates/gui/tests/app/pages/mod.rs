use super::*;

#[test]
fn every_page_has_a_title() {
    for page in [Page::Login, Page::Home, Page::Instance, Page::Settings, Page::Showcase] {
        assert!(!page.title().is_empty(), "{page:?} needs a title");
    }
}

#[test]
fn login_boots_first() {
    assert_eq!(Page::default(), Page::Login, "the app must boot to the login gate");
}
