use super::*;

fn parsed(host: &str, port: u16) -> ServerAddress {
    ServerAddress { host: host.to_owned(), port }
}

#[test]
fn a_bare_host_uses_the_default_port() {
    assert_eq!(ServerAddress::parse("mc.hypixel.net"), Ok(parsed("mc.hypixel.net", DEFAULT_PORT)));
}

#[test]
fn a_port_after_the_host_is_read() {
    assert_eq!(ServerAddress::parse("play.example.com:25566"), Ok(parsed("play.example.com", 25566)));
}

#[test]
fn an_ip_address_with_a_port_is_read() {
    assert_eq!(ServerAddress::parse("192.168.1.42:25565"), Ok(parsed("192.168.1.42", 25565)));
}

#[test]
fn an_ipv6_address_in_brackets_is_read_with_or_without_a_port() {
    assert_eq!(ServerAddress::parse("[2001:db8::1]:19132"), Ok(parsed("2001:db8::1", 19132)));
    assert_eq!(ServerAddress::parse("[::1]"), Ok(parsed("::1", DEFAULT_PORT)));
}

#[test]
fn an_ipv6_address_without_brackets_names_no_port() {
    assert_eq!(ServerAddress::parse("2001:db8::1"), Ok(parsed("2001:db8::1", DEFAULT_PORT)));
}

#[test]
fn surrounding_spaces_are_ignored() {
    assert_eq!(ServerAddress::parse("  mc.example.net  "), Ok(parsed("mc.example.net", DEFAULT_PORT)));
}

#[test]
fn an_empty_address_is_refused() {
    assert_eq!(ServerAddress::parse("   "), Err(AddressError::Empty));
}

#[test]
fn a_port_outside_one_to_65535_or_not_a_number_is_refused() {
    assert_eq!(ServerAddress::parse("host:0"), Err(AddressError::Port));
    assert_eq!(ServerAddress::parse("host:70000"), Err(AddressError::Port));
    assert_eq!(ServerAddress::parse("host:abc"), Err(AddressError::Port));
}

#[test]
fn text_that_is_not_a_host_is_refused() {
    assert_eq!(ServerAddress::parse("bad host"), Err(AddressError::Host));
    assert_eq!(ServerAddress::parse("a..b"), Err(AddressError::Host));
    assert_eq!(ServerAddress::parse("-leading.example"), Err(AddressError::Host));
    assert_eq!(ServerAddress::parse("[::1"), Err(AddressError::Host));
    assert!(ServerAddress::parse("http://example.com").is_err());
}

#[test]
fn an_address_displays_with_its_port_only_when_the_port_is_not_the_default() {
    assert_eq!(parsed("mc.hypixel.net", DEFAULT_PORT).to_string(), "mc.hypixel.net");
    assert_eq!(parsed("a.example", 25566).to_string(), "a.example:25566");
    assert_eq!(parsed("2001:db8::1", 19132).to_string(), "[2001:db8::1]:19132");
    assert_eq!(parsed("::1", DEFAULT_PORT).to_string(), "[::1]");
}

#[test]
fn an_address_reads_back_from_its_display() {
    for text in ["mc.hypixel.net", "a.example:25566", "[2001:db8::1]:19132"] {
        let address = ServerAddress::parse(text).expect("the address reads");
        assert_eq!(ServerAddress::parse(&address.to_string()), Ok(address));
    }
}
