use super::*;

#[test]
fn known_ids_resolve_and_unknown_ids_fall_back_to_the_first_entry() {
    assert_eq!(preset("indigo").id, "indigo");
    assert_eq!(preset("no-such-background").id, BACKGROUNDS[0].id);
    assert_eq!(symbol("diamond").id, "diamond");
    assert_eq!(symbol("no-such-symbol").id, SYMBOLS[0].id);
}

#[test]
fn every_symbol_is_an_svg_document() {
    for symbol in SYMBOLS {
        assert!(symbol.svg.trim_start().starts_with("<svg"), "{} must be an svg", symbol.id);
    }
}

#[test]
fn random_picks_come_from_the_catalog() {
    for _ in 0..32 {
        let (background, symbol_id) = random();
        assert!(BACKGROUNDS.iter().any(|preset| preset.id == background));
        assert!(SYMBOLS.iter().any(|symbol| symbol.id == symbol_id));
    }
}
