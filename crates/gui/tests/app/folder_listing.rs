use std::fs;

use super::*;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lodestone-listing-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("the scratch folder");
    dir
}

fn names(rows: &[Row]) -> Vec<&str> {
    let mut names: Vec<&str> = rows.iter().map(|row| row.name.as_str()).collect();
    names.sort_unstable();
    names
}

#[test]
fn a_folder_is_listed_with_paths_inside_the_instance() {
    let root = scratch("paths");
    fs::create_dir_all(root.join("config").join("fabric")).expect("the folders");
    fs::write(root.join("config").join("sodium.json"), "{}").expect("a file");

    let rows = read_rows(&root, "config", Entries::All).expect("the listing");
    assert_eq!(names(&rows), ["fabric", "sodium.json"]);
    let file = rows.iter().find(|row| row.name == "sodium.json").expect("the file");
    assert_eq!(file.path, "config/sodium.json");
    assert_eq!((file.is_folder, file.size), (false, 2));
    let folder = rows.iter().find(|row| row.name == "fabric").expect("the folder");
    assert_eq!((folder.is_folder, folder.size, folder.path.as_str()), (true, 0, "config/fabric"));
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn the_instance_folder_itself_is_the_empty_path() {
    let root = scratch("root");
    fs::write(root.join("options.txt"), "fov:70").expect("a file");
    let rows = read_rows(&root, "", Entries::All).expect("the listing");
    assert_eq!(rows[0].path, "options.txt");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_folder_that_is_gone_is_an_error() {
    let root = scratch("gone");
    assert!(read_rows(&root, "missing", Entries::All).is_err());
    let _ = fs::remove_dir_all(&root);
}

#[cfg(unix)]
#[test]
fn links_are_listed_for_the_files_tab_and_left_out_of_an_export() {
    let root = scratch("links");
    fs::create_dir(root.join("saves")).expect("a folder");
    std::os::unix::fs::symlink(root.join("saves"), root.join("worlds")).expect("a link to a folder");
    std::os::unix::fs::symlink(root.join("nowhere"), root.join("broken")).expect("a broken link");

    let all = read_rows(&root, "", Entries::All).expect("the listing");
    assert_eq!(names(&all), ["broken", "saves", "worlds"]);
    let link = all.iter().find(|row| row.name == "worlds").expect("the link");
    assert!(link.is_link && link.is_folder, "a link to a folder opens as a folder");
    let broken = all.iter().find(|row| row.name == "broken").expect("the broken link");
    assert!(broken.is_link && !broken.is_folder);

    let exportable = read_rows(&root, "", Entries::Exportable).expect("the listing");
    assert_eq!(names(&exportable), ["saves"]);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn choosing_a_column_sorts_by_it_and_choosing_it_again_flips_the_direction() {
    let sort = Sort::default().by(Column::Size);
    assert_eq!((sort.column, sort.descending), (Column::Size, false));
    let flipped = sort.by(Column::Size);
    assert!(flipped.descending);
    assert!(!flipped.by(Column::Name).descending, "another column starts ascending");
}

#[test]
fn the_order_lists_the_same_rows_as_the_sorted_view() {
    let row = |name: &str, is_folder: bool, size: u64| Row {
        name: name.to_string(),
        path: name.to_string(),
        is_folder,
        is_link: false,
        size,
        modified: None,
    };
    let rows = [
        row("b.txt", false, 5),
        row("mods", true, 0),
        row("A.txt", false, 9),
        row("config", true, 0),
    ];
    let by_name = order(&rows, Sort::default());
    assert_eq!(by_name, [3, 1, 2, 0], "folders first, then names without case");
    let names: Vec<&str> = sorted(&rows, Sort::default()).into_iter().map(|row| row.name.as_str()).collect();
    assert_eq!(names, ["config", "mods", "A.txt", "b.txt"]);
    let by_size = order(&rows, Sort::default().by(Column::Size).by(Column::Size));
    assert_eq!(by_size, [3, 1, 2, 0], "the largest file first, and the folders still lead, by name");
}
