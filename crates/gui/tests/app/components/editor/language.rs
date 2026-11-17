use super::*;

#[test]
fn the_extension_names_the_language_whatever_its_case() {
    assert_eq!(Language::of("config/sodium-options.json"), Language::Json);
    assert_eq!(Language::of("pack.MCMETA"), Language::Json);
    assert_eq!(Language::of("config/fml.toml"), Language::Toml);
    assert_eq!(Language::of("config/paper.YML"), Language::Yaml);
    assert_eq!(Language::of("server.properties"), Language::Ini);
    assert_eq!(Language::of("config/forge.cfg"), Language::Ini);
    assert_eq!(Language::of("config/log4j2.xml"), Language::Xml);
    assert_eq!(Language::of("kubejs/server_scripts/recipes.js"), Language::Script);
    assert_eq!(Language::of("scripts/ore.zs"), Language::Script);
    assert_eq!(Language::of("world/datapacks/a/data/x/functions/tick.mcfunction"), Language::Function);
}

#[test]
fn a_text_file_is_told_by_where_it_sits() {
    assert_eq!(Language::of("crash-reports/crash-2026-01-02_03.04.05-client.txt"), Language::Crash);
    assert_eq!(
        Language::of("crash-2026-01-02_03.04.05-server.txt"),
        Language::Crash,
        "a crash report moved elsewhere"
    );
    assert_eq!(Language::of("hs_err_pid1234.txt"), Language::Crash);
    assert_eq!(Language::of("logs/latest.log"), Language::Log);
    assert_eq!(Language::of("logs/telemetry/notes.txt"), Language::Log);
    assert_eq!(Language::of("options.txt"), Language::Ini);
    assert_eq!(Language::of("optionsof.txt"), Language::Ini);
    assert_eq!(Language::of("notes.txt"), Language::Plain);
}

#[test]
fn a_name_with_no_extension_or_an_unknown_one_is_plain_text() {
    assert_eq!(Language::of("LICENSE"), Language::Plain);
    assert_eq!(Language::of(".gitignore"), Language::Plain);
    assert_eq!(Language::of("mods/readme.md"), Language::Plain);
    assert_eq!(Language::of("my.json/readme"), Language::Plain, "only the file's own name counts");
}

#[test]
fn each_language_knows_its_comment_and_its_indent() {
    assert_eq!(Language::Toml.line_comment(), Some("#"));
    assert_eq!(Language::Script.line_comment(), Some("//"));
    assert_eq!(Language::Log.line_comment(), None);
    assert_eq!(Language::Json.indent_width(), 2);
    assert_eq!(Language::Toml.indent_width(), 4);
    assert!(Language::Yaml.colon_opens_block() && !Language::Json.colon_opens_block());
    assert!(Language::Json.pairs() && !Language::Crash.pairs());
}
