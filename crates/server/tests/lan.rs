use super::*;

#[test]
fn an_announcement_names_its_game_and_joins_at_the_senders_address_on_its_port() {
    let announcement = parse("[MOTD]Drew's World§r[/MOTD][AD]25565[/AD]", "192.168.1.42").expect("the announcement reads");
    assert_eq!(announcement.title, "Drew's World");
    assert_eq!(
        announcement.address,
        ServerAddress {
            host: "192.168.1.42".to_owned(),
            port: 25565,
        }
    );
}

#[test]
fn a_packet_that_is_not_an_announcement_is_none() {
    assert_eq!(parse("hello", "10.0.0.2"), None);
    assert_eq!(parse("[MOTD]Only a title[/MOTD]", "10.0.0.2"), None, "no port");
    assert_eq!(parse("[MOTD]x[/MOTD][AD]0[/AD]", "10.0.0.2"), None, "port zero");
    assert_eq!(parse("[MOTD]x[/MOTD][AD]port[/AD]", "10.0.0.2"), None, "port not a number");
}
