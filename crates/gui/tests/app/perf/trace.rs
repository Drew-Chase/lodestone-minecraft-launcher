use super::*;

fn parsed(event: &Event) -> serde_json::Value {
    serde_json::from_str(&encode(event)).expect("an encoded event is JSON")
}

#[test]
fn a_span_encodes_as_a_complete_event() {
    let value = parsed(&Event::Span {
        name: "view.root",
        start_us: 1_500,
        dur_us: 250,
        tid: 3,
    });
    assert_eq!(value["name"], "view.root");
    assert_eq!(value["ph"], "X");
    assert_eq!(value["ts"], 1_500);
    assert_eq!(value["dur"], 250);
    assert_eq!(value["tid"], 3);
}

#[test]
fn an_instant_carries_its_value_as_an_argument() {
    let value = parsed(&Event::Instant {
        name: "iced.svg",
        at_us: 9,
        tid: 1,
        value: 73_032,
    });
    assert_eq!(value["ph"], "i");
    assert_eq!(value["args"]["value"], 73_032);
}

#[test]
fn names_are_escaped() {
    let value = parsed(&Event::Span {
        name: "odd \"name\"",
        start_us: 0,
        dur_us: 0,
        tid: 1,
    });
    assert_eq!(value["name"], "odd \"name\"");
}
