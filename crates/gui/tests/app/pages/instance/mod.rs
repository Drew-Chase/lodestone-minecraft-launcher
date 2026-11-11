//! Tests for the instance page shell. They live here rather than inline and are
//! compiled as the `tests` child module of `instance`, so they can reach the
//! page's private state.

use std::path::PathBuf;

use chrono::Local;

use super::files::{Difficulty, GameMode};
use super::logs::{LogLevel, LogLine};
use super::*;
use crate::app::launch::games::{GameView, LaunchStatus};
use crate::app::pages::library::instance::{Loader, fixture};
use crate::app::world_pins::WorldPins;

fn aether() -> Instance {
    fixture("aether", "Aether Legacy", Loader::Fabric)
}

/// The game view of an instance with no game running.
fn idle_game() -> GameView<'static> {
    GameView {
        status: LaunchStatus::Idle,
        console: None,
    }
}

/// What a scan of a small instance folder finds: one mod, one world, one
/// server, one capture and two log lines.
fn sample_files() -> InstanceFiles {
    InstanceFiles {
        folder_bytes: 1_200_000_000,
        resource_packs: Vec::new(),
        shader_packs: Vec::new(),
        mods: vec![files::ModFile {
            file_name: "sodium.jar".to_string(),
            enabled: true,
            name: "Sodium".to_string(),
            author: Some("JellySquid".to_string()),
            version: Some("0.5.8".to_string()),
            icon: None,
        }],
        worlds: vec![files::WorldFile {
            path: PathBuf::from("/instances/aether/saves/Aether Hub"),
            name: "Aether Hub".to_string(),
            game_mode: Some(GameMode::Survival),
            difficulty: Some(Difficulty::Hard),
            seed: Some(-4_827_193_047_281),
            version: Some("1.20.1".to_string()),
            size_bytes: 1_503_238_554,
            saved: Some(Local::now()),
            play_time: Some(std::time::Duration::from_secs(48 * 3600 + 12 * 60)),
            icon: None,
            backdrop: None,
            backed_up: false,
        }],
        servers: vec![files::ServerFile {
            name: "Aether Server".to_string(),
            address: "play.aether.net".to_string(),
            icon: None,
        }],
        screenshots: vec![files::ScreenshotFile {
            file_name: "2026-10-29_12.00.00.png".to_string(),
            size_label: "2.1 MB".to_string(),
            path: PathBuf::from("/instances/aether/screenshots/2026-10-29_12.00.00.png"),
            modified: std::time::SystemTime::UNIX_EPOCH,
            bytes: 2_100_000,
        }],
        log: vec![
            LogLine {
                time: "12:00:00".to_string(),
                level: LogLevel::Info,
                message: "Loading Fabric Loader".to_string(),
            },
            LogLine {
                time: "12:00:01".to_string(),
                level: LogLevel::Warn,
                message: "Mod registered legacy recipe handler".to_string(),
            },
        ],
    }
}

#[test]
fn all_tabs_have_labels() {
    let labels: Vec<_> = Tab::ALL.iter().map(|tab| tab.label()).collect();
    assert_eq!(labels, ["Content", "Versions", "Worlds", "Screenshots", "Logs", "Settings"]);
}

#[test]
fn the_versions_tab_is_shown_only_for_an_instance_that_came_from_a_platform() {
    let local = aether();
    assert!(!Tab::visible(Some(&local)).contains(&Tab::Versions));
    let mut installed = aether();
    installed.source_platform = Some("modrinth".to_string());
    assert!(Tab::visible(Some(&installed)).contains(&Tab::Versions));
    assert_eq!(Tab::visible(Some(&local)).len(), 5, "the other tabs stay");
}

#[test]
fn starts_without_an_instance() {
    let page = InstancePage::default();
    assert!(page.instance.is_none());
}

#[test]
fn show_resets_the_tab() {
    let mut page = InstancePage::default();

    let _ = page.show(aether(), PathBuf::new(), &LaunchDefaults::default());
    let _ = page.update(Message::TabSelected(Tab::Logs));
    assert_eq!(page.tab, Tab::Logs);

    let _ = page.show(aether(), PathBuf::new(), &LaunchDefaults::default());
    assert_eq!(page.tab, Tab::Content, "opening another instance starts on Content");
}

#[test]
fn game_controls_are_left_to_the_root() {
    let mut page = InstancePage::default();
    let _ = page.show(aether(), PathBuf::new(), &LaunchDefaults::default());
    let _ = page.update(Message::Launch);
    let _ = page.update(Message::Kill);
    let _ = page.update(Message::Restart);
    assert_eq!(
        page.tab,
        Tab::Content,
        "the root starts, stops and restarts the game, so the page keeps its tab"
    );
}

#[test]
fn a_reload_replaces_only_the_shown_instance() {
    let mut page = InstancePage::default();
    let _ = page.show(aether(), PathBuf::new(), &LaunchDefaults::default());
    let mut reloaded = aether();
    reloaded.playtime_seconds = 90;
    page.refresh(reloaded, &LaunchDefaults::default());
    assert_eq!(page.instance().map(|instance| instance.playtime_seconds), Some(90));

    page.refresh(fixture("beta", "Beta", Loader::Quilt), &LaunchDefaults::default());
    assert_eq!(
        page.instance().map(|instance| instance.id.as_str()),
        Some("aether"),
        "another instance is ignored"
    );
}

#[test]
fn the_menu_and_the_source_link_are_left_to_the_root() {
    let mut page = InstancePage::default();
    let _ = page.show(aether(), PathBuf::new(), &LaunchDefaults::default());
    let _ = page.update(Message::Menu(None));
    let _ = page.update(Message::OpenSource);
    assert_eq!(page.tab, Tab::Content, "the root carries these out, so the page keeps its tab");
}

#[test]
fn back_and_navigation_are_left_to_the_root() {
    let mut page = InstancePage::default();
    let _ = page.show(aether(), PathBuf::new(), &LaunchDefaults::default());
    let _ = page.update(Message::TabSelected(Tab::Content));
    let _ = page.update(Message::Back);
    let _ = page.update(Message::Navigate(Page::Library));
    assert_eq!(page.tab, Tab::Content, "the root handles Back and Navigate, so the page keeps its tab");
}

#[test]
fn a_scan_for_the_current_instance_lands() {
    let mut page = InstancePage::default();
    let _ = page.show(aether(), PathBuf::new(), &LaunchDefaults::default());
    let generation = page.generation;

    let _ = page.update(Message::Scanned {
        generation,
        files: sample_files(),
    });
    assert_eq!(page.files.as_ref().map(|files| files.folder_bytes), Some(1_200_000_000));
    assert_eq!(page.screenshots.shots().len(), 1);
}

#[test]
fn a_scan_for_an_earlier_instance_is_dropped() {
    let mut page = InstancePage::default();
    let _ = page.show(aether(), PathBuf::from("/instances/aether"), &LaunchDefaults::default());
    let stale = page.generation;
    let _ = page.show(
        fixture("beta", "Beta", Loader::Quilt),
        PathBuf::from("/instances/beta"),
        &LaunchDefaults::default(),
    );

    let _ = page.update(Message::Scanned {
        generation: stale,
        files: sample_files(),
    });
    assert!(page.files.is_none(), "the page moved on, so the old scan must not land");
}

#[test]
fn packs_read_by_the_watch_replace_the_packs_the_scan_held() {
    let mut page = InstancePage::default();
    let _ = page.show(aether(), PathBuf::new(), &LaunchDefaults::default());
    let generation = page.generation;
    let _ = page.update(Message::Scanned {
        generation,
        files: sample_files(),
    });

    let _ = page.update(Message::FilesRead {
        generation,
        updates: vec![Update::Packs(
            vec![files::PackFile {
                file_name: "Faithful.zip".to_string(),
                name: "Faithful".to_string(),
                size: Some(1),
                is_dir: false,
            }],
            Vec::new(),
        )],
    });

    let files = page.files.as_ref().expect("the scan landed");
    assert_eq!(files.resource_packs.first().map(|pack| pack.name.as_str()), Some("Faithful"));
    assert!(files.shader_packs.is_empty(), "the shader folder was read too, and it holds none");
    assert_eq!(files.mods.len(), 1, "a pack read leaves the mods the scan held as they were");
}

#[test]
fn every_tab_builds_at_narrow_and_wide_widths() {
    let mut page = InstancePage::default();
    let _ = page.show(aether(), PathBuf::new(), &LaunchDefaults::default());
    let generation = page.generation;
    let _ = page.update(Message::Scanned {
        generation,
        files: sample_files(),
    });

    for tab in Tab::ALL {
        let _ = page.update(Message::TabSelected(tab));
        for viewport in [Size::new(900.0, 700.0), Size::new(1400.0, 900.0)] {
            let _ = page.view(viewport, idle_game(), None, &WorldPins::default(), None);
        }
    }
}

#[test]
fn every_tab_builds_before_the_scan_lands() {
    let mut page = InstancePage::default();
    let _ = page.show(aether(), PathBuf::new(), &LaunchDefaults::default());
    for tab in Tab::ALL {
        let _ = page.update(Message::TabSelected(tab));
        let _ = page.view(Size::new(1280.0, 800.0), idle_game(), None, &WorldPins::default(), None);
    }
}

#[test]
fn the_page_builds_without_an_instance() {
    let page = InstancePage::default();
    let _ = page.view(Size::new(1280.0, 800.0), idle_game(), None, &WorldPins::default(), None);
}
