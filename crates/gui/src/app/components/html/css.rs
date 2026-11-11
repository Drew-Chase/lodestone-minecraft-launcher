//! The inline styles descriptions are written with: colour, size, weight, slant, decoration and alignment, read from an
//! element's `style` attribute and from the older presentational attributes. Sizes are read against the platform's
//! 16px text, drawn at the page's 14px and kept within a range. An author's colour is drawn only where it stands out
//! against the page, since descriptions are often written for a light page.

use iced::Color;

use super::{Align, Style};
use crate::app::theme::parse_hex;

/// The text size a platform sets its descriptions in. The sizes the HTML names are read against it.
const PLATFORM_SIZE: f32 = 16.0;
/// The size the page draws body text at.
pub const TEXT_SIZE: f32 = 14.0;
/// The smallest size text is drawn at.
const MIN_SIZE: f32 = 10.0;
/// The largest size text is drawn at.
const MAX_SIZE: f32 = 36.0;
/// The least contrast with the page an author's colour needs to be drawn: the WCAG ratio for large text.
const MIN_CONTRAST: f32 = 3.0;

/// The value of a property in a `style` attribute. The last declaration of the property wins, as in CSS, and an
/// `!important` mark is dropped.
pub fn declaration<'a>(style: &'a str, property: &str) -> Option<&'a str> {
    style
        .rsplit(';')
        .filter_map(|declaration| declaration.split_once(':'))
        .find(|(name, _)| name.trim().eq_ignore_ascii_case(property))
        .map(|(_, value)| value.trim().trim_end_matches("!important").trim_end())
}

/// Applies the text styles a `style` attribute sets: colour, size, weight, slant and decoration. A value that does not
/// read leaves the style as it was.
pub fn apply(style: &str, to: &mut Style) {
    if let Some(color) = declaration(style, "color").and_then(color) {
        to.color = Some(color);
    }
    if let Some(size) = declaration(style, "font-size").and_then(font_size) {
        to.size = Some(size);
    }
    if let Some(bold) = declaration(style, "font-weight").and_then(bold) {
        to.bold = bold;
    }
    if let Some(italic) = declaration(style, "font-style").and_then(italic) {
        to.italic = italic;
    }
    if let Some(decoration) = declaration(style, "text-decoration-line").or_else(|| declaration(style, "text-decoration")) {
        let decoration = decoration.to_ascii_lowercase();
        if decoration.split_ascii_whitespace().any(|word| word == "none") {
            to.underline = false;
            to.strike = false;
        } else {
            to.underline |= decoration.contains("underline");
            to.strike |= decoration.contains("line-through");
        }
    }
}

/// A CSS colour: `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`, `rgb()`, `rgba()`, or one of the common names. Keywords such
/// as `inherit` and `currentcolor` give `None`, so the colour around the text stays.
pub fn color(value: &str) -> Option<Color> {
    let value = value.trim();
    if let Some(hex) = value.strip_prefix('#') {
        return hex_color(hex);
    }
    let lower = value.to_ascii_lowercase();
    if let Some(arguments) = lower.strip_prefix("rgba(").or_else(|| lower.strip_prefix("rgb(")) {
        return rgb_function(arguments.strip_suffix(')')?);
    }
    named_color(&lower)
}

/// A `font-size` value in the page's pixels: `px`, `pt`, `em`, `rem`, `%` or a size keyword, scaled from the
/// platform's text size to the page's and kept between the smallest and largest sizes drawn.
pub fn font_size(value: &str) -> Option<f32> {
    let value = value.trim().to_ascii_lowercase();
    let platform = match value.as_str() {
        "xx-small" => 9.0,
        "x-small" => 10.0,
        "small" => 13.0,
        "medium" => 16.0,
        "large" => 18.0,
        "x-large" => 24.0,
        "xx-large" => 32.0,
        "xxx-large" => 48.0,
        _ => {
            let (number, unit) = split_unit(&value)?;
            match unit {
                "px" => number,
                "pt" => number * 4.0 / 3.0,
                "em" | "rem" => number * PLATFORM_SIZE,
                "%" => number * PLATFORM_SIZE / 100.0,
                _ => return None,
            }
        }
    };
    Some(scaled(platform))
}

/// The size a `<font size>` names, in the page's pixels: 1 to 7, or a step up or down from 3.
pub fn font_size_attribute(value: &str) -> Option<f32> {
    const SIZES: [f32; 7] = [10.0, 13.0, 16.0, 18.0, 24.0, 32.0, 48.0];
    let value = value.trim();
    let step: i32 = if let Some(up) = value.strip_prefix('+') {
        3 + up.parse::<i32>().ok()?
    } else if let Some(down) = value.strip_prefix('-') {
        3 - down.parse::<i32>().ok()?
    } else {
        value.parse().ok()?
    };
    let index = usize::try_from(step.clamp(1, 7) - 1).ok()?;
    Some(scaled(SIZES[index]))
}

/// A length in pixels, such as a picture's width: `120` or `120px`. Any other unit, and anything under a pixel, gives
/// `None`.
pub fn pixels(value: &str) -> Option<f32> {
    let value = value.trim();
    number(value.strip_suffix("px").unwrap_or(value).trim()).filter(|pixels| *pixels >= 1.0)
}

/// Whether a `font-weight` makes text bold: `bold`, `bolder`, or a weight of 600 or more.
pub fn bold(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "bold" | "bolder" => Some(true),
        "normal" | "lighter" => Some(false),
        weight => number(weight).map(|weight| weight >= 600.0),
    }
}

/// Whether a `font-style` slants text.
pub fn italic(value: &str) -> Option<bool> {
    let value = value.trim().to_ascii_lowercase();
    if value == "italic" || value.starts_with("oblique") {
        Some(true)
    } else if value == "normal" {
        Some(false)
    } else {
        None
    }
}

/// The alignment a `text-align` value or an `align` attribute names. Justified text is drawn at the left.
pub fn align(value: &str) -> Option<Align> {
    match value.trim().to_ascii_lowercase().as_str() {
        "center" | "middle" | "-webkit-center" | "-moz-center" => Some(Align::Center),
        "right" | "end" | "-webkit-right" | "-moz-right" => Some(Align::Right),
        "left" | "start" | "justify" | "-webkit-left" | "-moz-left" => Some(Align::Left),
        _ => None,
    }
}

/// Whether `color` stands out against `background` enough to read: a WCAG contrast ratio of at least 3:1. A colour
/// with transparency is blended over the background first.
pub fn readable(color: Color, background: Color) -> bool {
    let shown = luminance(blend(color, background));
    let behind = luminance(background);
    let (light, dark) = if shown > behind { (shown, behind) } else { (behind, shown) };
    (light + 0.05) / (dark + 0.05) >= MIN_CONTRAST
}

/// `color` laid over `background` by its alpha.
fn blend(color: Color, background: Color) -> Color {
    let alpha = color.a.clamp(0.0, 1.0);
    let mix = |front: f32, back: f32| front * alpha + back * (1.0 - alpha);
    Color::from_rgb(mix(color.r, background.r), mix(color.g, background.g), mix(color.b, background.b))
}

/// A colour's relative luminance, as WCAG defines it.
fn luminance(color: Color) -> f32 {
    let linear = |channel: f32| {
        if channel <= 0.040_45 {
            channel / 12.92
        } else {
            ((channel + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(color.r) + 0.7152 * linear(color.g) + 0.0722 * linear(color.b)
}

/// A size in the platform's pixels, drawn at the page's text size and kept within the sizes drawn.
fn scaled(platform: f32) -> f32 {
    (platform * TEXT_SIZE / PLATFORM_SIZE).clamp(MIN_SIZE, MAX_SIZE)
}

/// A finite number. `NaN` and the infinities, which Rust's parser accepts, do not count.
fn number(text: &str) -> Option<f32> {
    text.parse::<f32>().ok().filter(|value| value.is_finite())
}

/// A number and the unit written after it: `17px` is 17 and `px`. A number with no unit gives `None`.
fn split_unit(value: &str) -> Option<(f32, &str)> {
    let at = value.find(|character: char| !(character.is_ascii_digit() || matches!(character, '.' | '-' | '+')))?;
    Some((number(&value[..at])?, value[at..].trim()))
}

/// A hex colour without its `#`: three, four, six or eight digits, the last of four or eight being the alpha.
fn hex_color(hex: &str) -> Option<Color> {
    if !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let full: String = match hex.len() {
        3 | 4 => hex.chars().flat_map(|digit| [digit, digit]).collect(),
        6 | 8 => hex.to_owned(),
        _ => return None,
    };
    let mut color = parse_hex(&full[..6])?;
    if full.len() == 8 {
        color.a = f32::from(u8::from_str_radix(&full[6..], 16).ok()?) / 255.0;
    }
    Some(color)
}

/// The arguments of `rgb()` or `rgba()`: three channels as 0 to 255 or percentages, and an optional alpha, separated by
/// commas or spaces, the alpha after a `/` in the spaced form.
fn rgb_function(arguments: &str) -> Option<Color> {
    let parts: Vec<&str> = arguments
        .split(|character: char| character == ',' || character == '/' || character.is_ascii_whitespace())
        .filter(|part| !part.is_empty())
        .collect();
    if parts.len() != 3 && parts.len() != 4 {
        return None;
    }
    let channel = |part: &str| match part.strip_suffix('%') {
        Some(percent) => number(percent).map(|percent| (percent / 100.0).clamp(0.0, 1.0)),
        None => number(part).map(|value| (value / 255.0).clamp(0.0, 1.0)),
    };
    let alpha = match parts.get(3) {
        Some(part) => match part.strip_suffix('%') {
            Some(percent) => number(percent)? / 100.0,
            None => number(part)?,
        },
        None => 1.0,
    };
    Some(Color {
        r: channel(parts[0])?,
        g: channel(parts[1])?,
        b: channel(parts[2])?,
        a: alpha.clamp(0.0, 1.0),
    })
}

/// The common colour names.
fn named_color(name: &str) -> Option<Color> {
    let value: u32 = match name {
        "black" => 0x000000,
        "white" => 0xFFFFFF,
        "red" => 0xFF0000,
        "green" => 0x008000,
        "blue" => 0x0000FF,
        "yellow" => 0xFFFF00,
        "orange" => 0xFFA500,
        "purple" => 0x800080,
        "gray" | "grey" => 0x808080,
        "silver" => 0xC0C0C0,
        "maroon" => 0x800000,
        "navy" => 0x000080,
        "teal" => 0x008080,
        "olive" => 0x808000,
        "lime" => 0x00FF00,
        "aqua" | "cyan" => 0x00FFFF,
        "fuchsia" | "magenta" => 0xFF00FF,
        "pink" => 0xFFC0CB,
        "gold" => 0xFFD700,
        "brown" => 0xA52A2A,
        "violet" => 0xEE82EE,
        "indigo" => 0x4B0082,
        "crimson" => 0xDC143C,
        "coral" => 0xFF7F50,
        "salmon" => 0xFA8072,
        "tomato" => 0xFF6347,
        "orchid" => 0xDA70D6,
        "plum" => 0xDDA0DD,
        "khaki" => 0xF0E68C,
        "turquoise" => 0x40E0D0,
        "skyblue" => 0x87CEEB,
        "steelblue" => 0x4682B4,
        "royalblue" => 0x4169E1,
        "darkred" => 0x8B0000,
        "darkgreen" => 0x006400,
        "darkblue" => 0x00008B,
        "darkorange" => 0xFF8C00,
        "lightblue" => 0xADD8E6,
        "lightgreen" => 0x90EE90,
        "lightgray" | "lightgrey" => 0xD3D3D3,
        "darkgray" | "darkgrey" => 0xA9A9A9,
        _ => return None,
    };
    let [_, red, green, blue] = value.to_be_bytes();
    Some(Color::from_rgb8(red, green, blue))
}

#[cfg(test)]
#[path = "../../../../tests/app/components/html/css.rs"]
mod tests;
