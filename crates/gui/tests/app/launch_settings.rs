//! Tests for `launch_settings.rs`, kept out of the source file. They compile as its `tests` child module.

use super::*;

fn custom_defaults() -> LaunchDefaults {
    LaunchDefaults {
        memory_mb: 6144,
        game_args: "--demo".to_string(),
        ..LaunchDefaults::default()
    }
}

#[test]
fn built_in_defaults_match_the_reference() {
    let defaults = LaunchDefaults::default();
    assert_eq!(defaults.java, JavaChoice::Follow);
    assert_eq!(defaults.memory_mb, DEFAULT_MEMORY_MB);
    assert_eq!(defaults.collector, Collector::G1);
    assert_eq!(defaults.resolution, Resolution { width: 1920, height: 1080 });
    assert_eq!(defaults.launch_mode, LaunchMode::Windowed);
    assert!(defaults.jvm_args.is_empty());
    assert!(!defaults.skip_integrity);
}

#[test]
fn a_fresh_instance_inherits_every_setting() {
    let settings = InstanceSettings::default();
    assert_eq!(settings.inherited_count(), 13);
    let resolved = settings.resolve(&custom_defaults());
    assert_eq!(resolved.memory_mb, 6144);
    assert_eq!(resolved.game_args, "--demo");
}

#[test]
fn an_override_wins_over_the_default_and_only_its_setting_stops_inheriting() {
    let mut settings = InstanceSettings::default();
    settings.override_with(LaunchChange::Memory(8192));
    settings.override_with(LaunchChange::VerboseLogging(true));
    assert_eq!(settings.inherited_count(), 11);
    let resolved = settings.resolve(&custom_defaults());
    assert_eq!(resolved.memory_mb, 8192);
    assert!(resolved.verbose_logging);
    assert_eq!(resolved.game_args, "--demo", "an unchanged setting still inherits");
}

#[test]
fn an_explicit_follow_overrides_a_bundled_default() {
    let mut defaults = LaunchDefaults::default();
    defaults.apply(LaunchChange::Java(JavaChoice::Bundled {
        component: "java-runtime-gamma".to_string(),
    }));
    let mut settings = InstanceSettings::default();
    assert_eq!(
        settings.resolve(&defaults).java,
        JavaChoice::Bundled {
            component: "java-runtime-gamma".to_string()
        }
    );
    settings.override_with(LaunchChange::Java(JavaChoice::Follow));
    assert_eq!(settings.resolve(&defaults).java, JavaChoice::Follow);
}

#[test]
fn reset_clears_launch_overrides_and_keeps_snapshots() {
    let mut settings = InstanceSettings {
        snapshots: true,
        ..InstanceSettings::default()
    };
    settings.override_with(LaunchChange::JvmArgs("-Dx=1".to_string()));
    settings.override_with(LaunchChange::PreLaunchHook("echo hi".to_string()));
    settings.reset();
    assert_eq!(settings.inherited_count(), 13);
    assert!(settings.snapshots);
}

#[test]
fn the_json_of_an_empty_override_set_reads_back_as_inherited() {
    let settings: InstanceSettings = serde_json::from_str("{}").expect("an empty object parses");
    assert_eq!(settings, InstanceSettings::default());
}

#[test]
fn overrides_round_trip_through_json() {
    let mut settings = InstanceSettings::default();
    settings.override_with(LaunchChange::Env(vec![EnvVar {
        name: "MANGOHUD".to_string(),
        value: "1".to_string(),
    }]));
    settings.override_with(LaunchChange::Resolution(Resolution { width: 1280, height: 720 }));
    let json = serde_json::to_string(&settings).expect("settings serialize");
    let back: InstanceSettings = serde_json::from_str(&json).expect("settings parse");
    assert_eq!(back, settings);
}

#[test]
fn defaults_with_missing_fields_fill_in_the_built_ins() {
    let defaults: LaunchDefaults = serde_json::from_str(r#"{"memory_mb": 8192}"#).expect("partial defaults parse");
    assert_eq!(defaults.memory_mb, 8192);
    assert_eq!(defaults.collector, Collector::G1);
    assert_eq!(defaults.resolution, Resolution::default());
}

#[test]
fn arguments_split_on_whitespace_and_keep_quoted_runs_together() {
    assert_eq!(split_arguments("-Xss1M   -Dflag=on"), ["-Xss1M", "-Dflag=on"]);
    assert_eq!(split_arguments(r#"-Dname="a b" --x"#), ["-Dname=a b", "--x"]);
    assert_eq!(split_arguments(r#""" tail"#), ["", "tail"], "an empty quoted argument is kept");
    assert!(split_arguments("   ").is_empty());
}

#[test]
fn environment_names_must_be_present_and_plain() {
    assert!(valid_env_name("MANGOHUD"));
    assert!(!valid_env_name(""));
    assert!(!valid_env_name("A=B"));
}
