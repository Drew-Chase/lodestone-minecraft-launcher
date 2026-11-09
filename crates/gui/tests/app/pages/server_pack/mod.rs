use super::*;
use lodestone_export::server_pack::{ModDetection, Side};

/// A scanned mod with `side` as its detected side, and `mod_id` as its manifest id.
fn row(name: &str, side: Option<Side>, mod_id: Option<&str>) -> ModRow {
    ModRow {
        detection: ModDetection {
            path: PathBuf::from(format!("/instance/mods/{name}")),
            filename: name.to_string(),
            side,
            evidence: String::new(),
            loader: None,
            mod_id: mod_id.map(str::to_string),
            icon: None,
        },
        icon: None,
    }
}

/// A wizard for an instance at `/instance` whose scan found `rows`, on the Review step.
fn reviewing(rows: Vec<ModRow>) -> State {
    let mut state = State::new("id".into(), "Test".into(), PathBuf::from("/instance"), Arc::new(AtomicBool::new(false)));
    state.scanned(rows);
    state
}

#[test]
fn a_client_only_mod_is_left_out_and_every_other_mod_is_taken() {
    let state = reviewing(vec![
        row("a.jar", Some(Side::ClientOnly), None),
        row("b.jar", Some(Side::ServerOnly), None),
        row("c.jar", Some(Side::ClientAndServer), None),
        row("d.jar", None, None),
    ]);
    let counts = state.counts();
    assert_eq!((counts.taken, counts.left_out), (3, 1));
    assert_eq!((counts.client, counts.server, counts.both, counts.unsure), (1, 1, 1, 1));
}

#[test]
fn a_choice_overrides_the_detected_side_and_is_remembered_by_mod_id() {
    let mut state = reviewing(vec![row("a.jar", Some(Side::ClientOnly), Some("a"))]);
    let remember = state.choose(&[0], Some(Choice::Include));
    assert!(state.takes(0));
    assert_eq!(remember, vec![("a".to_string(), Some(Choice::Include))]);
    let remember = state.choose(&[0], None);
    assert!(!state.takes(0));
    assert_eq!(remember, vec![("a".to_string(), None)]);
}

#[test]
fn a_mod_without_an_id_keeps_its_choice_for_this_session_only() {
    let mut state = reviewing(vec![row("a.jar", Some(Side::ServerOnly), None)]);
    let remember = state.choose(&[0], Some(Choice::Exclude));
    assert!(remember.is_empty(), "nothing is remembered without an id");
    assert_eq!(state.choice_of(0), Some(Choice::Exclude));
}

#[test]
fn remembered_choices_fill_in_without_overwriting_a_choice_made_this_session() {
    let mut state = reviewing(vec![
        row("a.jar", Some(Side::ClientOnly), Some("a")),
        row("b.jar", Some(Side::ClientOnly), Some("b")),
    ]);
    state.choose(&[0], Some(Choice::Exclude));
    state.remember(vec![
        ("a".into(), "include".into()),
        ("b".into(), "include".into()),
        ("c".into(), "unknown".into()),
    ]);
    assert_eq!(state.choice_of(0), Some(Choice::Exclude));
    assert_eq!(state.choice_of(1), Some(Choice::Include));
}

#[test]
fn the_list_filters_by_side_and_searches_file_names() {
    let mut state = reviewing(vec![
        row("Sodium.jar", Some(Side::ClientOnly), None),
        row("lithium.jar", Some(Side::ClientAndServer), None),
        row("spark.jar", None, None),
    ]);
    state.update(Message::Filter(Filter::Both));
    assert_eq!(state.visible, vec![1]);
    state.update(Message::Filter(Filter::All));
    state.update(Message::Search("SODIUM".into()));
    assert_eq!(state.visible, vec![0]);
}

#[test]
fn the_list_sorts_by_side_then_name_and_by_name_on_request() {
    let mut state = reviewing(vec![
        row("zeta.jar", Some(Side::ServerOnly), None),
        row("alpha.jar", Some(Side::ClientOnly), None),
        row("beta.jar", None, None),
        row("gamma.jar", Some(Side::ClientAndServer), None),
    ]);
    assert_eq!(state.visible, vec![1, 0, 3, 2]);
    state.update(Message::Sort(Sort::Name));
    assert_eq!(state.visible, vec![1, 2, 3, 0]);
}

#[test]
fn select_visible_selects_every_visible_row_and_then_clears_them() {
    let mut state = reviewing(vec![row("a.jar", None, None), row("b.jar", None, None)]);
    state.update(Message::SelectVisible);
    assert_eq!(state.selected.len(), 2);
    state.update(Message::SelectVisible);
    assert!(state.selected.is_empty());
}

#[test]
fn the_folders_a_new_pack_leaves_out_start_unticked() {
    let mut state = reviewing(vec![]);
    state.folders_listed(vec![
        FolderRow {
            name: "config".into(),
            is_folder: true,
        },
        FolderRow {
            name: "logs".into(),
            is_folder: true,
        },
        FolderRow {
            name: "resourcepacks".into(),
            is_folder: true,
        },
    ]);
    assert!(state.folder_ticked(0));
    assert!(!state.folder_ticked(1));
    assert!(!state.folder_ticked(2));
    state.update(Message::Folder(0));
    assert!(!state.folder_ticked(0));
}

#[test]
fn the_pack_is_named_after_the_instance_and_its_output() {
    let mut state = reviewing(vec![]);
    state.name = "Aether Legacy".into();
    assert_eq!(state.pack_name(), "Aether Legacy server pack");
    state.kind = Kind::Zip;
    assert_eq!(state.pack_name(), "Aether Legacy server pack.zip");
}

#[test]
fn a_destination_inside_the_instance_blocks_the_export() {
    let mut state = reviewing(vec![]);
    assert_eq!(state.export_blocker(), Some(Blocker::NoDestination));
    state.destination = Some(PathBuf::from("/instance/out"));
    assert_eq!(state.export_blocker(), Some(Blocker::Inside));
}

#[test]
fn a_scan_or_listing_of_another_folder_is_ignored() {
    let mut state = State::new("id".into(), "Test".into(), PathBuf::from("/instance"), Arc::new(AtomicBool::new(false)));
    assert!(state.scanning_for(Path::new("/instance")));
    assert!(!state.scanning_for(Path::new("/other")));
    state.scan_tick(ScanTick {
        folder: PathBuf::from("/other"),
        index: 1,
        total: 2,
        name: "x.jar".into(),
        side: None,
    });
    assert_eq!(state.tally, Tally::default());
    state.scan_tick(ScanTick {
        folder: PathBuf::from("/instance"),
        index: 1,
        total: 2,
        name: "x.jar".into(),
        side: None,
    });
    assert_eq!(state.tally.unsure, 1);
}

#[test]
fn a_finished_export_writes_its_receipt_and_moves_to_done() {
    let mut state = reviewing(vec![
        row("client.jar", Some(Side::ClientOnly), None),
        row("server.jar", Some(Side::ServerOnly), None),
        row("unsure.jar", None, None),
    ]);
    state.choose(&[0], Some(Choice::Include));
    state.finish(PathBuf::from("/out/Test server pack"), 5);
    assert_eq!(state.step, Step::Done);
    let receipt = state.receipt.expect("the receipt is kept");
    assert_eq!(
        (receipt.taken, receipt.left_out, receipt.unsure, receipt.chosen, receipt.files),
        (3, 0, 1, 1, 5)
    );
}

#[test]
fn the_stored_text_of_a_choice_reads_back_as_the_same_choice() {
    for choice in [Choice::Include, Choice::Exclude] {
        assert_eq!(stored_choice(stored(choice)), Some(choice));
    }
    assert_eq!(stored_choice("maybe"), None);
}

#[test]
fn the_server_is_offered_for_every_loader_but_quilt() {
    let mut state = reviewing(vec![]);
    state.setup = Some(ServerSetup {
        kind: ServerKind::Quilt,
        minecraft: "1.21.1".into(),
        loader_version: None,
    });
    assert!(!state.server_available());
    state.update(Message::InstallServer(true));
    assert!(!state.install_server, "a Quilt server cannot be installed yet");

    state.setup = Some(ServerSetup {
        kind: ServerKind::Fabric,
        minecraft: "1.21.1".into(),
        loader_version: Some("0.16.10".into()),
    });
    state.update(Message::InstallServer(true));
    assert!(state.install_server);
    state.update(Message::AcceptEula(true));
    assert!(state.accept_eula);
}
