//! Tests for `pages/instance/hero.rs`, kept out of the source file. They are
//! compiled as the `tests` child module of `hero`.

use super::*;
use crate::app::pages::instance::files::ModFile;
use crate::app::pages::library::instance::{Loader, fixture};

#[test]
fn the_identity_line_reads_the_folder_and_waits_for_the_scan() {
    let instance = fixture("aether-legacy", "Aether Legacy", Loader::Fabric);
    assert_eq!(meta_line(&instance, None), "/instances/aether-legacy · —");

    let files = InstanceFiles {
        folder_bytes: 1_503_238_554,
        ..InstanceFiles::default()
    };
    assert_eq!(meta_line(&instance, Some(&files)), "/instances/aether-legacy · 1.4 GB · No saves yet");
}

#[test]
fn the_mod_badge_counts_the_jars_the_scan_found() {
    assert_eq!(mods_label(None), "— MODS", "a dash until the folder is read");
    let files = InstanceFiles {
        mods: vec![
            ModFile {
                file_name: "a.jar".to_string(),
                enabled: true,
                name: "A".to_string(),
                author: None,
                version: None,
            },
            ModFile {
                file_name: "b.jar.disabled".to_string(),
                enabled: false,
                name: "B".to_string(),
                author: None,
                version: None,
            },
        ],
        ..InstanceFiles::default()
    };
    assert_eq!(mods_label(Some(&files)), "2 MODS", "disabled mods count too");
}
