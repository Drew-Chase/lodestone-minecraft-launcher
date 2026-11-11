use super::*;
use crate::app::database::servers::SavedServer;
use crate::app::pages::instance::files::ServerFile;
use chrono::Local;
use lodestone_server::address::ServerAddress;
use lodestone_server::status::Status;
use std::time::Duration;

/// A saved server numbered `id`, called `name` at `address`.
fn saved(id: i64, name: &str, address: &str, favorite: bool) -> SavedServer {
    SavedServer {
        id,
        name: name.to_owned(),
        address: address.to_owned(),
        favorite,
        added_at: Local::now(),
    }
}

/// A ping that answered with `online` players.
fn reply(online: u32) -> Reply {
    Reply {
        status: Status {
            version: "1.21.4".to_owned(),
            protocol: 769,
            online,
            max: 20,
            motd: String::new(),
            favicon: None,
            latency: Duration::from_millis(30),
        },
        icon: None,
    }
}

/// A game announced at `host` on `port`, titled `title`.
fn game(title: &str, host: &str, port: u16) -> Announcement {
    Announcement {
        title: title.to_owned(),
        address: ServerAddress { host: host.to_owned(), port },
    }
}

#[test]
fn the_all_tab_keeps_every_server_and_favorites_keep_only_the_marked_ones() {
    let mut state = State::default();
    state.set_saved(vec![saved(1, "Alpha", "a.example", false), saved(2, "Beta", "b.example", true)]);
    assert_eq!(state.shown_servers().len(), 2);
    state.update(Message::Tab(Tab::Favorites));
    let shown: Vec<&str> = state.shown_servers().iter().map(|server| server.name.as_str()).collect();
    assert_eq!(shown, ["Beta"]);
}

#[test]
fn the_search_keeps_servers_whose_name_or_address_contains_it() {
    let mut state = State::default();
    state.set_saved(vec![saved(1, "Alpha", "a.example", false), saved(2, "Beta", "play.beta.net", false)]);
    state.update(Message::Query("BETA".to_owned()));
    assert_eq!(
        state.shown_servers().iter().map(|server| server.name.as_str()).collect::<Vec<_>>(),
        ["Beta"]
    );
    state.update(Message::Query("a.example".to_owned()));
    assert_eq!(
        state.shown_servers().iter().map(|server| server.name.as_str()).collect::<Vec<_>>(),
        ["Alpha"]
    );
}

#[test]
fn a_server_an_instance_lists_is_listed_and_pinged_with_the_saved_ones() {
    let mut state = State::default();
    state.set_saved(vec![saved(1, "Alpha", "a.example", false)]);
    state.set_instances(vec![InstanceServers {
        instance: "Survival".to_owned(),
        servers: vec![ServerFile {
            name: "Hub".to_owned(),
            address: "hub.example".to_owned(),
            icon: None,
        }],
    }]);
    assert_eq!(state.addresses(), ["a.example", "hub.example"]);
    assert_eq!(state.shown_servers().len(), 2);
    assert_eq!(
        state.listed_at("hub.example").map(|server| server.instances.clone()),
        Some(vec!["Survival".to_owned()])
    );
}

#[test]
fn the_favorites_tab_keeps_a_listed_server_only_while_the_launcher_marks_it() {
    let mut state = State::default();
    state.set_saved(vec![saved(1, "Alpha", "a.example", true)]);
    state.set_instances(vec![InstanceServers {
        instance: "Survival".to_owned(),
        servers: vec![ServerFile {
            name: "Hub".to_owned(),
            address: "hub.example".to_owned(),
            icon: None,
        }],
    }]);
    state.update(Message::Tab(Tab::Favorites));
    let shown: Vec<&str> = state.shown_servers().iter().map(|server| server.name.as_str()).collect();
    assert_eq!(shown, ["Alpha"]);
}

#[test]
fn a_server_the_launcher_has_not_saved_offers_join_copy_and_favorite_only() {
    assert_eq!(RowAction::menu(false), [RowAction::Join, RowAction::Copy, RowAction::Favorite]);
    assert_eq!(RowAction::menu(true), RowAction::MENU);
}

#[test]
fn the_summary_counts_the_servers_that_answered_and_their_players() {
    let mut state = State::default();
    state.set_saved(vec![
        saved(1, "A", "a.example", false),
        saved(2, "B", "b.example", false),
        saved(3, "C", "c.example", false),
    ]);
    state.probed("a.example".to_owned(), Ok(reply(3)));
    state.probed("b.example".to_owned(), Ok(reply(5)));
    state.probed("c.example".to_owned(), Err("refused".to_owned()));
    assert_eq!(state.online_summary(), (2, 8));
}

#[test]
fn a_server_keeps_its_last_answer_while_it_waits_again() {
    let mut state = State::default();
    state.probed("a.example".to_owned(), Ok(reply(3)));
    state.begin_probe("a.example");
    assert!(matches!(state.probe("a.example"), Some(Probe::Online(_))));
    state.begin_probe("new.example");
    assert!(matches!(state.probe("new.example"), Some(Probe::Waiting)));
}

#[test]
fn a_game_heard_twice_is_listed_once_and_a_quiet_one_is_forgotten() {
    let mut state = State::default();
    let start = Instant::now();
    state.heard(game("World", "10.0.0.2", 25565), start);
    state.heard(game("World", "10.0.0.2", 25565), start + Duration::from_secs(2));
    assert_eq!(state.lan().len(), 1);
    state.forget_quiet_games(start + Duration::from_secs(60));
    assert!(state.lan().is_empty());
}

#[test]
fn a_game_heard_on_the_network_is_listed_with_the_rest_and_not_among_the_favorites() {
    let mut state = State::default();
    state.set_saved(vec![saved(1, "Alpha", "a.example", true)]);
    state.heard(game("World", "10.0.0.2", 54321), Instant::now());
    let shown: Vec<&str> = state.shown_servers().iter().map(|server| server.name.as_str()).collect();
    assert_eq!(shown, ["Alpha", "World"]);
    state.update(Message::Tab(Tab::Favorites));
    let favorites: Vec<&str> = state.shown_servers().iter().map(|server| server.name.as_str()).collect();
    assert_eq!(favorites, ["Alpha"]);
}

#[test]
fn a_game_that_goes_quiet_is_no_longer_listed_or_pinged() {
    let mut state = State::default();
    let start = Instant::now();
    state.heard(game("World", "10.0.0.2", 54321), start);
    state.probed("10.0.0.2:54321".to_owned(), Ok(reply(2)));
    assert_eq!(state.addresses(), ["10.0.0.2:54321"]);
    state.forget_quiet_games(start + Duration::from_secs(60));
    assert!(state.listed().is_empty());
    assert!(state.addresses().is_empty());
    assert!(state.probe("10.0.0.2:54321").is_none());
}

#[test]
fn the_summary_counts_only_the_servers_still_listed() {
    let mut state = State::default();
    state.set_saved(vec![saved(1, "A", "a.example", false)]);
    state.probed("a.example".to_owned(), Ok(reply(3)));
    state.probed("removed.example".to_owned(), Ok(reply(7)));
    assert_eq!(state.online_summary(), (1, 3));
}

#[test]
fn the_row_menu_lists_join_first_and_remove_last() {
    assert_eq!(RowAction::MENU[0], RowAction::Join);
    assert_eq!(RowAction::MENU[RowAction::MENU.len() - 1], RowAction::Remove);
}

#[test]
fn only_an_address_that_reads_can_be_saved_from_the_search() {
    assert!(is_address("mc.hypixel.net"));
    assert!(!is_address("not a server"));
}
