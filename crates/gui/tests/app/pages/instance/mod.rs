//! Tests for the instance page shell. They live here rather than inline and are
//! compiled as the `tests` child module of `instance`, so they can reach the
//! page's private state.

use chrono::Local;

use super::*;

fn aether() -> Instance {
    Instance::seed_pack(Local::now()).remove(0)
}

#[test]
fn all_tabs_have_labels() {
    let labels: Vec<_> = Tab::ALL.iter().map(|tab| tab.label()).collect();
    assert_eq!(labels, ["Overview", "Mods", "Worlds", "Screenshots", "Logs", "Settings"]);
}

#[test]
fn starts_without_an_instance() {
    let page = InstancePage::default();
    assert!(page.instance.is_none());
}

#[test]
fn show_resets_the_tab_and_controls() {
    let mut page = InstancePage::default();

    page.show(aether());
    page.update(Message::TabSelected(Tab::Logs));
    page.update(Message::TogglePause);
    assert_eq!(page.tab, Tab::Logs);
    assert!(page.paused, "pause is display state");

    page.show(aether());
    assert_eq!(page.tab, Tab::Overview, "opening another instance starts on Overview");
    assert!(!page.paused);
}

#[test]
fn favorite_toggles() {
    let mut page = InstancePage::default();
    assert!(!page.favorite);
    page.update(Message::ToggleFavorite);
    assert!(page.favorite);
    page.update(Message::ToggleFavorite);
    assert!(!page.favorite);
}

#[test]
fn back_and_navigation_are_left_to_the_root() {
    let mut page = InstancePage::default();
    page.show(aether());
    page.update(Message::TabSelected(Tab::Mods));
    page.update(Message::Back);
    page.update(Message::Navigate(Page::Home));
    assert_eq!(page.tab, Tab::Mods, "the root handles Back and Navigate, so the page keeps its tab");
}

#[test]
fn every_tab_builds_at_narrow_and_wide_widths() {
    let mut page = InstancePage::default();
    page.show(aether());
    for tab in Tab::ALL {
        page.update(Message::TabSelected(tab));
        for viewport in [Size::new(900.0, 700.0), Size::new(1400.0, 900.0)] {
            let _ = page.view(viewport);
        }
    }
}

#[test]
fn the_page_builds_without_an_instance() {
    let page = InstancePage::default();
    let _ = page.view(Size::new(1280.0, 800.0));
}
