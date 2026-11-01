use super::*;

fn row(name: &str, is_folder: bool, size: u64) -> Row {
    Row {
        name: name.to_string(),
        path: format!("x/{name}"),
        is_folder,
        size,
        modified: None,
    }
}

#[test]
fn folders_come_first_and_names_sort_without_case() {
    let rows = vec![
        row("zeta.jar", false, 1),
        row("Beta", true, 0),
        row("alpha.jar", false, 1),
        row("alpha", true, 0),
    ];
    let order: Vec<&str> = sorted(&rows, Sort::default()).iter().map(|row| row.name.as_str()).collect();
    assert_eq!(order, ["alpha", "Beta", "alpha.jar", "zeta.jar"]);
}

#[test]
fn a_descending_size_sort_keeps_the_folders_first() {
    let rows = vec![row("small.jar", false, 1), row("big.jar", false, 9), row("dir", true, 0)];
    let descending = Sort {
        column: Column::Size,
        descending: true,
    };
    let order: Vec<&str> = sorted(&rows, descending).iter().map(|row| row.name.as_str()).collect();
    assert_eq!(order, ["dir", "big.jar", "small.jar"]);
}

#[test]
fn choosing_the_sort_column_again_flips_its_direction() {
    let mut tree = Tree::default();
    tree.sort_by(Column::Size);
    assert_eq!(
        tree.sort,
        Sort {
            column: Column::Size,
            descending: false
        }
    );
    tree.sort_by(Column::Size);
    assert!(tree.sort.descending);
    tree.sort_by(Column::Name);
    assert_eq!(
        tree.sort,
        Sort {
            column: Column::Name,
            descending: false
        }
    );
}

#[test]
fn a_folder_is_read_once_and_its_children_are_then_known() {
    let mut tree = Tree::default();
    assert!(tree.open(String::new()), "the root has not been read yet");
    tree.store(String::new(), Ok(vec![row("mods", true, 0)]));
    assert!(!tree.open(String::new()), "a read folder is not read again");
    assert_eq!(tree.children(""), vec!["x/mods".to_string()]);
}

#[test]
fn a_failed_read_is_kept_as_the_folder_error_and_is_read_again_when_reopened() {
    let mut tree = Tree::default();
    assert!(tree.open("mods".to_string()));
    tree.store("mods".to_string(), Err("denied".to_string()));
    assert_eq!(tree.error_of("mods"), Some("denied"));
    assert!(tree.rows_of("mods").is_none());
    assert!(tree.open("mods".to_string()), "a folder that failed is read again");
}

#[test]
fn the_rows_on_screen_are_the_folder_the_tree_shows() {
    let mut tree = Tree::default();
    tree.store(String::new(), Ok(vec![row("mods", true, 0)]));
    tree.store("mods".to_string(), Ok(vec![row("sodium.jar", false, 4)]));
    tree.open("mods".to_string());
    let on_screen: Vec<String> = tree.sorted_rows().expect("loaded").iter().map(|row| row.path.clone()).collect();
    assert_eq!(on_screen, ["x/sodium.jar"]);
}
