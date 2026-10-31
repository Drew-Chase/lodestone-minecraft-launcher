use super::*;

#[test]
fn vector_marks_are_svg_documents() {
    for mark in [
        Icon::selfhst().minecraft(),
        Icon::material_icon_theme().minecraft_fabric(),
        Icon::at_icons().anvil(),
    ] {
        assert!(mark.trim_start().starts_with("<svg"), "every vector mark must be an svg document");
    }
}

#[test]
fn the_forge_anvil_is_monochrome() {
    // The tint paints the whole svg, so the anvil has to draw in currentColor alone.
    assert!(Icon::at_icons().anvil().contains("currentColor"));
}

#[test]
fn raster_marks_decode_as_squares() {
    for (name, bytes) in [("neoforge", NEOFORGE_PNG), ("quilt", QUILT_PNG)] {
        let decoded = ::image::load_from_memory(bytes).unwrap_or_else(|error| panic!("{name} must decode: {error}"));
        assert_eq!(decoded.width(), decoded.height(), "{name} must be square");
    }
}

#[test]
fn reframe_replaces_the_default_24_unit_box_with_the_given_frame() {
    let marks = [
        (Icon::material_icon_theme().minecraft_fabric(), (1, 1, 13, 14), r#"viewBox="1 1 13 14""#),
        (Icon::at_icons().anvil(), (0, 0, 16, 16), r#"viewBox="0 0 16 16""#),
    ];
    for (markup, frame, expected) in marks {
        let boxed = String::from_utf8(reframe(markup, frame)).expect("svg markup is utf-8");
        assert!(boxed.contains(expected), "the mark must sit in its own frame");
        assert!(!boxed.contains(r#"viewBox="0 0 24 24""#), "the 24 unit box must be gone");
        assert!(boxed.contains("<path"), "the glyph must survive the reframe");
    }
}

#[test]
fn every_loader_wears_its_own_hue() {
    let loaders = [Loader::Vanilla, Loader::Fabric, Loader::Forge, Loader::NeoForge, Loader::Quilt];
    for (index, loader) in loaders.iter().enumerate() {
        for other in &loaders[index + 1..] {
            assert_ne!(hue(*loader), hue(*other), "{loader:?} and {other:?} share a hue");
        }
    }
}
