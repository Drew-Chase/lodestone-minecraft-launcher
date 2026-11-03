use super::*;
use serde_json::json;

#[test]
fn a_plain_string_loses_its_formatting_codes() {
    assert_eq!(plain(&json!("A §aGreen §cRed")), "A Green Red");
}

#[test]
fn an_object_reads_its_text_and_then_what_it_extends() {
    let component = json!({"text": "Hello ", "extra": [{"text": "§lbold", "color": "gold"}]});
    assert_eq!(plain(&component), "Hello bold");
}

#[test]
fn a_list_of_components_reads_in_order() {
    assert_eq!(plain(&json!(["a", {"text": "b"}, "c"])), "abc");
}

#[test]
fn a_value_that_is_no_text_reads_as_nothing() {
    assert_eq!(plain(&json!(42)), "");
    assert_eq!(plain(&json!({"color": "red"})), "");
}

#[test]
fn a_trailing_section_sign_is_dropped() {
    assert_eq!(strip_codes("end§"), "end");
}
