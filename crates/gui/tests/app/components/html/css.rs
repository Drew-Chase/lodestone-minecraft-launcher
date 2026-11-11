use super::*;

/// The Void surface theme's panel colour (`--bg-2`), which the view checks colours against.
fn panel() -> Color {
    Color::from_rgb8(0x15, 0x18, 0x1C)
}

#[test]
fn a_declaration_reads_by_name_and_the_last_one_wins() {
    let style = "COLOR: #e8a33d; font-size:17px;color: red !important";
    assert_eq!(declaration(style, "color"), Some("red"));
    assert_eq!(declaration(style, "font-size"), Some("17px"));
    assert_eq!(declaration(style, "font-weight"), None);
}

#[test]
fn colours_read_in_hex_rgb_and_by_name() {
    assert_eq!(color("#9d5cff"), Some(Color::from_rgb8(0x9d, 0x5c, 0xff)));
    assert_eq!(color("#fa0"), Some(Color::from_rgb8(0xff, 0xaa, 0x00)));
    assert_eq!(color("rgb(157, 92, 255)"), Some(Color::from_rgb8(157, 92, 255)));
    assert_eq!(color("RGB(100%, 0%, 0%)"), Some(Color::from_rgb8(255, 0, 0)));
    assert_eq!(color("orange"), Some(Color::from_rgb8(0xFF, 0xA5, 0x00)));
    let translucent = color("rgba(255 255 255 / 50%)").expect("a colour");
    assert!((translucent.a - 0.5).abs() < f32::EPSILON);
    assert!((color("#ffffff80").expect("a colour").a - 128.0 / 255.0).abs() < f32::EPSILON);
}

#[test]
fn colour_keywords_and_broken_values_give_nothing() {
    for value in ["inherit", "currentcolor", "transparent", "#12", "#ggg", "rgb(1, 2)", "rgb(nan, 0, 0)", ""] {
        assert_eq!(color(value), None, "{value}");
    }
}

#[test]
fn a_colour_made_for_a_light_page_is_not_readable_on_the_panel() {
    assert!(!readable(Color::from_rgb8(0x20, 0x2c, 0x44), panel()));
    assert!(!readable(Color::BLACK, panel()));
}

#[test]
fn bright_author_colours_are_readable_on_the_panel() {
    for hex in [0x9d5cff_u32, 0xe8a33d, 0xe03e2d, 0xffffff] {
        let [_, red, green, blue] = hex.to_be_bytes();
        assert!(readable(Color::from_rgb8(red, green, blue), panel()), "{hex:06x}");
    }
}

#[test]
fn a_faint_colour_is_judged_as_it_shows_over_the_panel() {
    assert!(!readable(Color::from_rgba8(255, 255, 255, 0.1), panel()));
    assert!(readable(Color::from_rgba8(255, 255, 255, 0.9), panel()));
}

#[test]
fn font_sizes_scale_to_the_page_and_stay_in_range() {
    assert_eq!(font_size("16px"), Some(14.0));
    assert_eq!(font_size("40px"), Some(35.0));
    assert_eq!(font_size("12pt"), Some(14.0));
    assert_eq!(font_size("1.5em"), Some(21.0));
    assert_eq!(font_size("150%"), Some(21.0));
    assert_eq!(font_size("large"), Some(15.75));
    assert_eq!(font_size("96px"), Some(36.0));
    assert_eq!(font_size("2px"), Some(10.0));
    assert_eq!(font_size("17"), None);
    assert_eq!(font_size("calc(1em + 2px)"), None);
}

#[test]
fn the_font_size_attribute_steps_from_three() {
    assert_eq!(font_size_attribute("3"), Some(14.0));
    assert_eq!(font_size_attribute("+2"), Some(21.0));
    assert_eq!(font_size_attribute("-1"), font_size_attribute("2"));
    assert_eq!(font_size_attribute("9"), font_size_attribute("7"));
    assert_eq!(font_size_attribute("big"), None);
}

#[test]
fn pixels_read_bare_or_in_px_only() {
    assert_eq!(pixels("156"), Some(156.0));
    assert_eq!(pixels(" 638px "), Some(638.0));
    assert_eq!(pixels("50%"), None);
    assert_eq!(pixels("0"), None);
    assert_eq!(pixels("inf"), None);
}

#[test]
fn weight_slant_and_alignment_read() {
    assert_eq!(bold("700"), Some(true));
    assert_eq!(bold("Bold"), Some(true));
    assert_eq!(bold("400"), Some(false));
    assert_eq!(italic("oblique 10deg"), Some(true));
    assert_eq!(italic("normal"), Some(false));
    assert_eq!(align("Center"), Some(Align::Center));
    assert_eq!(align("justify"), Some(Align::Left));
    assert_eq!(align("end"), Some(Align::Right));
    assert_eq!(align("inherit"), None);
}

#[test]
fn a_style_attribute_applies_its_text_styles() {
    let mut style = Style::default();
    apply(
        "color:#9d5cff;font-size:40px;font-weight:bold;font-style:italic;text-decoration:underline line-through",
        &mut style,
    );
    assert_eq!(style.color, Some(Color::from_rgb8(0x9d, 0x5c, 0xff)));
    assert_eq!(style.size, Some(35.0));
    assert!(style.bold && style.italic && style.underline && style.strike);
    apply("text-decoration: none; color: inherit", &mut style);
    assert!(!style.underline && !style.strike);
    assert_eq!(
        style.color,
        Some(Color::from_rgb8(0x9d, 0x5c, 0xff)),
        "inherit keeps the colour around it"
    );
}
