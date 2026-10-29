//! Tests for `pages/instance/hero.rs`, kept out of the source file. They are
//! compiled as the `tests` child module of `hero`.

use chrono::Local;

use super::*;

#[test]
fn the_identity_line_matches_the_reference() {
    let instance = Instance::seed_pack(Local::now()).remove(0);
    assert_eq!(meta_line(&instance), "/instances/aether-legacy · 4.8 GB · last saved 02:14");
}
