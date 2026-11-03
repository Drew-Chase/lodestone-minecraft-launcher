use lodestone_minecraft_authentication::{Profile, Session};

use super::*;

/// The moment every check is made, in seconds since the Unix epoch.
const NOW: u64 = 1_700_000_000;

#[test]
fn quick_play_needs_a_version_from_1_20_on() {
    assert!(supports_quick_play("1.20"));
    assert!(supports_quick_play("1.20.1"));
    assert!(supports_quick_play("1.21.4"));
    assert!(!supports_quick_play("1.19.4"));
    assert!(!supports_quick_play("24w14a"), "a snapshot starts the instance alone");
    assert!(!supports_quick_play("1"), "a version without a minor part starts the instance alone");
}

/// A Microsoft account whose session expires `in_seconds` after [`NOW`], holding `refresh_token`.
fn microsoft(in_seconds: u64, refresh_token: Option<&str>) -> Account {
    Account::Microsoft {
        session: Session {
            minecraft_token: "minecraft-token".to_string(),
            expires_at: NOW + in_seconds,
            refresh_token: refresh_token.map(str::to_string),
            profile: Profile {
                id: "0123456789abcdef".to_string(),
                name: "Drew".to_string(),
                skin_url: None,
                slim: false,
                capes: Vec::new(),
            },
        },
    }
}

#[test]
fn an_offline_account_never_needs_a_refresh() {
    let offline = Account::Offline {
        username: "Drew".to_string(),
    };
    assert_eq!(session_check(&offline, NOW), SessionCheck::Fresh);
}

#[test]
fn a_session_far_from_expiry_is_fresh() {
    assert_eq!(session_check(&microsoft(REFRESH_WITHIN + 1, Some("r1")), NOW), SessionCheck::Fresh);
}

#[test]
fn a_session_close_to_expiry_is_refreshed_with_its_refresh_token() {
    assert_eq!(
        session_check(&microsoft(REFRESH_WITHIN, Some("r1")), NOW),
        SessionCheck::Refresh("r1".to_string())
    );
    assert_eq!(session_check(&microsoft(0, Some("r1")), NOW), SessionCheck::Refresh("r1".to_string()));
}

#[test]
fn a_session_close_to_expiry_without_a_refresh_token_needs_a_sign_in() {
    assert_eq!(session_check(&microsoft(10, None), NOW), SessionCheck::SignInAgain);
}

/// The launch settings of the built-in defaults, with no instance overrides.
fn defaults() -> LaunchSettings {
    crate::app::launch_settings::InstanceSettings::default().resolve(&crate::app::launch_settings::LaunchDefaults::default())
}

#[test]
fn the_window_takes_its_size_and_goes_fullscreen_only_in_fullscreen_mode() {
    assert_eq!(window_arguments(&defaults()), ["--width", "1920", "--height", "1080"]);
    let mut fullscreen = defaults();
    fullscreen.launch_mode = LaunchMode::Fullscreen;
    assert_eq!(window_arguments(&fullscreen), ["--width", "1920", "--height", "1080", "--fullscreen"]);
}

#[test]
fn the_debug_copy_turns_the_root_logger_up_and_leaves_other_configs_alone() {
    let config = r#"<Configuration><Loggers><Root level="info"><AppenderRef ref="Console"/></Root></Loggers></Configuration>"#;
    assert!(debug_log_text(config).contains(r#"<Root level="debug">"#));
    assert_eq!(debug_log_text("<Configuration/>"), "<Configuration/>");
}

#[test]
fn a_follow_choice_runs_the_runtime_the_version_asks_for() {
    let vanilla: Manifest = Manifest::parse(r#"{"id":"1.20.1","mainClass":"net.minecraft.client.main.Main"}"#).expect("a minimal manifest parses");
    assert!(matches!(runtime_of(&JavaChoice::Follow, &vanilla), Runtime::Bundled(component) if component == "jre-legacy"));
    assert!(matches!(runtime_of(&JavaChoice::System, &vanilla), Runtime::System));
}

#[test]
fn the_launch_carries_the_instance_memory_collector_arguments_window_and_variables() {
    use crate::app::launch::identity::Identity;
    use crate::app::launch::libraries::Plan;
    use crate::app::launch_settings::{Collector, EnvVar};

    let vanilla = Manifest::parse(r#"{"id":"1.20.1","mainClass":"net.minecraft.client.main.Main"}"#).expect("a minimal manifest parses");
    let mut settings = defaults();
    settings.memory_mb = 6144;
    settings.collector = Collector::Zgc;
    settings.jvm_args = "-Dfoo=1 -Dbar=\"a b\"".to_string();
    settings.game_args = "--demo".to_string();
    settings.env = vec![
        EnvVar {
            name: "MANGOHUD".to_string(),
            value: "1".to_string(),
        },
        EnvVar {
            name: String::new(),
            value: "dropped".to_string(),
        },
    ];
    let target = Target {
        mc_version: "1.20.1".to_string(),
        loader: Loader::Vanilla,
        loader_version: None,
        version_id: None,
        directory: std::env::temp_dir(),
        quick_play: None,
        settings,
    };
    // The launcher checks that its Java program exists, so the test binary stands in for one.
    let java = std::env::current_exe().expect("the test binary has a path");
    let launcher = build(Build {
        paths: &LauncherPaths::default(),
        target: &target,
        merged: &vanilla,
        vanilla: &vanilla,
        plan: &Plan::default(),
        log_config: None,
        identity: &Identity::offline("Drew"),
        java: &java,
        natives: std::env::temp_dir(),
    })
    .expect("the launch builds");

    assert_eq!(
        &launcher.jvm_args()[..2],
        ["-Xmx6144M", "-XX:+UseZGC"],
        "memory and the collector come first"
    );
    assert!(launcher.jvm_args().ends_with(&["-Dfoo=1".to_string(), "-Dbar=a b".to_string()]));
    assert!(
        launcher
            .game_args()
            .ends_with(&["--width", "1920", "--height", "1080", "--demo"].map(String::from))
    );
    assert_eq!(launcher.env().len(), 1, "a variable without a name is left out");
    assert_eq!(
        launcher.env().get("MANGOHUD").map(|value| value.to_string_lossy().into_owned()),
        Some("1".to_string())
    );
}
