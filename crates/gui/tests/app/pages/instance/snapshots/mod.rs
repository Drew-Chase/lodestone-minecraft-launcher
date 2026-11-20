use super::*;
use chrono::NaiveDate;
use lodestone_snapshots::naming::Name;
use lodestone_snapshots::store::Listed;
use lodestone_snapshots::{Kind, Snapshot};
use std::path::PathBuf;

fn loaded(second: u32, kind: Kind) -> Loaded {
    let when = NaiveDate::from_ymd_opt(2026, 11, 20)
        .expect("a date")
        .and_hms_opt(14, 5, second)
        .expect("a time");
    let name = Name { when, copy: 1, kind };
    Loaded {
        listed: Listed {
            name,
            json: PathBuf::from(format!("/inst/.lodestone/snapshots/{}.json.xz", name.stem())),
            zip: None,
        },
        snapshot: Snapshot::default(),
    }
}

#[test]
fn an_instance_with_no_snapshot_offers_none() {
    let state = State::default();
    assert!(state.is_empty());
    assert!(state.find("anything").is_none());
}

#[test]
fn the_list_keeps_its_order_and_finds_a_snapshot_by_its_stem() {
    let mut state = State::default();
    state.set_list(vec![loaded(30, Kind::Manual), loaded(10, Kind::UpdateAll)]);

    assert!(!state.is_empty());
    assert_eq!(state.list().len(), 2);
    assert_eq!(state.list().first().map(|loaded| loaded.listed.name.kind), Some(Kind::Manual));
    let found = state.find("2026-11-20_14-05-10_update-all").expect("the older snapshot is listed");
    assert_eq!(found.listed.name.kind, Kind::UpdateAll);
    assert!(state.find("2026-11-20_14-05-99_manual").is_none());
}
