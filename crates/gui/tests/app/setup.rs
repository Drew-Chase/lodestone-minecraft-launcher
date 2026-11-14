//! Tests for `app/setup.rs`: the page the launcher shows while no account plays.

use super::gate_page;
use crate::app::pages::Page;

#[test]
fn the_setup_wizard_shows_on_first_run_and_the_sign_in_page_after_it() {
    assert_eq!(gate_page(false, false), Page::Setup);
    assert_eq!(gate_page(false, true), Page::Login);
}

#[test]
fn a_portable_launcher_shows_the_sign_in_page_without_the_wizard() {
    assert_eq!(gate_page(true, false), Page::Login);
    assert_eq!(gate_page(true, true), Page::Login);
}
