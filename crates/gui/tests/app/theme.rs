use iced::Color;

use super::color::Palette;
use super::{Accent, SurfaceTheme, parse_hex};

/// A color's channels rounded to bytes, so a comparison ignores float noise.
fn rgb8(color: Color) -> [u8; 3] {
    [color.r, color.g, color.b].map(|channel| (channel * 255.0).round() as u8)
}

#[test]
fn the_green_accent_on_the_void_surfaces_is_the_default_palette() {
    assert_eq!(Palette::new(&Accent::Green, SurfaceTheme::Void), Palette::DEFAULT);
}

#[test]
fn an_accent_or_a_surface_theme_changes_the_palette() {
    let violet = Palette::new(&Accent::Violet, SurfaceTheme::Void);
    assert_ne!(violet, Palette::new(&Accent::Violet, SurfaceTheme::Slate));
    assert_ne!(violet, Palette::new(&Accent::Cyan, SurfaceTheme::Void));
    assert_ne!(violet, Palette::DEFAULT);
}

#[test]
fn each_swatch_reads_its_own_color() {
    assert_eq!(rgb8(Accent::Green.color()), [0x22, 0xff, 0x84]);
    assert_eq!(rgb8(Accent::Violet.color()), [0x97, 0x47, 0xff]);
    assert_eq!(rgb8(Accent::Cyan.color()), [0x47, 0xd9, 0xff]);
}

#[test]
fn a_hex_color_is_read_with_or_without_the_hash() {
    assert_eq!(rgb8(parse_hex("#47d9ff").expect("a valid hex color")), [0x47, 0xd9, 0xff]);
    assert_eq!(rgb8(parse_hex("47D9FF").expect("a valid hex color")), [0x47, 0xd9, 0xff]);
}

#[test]
fn a_malformed_hex_color_is_refused() {
    assert_eq!(parse_hex("#47d9f"), None);
    assert_eq!(parse_hex("#47d9fg"), None);
    assert_eq!(parse_hex(""), None);
}

#[test]
fn a_custom_accent_that_is_not_hex_falls_back_to_green() {
    let custom = Accent::Custom {
        hex: "not a color".to_owned(),
    };
    assert_eq!(rgb8(custom.color()), rgb8(Accent::Green.color()));
}
