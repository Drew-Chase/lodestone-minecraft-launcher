//! Tests for `pages/servers/listed.rs`, kept out of the source file. They are compiled as the `tests` child module of
//! `listed`.

use chrono::Local;
use iced::widget::image::Handle;

use super::*;
use crate::app::pages::instance::files::ServerFile;

/// A server the launcher saved under `id`, called `name` at `address`.
fn saved(id: i64, name: &str, address: &str, favorite: bool) -> SavedServer {
    SavedServer {
        id,
        name: name.to_owned(),
        address: address.to_owned(),
        favorite,
        added_at: Local::now(),
    }
}

/// A server an instance's list holds, with an icon when `icon` is set.
fn file(name: &str, address: &str, icon: bool) -> ServerFile {
    ServerFile {
        name: name.to_owned(),
        address: address.to_owned(),
        icon: icon.then(|| Handle::from_bytes(vec![1, 2, 3])),
    }
}

/// The list an instance named `instance` holds.
fn list(instance: &str, servers: Vec<ServerFile>) -> InstanceServers {
    InstanceServers {
        instance: instance.to_owned(),
        servers,
    }
}

/// A game announced on the local network under `title`, at `host` on `port`.
fn game(title: &str, host: &str, port: u16) -> Announcement {
    Announcement {
        title: title.to_owned(),
        address: ServerAddress { host: host.to_owned(), port },
    }
}

#[test]
fn a_server_saved_and_listed_is_one_entry_under_the_saved_name() {
    let merged = merge(
        &[saved(1, "Alpha", "a.example", true)],
        &[list("Survival", vec![file("Alpha Copy", "a.example", true)])],
        &[],
    );
    assert_eq!(merged.len(), 1);
    assert_eq!(merged[0].name, "Alpha");
    assert!(merged[0].saved.is_some());
    assert!(merged[0].favorite());
    assert_eq!(merged[0].instances, ["Survival"]);
    assert!(merged[0].icon.is_some());
}

#[test]
fn the_default_port_and_the_case_of_a_host_do_not_make_two_entries() {
    let merged = merge(
        &[saved(1, "Hub", "mc.example.net:25565", false)],
        &[list("Survival", vec![file("Hub", "MC.example.net", false)])],
        &[],
    );
    assert_eq!(merged.len(), 1);
    assert_eq!(merged[0].address, "mc.example.net:25565");
    assert_eq!(merged[0].instances, ["Survival"]);
}

#[test]
fn a_server_only_an_instance_lists_is_named_by_its_list_and_is_not_saved() {
    let merged = merge(&[], &[list("Survival", vec![file("Hub", "mc.example.net", false)])], &[]);
    assert_eq!(merged.len(), 1);
    assert_eq!(merged[0].name, "Hub");
    assert!(merged[0].saved.is_none());
    assert!(!merged[0].favorite());
}

#[test]
fn favorites_come_first_and_the_rest_follow_by_name() {
    let merged = merge(
        &[saved(1, "Zeta", "z.example", false), saved(2, "Mid", "m.example", true)],
        &[list(
            "Survival",
            vec![file("alpha", "a.example", false), file("Beta", "b.example", false)],
        )],
        &[],
    );
    let names: Vec<&str> = merged.iter().map(|entry| entry.name.as_str()).collect();
    assert_eq!(names, ["Mid", "alpha", "Beta", "Zeta"]);
}

#[test]
fn each_server_names_each_instance_once_in_the_order_the_instances_come() {
    let merged = merge(
        &[],
        &[
            list("Survival", vec![file("Hub", "mc.example.net", false)]),
            list("Creative", vec![file("Hub", "mc.example.net", false)]),
            list("Modded", vec![file("Hub", "mc.example.net", false), file("Other", "o.example", false)]),
        ],
        &[],
    );
    assert_eq!(merged.len(), 2);
    assert_eq!(merged[0].instances, ["Survival", "Creative", "Modded"]);
    assert_eq!(merged[1].instances, ["Modded"]);
}

#[test]
fn the_first_list_that_holds_an_icon_gives_the_icon() {
    let merged = merge(
        &[],
        &[
            list("A", vec![file("Hub", "mc.example.net", false)]),
            list("B", vec![file("Hub", "mc.example.net", true)]),
        ],
        &[],
    );
    assert!(merged[0].icon.is_some());
    let without = merge(&[], &[list("A", vec![file("Hub", "mc.example.net", false)])], &[]);
    assert!(without[0].icon.is_none());
}

#[test]
fn a_game_on_the_network_is_listed_under_its_title_and_is_not_known() {
    let merged = merge(&[], &[], &[game("Ad Astra World", "192.168.1.42", 54321)]);
    assert_eq!(merged.len(), 1);
    assert_eq!(merged[0].name, "Ad Astra World");
    assert_eq!(merged[0].address, "192.168.1.42:54321");
    assert!(merged[0].announced);
    assert!(!merged[0].known());
    assert!(!merged[0].favorite());
}

#[test]
fn a_game_on_the_default_port_is_listed_without_the_port_and_an_untitled_one_takes_its_address() {
    let merged = merge(&[], &[], &[game("Hub", "10.0.0.2", 25565), game("  ", "10.0.0.3", 54321)]);
    assert_eq!(merged.len(), 2);
    let hub = merged.iter().find(|entry| entry.name == "Hub").expect("the titled game is listed");
    assert_eq!(hub.address, "10.0.0.2");
    let untitled = merged
        .iter()
        .find(|entry| entry.name == "10.0.0.3:54321")
        .expect("the untitled game is named by its address");
    assert_eq!(untitled.address, "10.0.0.3:54321");
}

#[test]
fn a_game_that_a_saved_server_holds_is_one_entry_under_the_saved_name_and_is_announced() {
    let merged = merge(
        &[saved(1, "Alpha", "192.168.1.42:54321", false)],
        &[list("Survival", vec![file("Survival Copy", "192.168.1.42:54321", false)])],
        &[game("Open world", "192.168.1.42", 54321)],
    );
    assert_eq!(merged.len(), 1);
    assert_eq!(merged[0].name, "Alpha");
    assert!(merged[0].announced);
    assert!(merged[0].known());
}

#[test]
fn a_game_that_an_instance_list_holds_keeps_the_list_name() {
    let merged = merge(
        &[],
        &[list("Survival", vec![file("Survival Hub", "10.0.0.2", false)])],
        &[game("Open world", "10.0.0.2", 25565)],
    );
    assert_eq!(merged.len(), 1);
    assert_eq!(merged[0].name, "Survival Hub");
    assert!(merged[0].announced);
    assert_eq!(merged[0].instances, ["Survival"]);
}

#[test]
fn a_game_on_the_network_sorts_by_its_title_among_the_rest() {
    let merged = merge(&[saved(1, "Zeta", "z.example", false)], &[], &[game("alpha", "10.0.0.2", 54321)]);
    let names: Vec<&str> = merged.iter().map(|entry| entry.name.as_str()).collect();
    assert_eq!(names, ["alpha", "Zeta"]);
}
