use super::*;
use crate::app::export::tree::Row;
use crate::app::pages::library::instance::Loader;
use crate::app::pages::library::instance::fixture;

fn dialog() -> Dialog {
    Dialog::new(
        &fixture("instance-id", "Enhanced Worlds", Loader::Fabric),
        PathBuf::from("/instances/enhanced"),
    )
}

fn row(name: &str, is_folder: bool) -> Row {
    Row {
        name: name.to_string(),
        path: name.to_string(),
        is_folder,
        size: 0,
        modified: None,
    }
}

/// A dialog whose top-level folder has been read as `rows`.
fn read(rows: Vec<Row>) -> Dialog {
    let mut dialog = dialog();
    dialog.tree.store(String::new(), Ok(rows));
    dialog
}

#[test]
fn a_dialog_cannot_export_until_its_top_level_has_a_taken_entry() {
    let mut dialog = dialog();
    assert!(!dialog.ready(), "nothing has been read yet");
    dialog.tree.store(String::new(), Ok(Vec::new()));
    assert!(!dialog.ready(), "an empty instance has nothing to take");
    dialog.tree.store(String::new(), Ok(vec![row("mods", true)]));
    assert!(dialog.ready());
}

#[test]
fn a_new_export_leaves_logs_and_worlds_out_by_default() {
    let dialog = read(vec![row("logs", true), row("saves", true), row("mods", true)]);
    assert_eq!(dialog.selection.state("logs"), CheckState::Unchecked);
    assert_eq!(dialog.selection.state("saves"), CheckState::Unchecked);
    assert_eq!(dialog.selection.state("mods"), CheckState::Checked);
}

#[test]
fn a_row_toggles_off_and_back_on() {
    let mut dialog = read(vec![row("mods", true)]);
    dialog.toggle("mods");
    assert_eq!(dialog.selection.state("mods"), CheckState::Unchecked);
    dialog.toggle("mods");
    assert_eq!(dialog.selection.state("mods"), CheckState::Checked);
}

#[test]
fn the_header_turns_every_row_on_screen_off_then_on() {
    let mut dialog = read(vec![row("mods", true), row("config", true)]);
    assert_eq!(dialog.header_state(), CheckState::Checked);
    dialog.toggle_all();
    assert_eq!(dialog.header_state(), CheckState::Unchecked);
    dialog.toggle_all();
    assert_eq!(dialog.header_state(), CheckState::Checked);
}

#[test]
fn a_file_name_carries_the_format_extension() {
    let mut dialog = dialog();
    assert_eq!(dialog.file_name(), "Enhanced Worlds-1.0.0.mrpack");
    dialog.format = Format::Zip;
    assert_eq!(dialog.file_name(), "Enhanced Worlds-1.0.0.zip");
    dialog.format = Format::Lodestone;
    assert_eq!(dialog.file_name(), "Enhanced Worlds-1.0.0.lodestone");
}

#[test]
fn the_metadata_is_trimmed_and_names_the_loader() {
    let mut dialog = dialog();
    dialog.name = "  Enhanced Worlds ".to_string();
    dialog.version = " 2.0 ".to_string();
    let metadata = dialog.metadata();
    assert_eq!(metadata.name, "Enhanced Worlds");
    assert_eq!(metadata.version, "2.0");
    assert_eq!(metadata.loader, "fabric");
    assert_eq!(metadata.minecraft, "1.20.1");
}

#[test]
fn the_mods_choice_applies_to_the_export_request() {
    let mut dialog = dialog();
    let _ = dialog.update(Message::ModsSelected(MODS_OFFLINE));
    assert!(dialog.offline);
    let _ = dialog.update(Message::ModsSelected(MODS_ONLINE));
    assert!(!dialog.offline);
}

#[test]
fn a_mrpack_request_does_not_copy_the_launcher() {
    let request = dialog().request(PathBuf::from("/exports/pack.mrpack")).expect("builds");
    assert_eq!(request.format, Format::Mrpack);
    assert!(request.executable.as_os_str().is_empty());
    assert_eq!(request.target, PathBuf::from("/exports/pack.mrpack"));
}
