use super::*;

fn entry(name: &str, is_folder: bool, is_link: bool) -> Row {
    Row {
        name: name.to_string(),
        path: name.to_string(),
        is_folder,
        is_link,
        size: 0,
        modified: None,
    }
}

#[test]
fn a_wide_list_shows_every_column() {
    assert_eq!(shown(900.0), Shown { size: true, modified: true });
}

#[test]
fn a_narrowing_list_drops_the_modified_time_first_and_then_the_size() {
    // The split beside the editor leaves the list about 280px, which is room for the names alone.
    let narrow = shown(360.0);
    assert!(narrow.size && !narrow.modified, "the modified time goes first");
    assert_eq!(
        shown(240.0),
        Shown {
            size: false,
            modified: false,
        }
    );
}

#[test]
fn an_entry_is_drawn_with_the_glyph_of_what_it_holds() {
    assert_eq!(glyph(&entry("mods", true, false)), Icon::lucide().folder());
    assert_eq!(glyph(&entry("saves", true, true)), Icon::lucide().folder_symlink());
    assert_eq!(glyph(&entry("latest.log", false, true)), Icon::lucide().file_symlink());
    assert_eq!(
        glyph(&entry("sodium.JAR", false, false)),
        Icon::lucide().file_archive(),
        "the case of the extension does not matter"
    );
    assert_eq!(glyph(&entry("pack.png", false, false)), Icon::lucide().file_image());
    assert_eq!(glyph(&entry("sodium-options.json", false, false)), Icon::lucide().file_code());
    assert_eq!(glyph(&entry("server.properties", false, false)), Icon::lucide().file_cog());
    assert_eq!(glyph(&entry("latest.log", false, false)), Icon::lucide().file_text());
    assert_eq!(glyph(&entry("level.dat", false, false)), Icon::lucide().file());
    assert_eq!(glyph(&entry("LICENSE", false, false)), Icon::lucide().file());
}

#[test]
fn the_selection_tints_a_row_and_a_drop_target_gets_a_ring() {
    assert!(row_style(false, false, false).background.is_none());
    assert!(row_style(false, true, false).background.is_some());
    assert_ne!(row_style(true, false, false).background, row_style(false, true, false).background);
    assert_eq!(row_style(false, true, true).border.width, 1.0);
    assert_eq!(row_style(true, true, false).border.width, 0.0);
}

#[test]
fn a_small_file_is_sized_in_bytes() {
    assert_eq!(size_label(0), "0 B");
    assert_eq!(size_label(1023), "1023 B");
    assert_eq!(size_label(1024), format_bytes(1024));
    assert_eq!(size_label(5 << 20), format_bytes(5 << 20));
}
