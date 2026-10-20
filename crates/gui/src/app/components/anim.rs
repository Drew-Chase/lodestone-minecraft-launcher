//! Motion presets — the animation language of the style guide (`§08`).
//!
//! The guide defines two interaction speeds — *micro* (120ms ease) and *lift*
//! (200ms cubic-bezier(.2,.7,.3,1)) — plus long ambient loops that "never
//! compete with content". These map onto [`iced_anim`] spring motions so every
//! control in this module settles with the same feel:
//!
//! | Preset    | Guide reference                  | Used by                          |
//! |-----------|----------------------------------|----------------------------------|
//! | [`micro`] | §08 Micro · 120ms ease           | toggles, hover tints, chevrons   |
//! | [`lift`]  | §08 Lift · 200ms                 | buttons, cards, menus            |
//! | [`glide`] | §08 lift family, longer travel   | tab pills, nav indicator, fills  |

use std::time::Duration;

use iced::Color;
use iced_anim::spring::Motion;

/// Micro interactions — near-instant response, tiny travel (§08 "Micro").
pub fn micro() -> Motion {
    Motion::SNAPPY.with_duration(Duration::from_millis(120))
}

/// Lift — hover elevation and popover entrances (§08 "Lift · 200ms").
pub fn lift() -> Motion {
    Motion::SMOOTH.with_duration(Duration::from_millis(200))
}

/// Glide — longer travels: the tab pill, nav indicator and progress fill.
pub fn glide() -> Motion {
    Motion::SMOOTH.with_duration(Duration::from_millis(300))
}

/// Menus — dropdown/select open and close. Fast and critically damped:
/// overshoot on a floating surface reads as a glitch, not as playfulness.
pub fn menu() -> Motion {
    Motion::SMOOTH.with_duration(Duration::from_millis(180)).with_damping(0.9)
}

/// Multiplies a color's alpha — used for fades (menu open/close, pulse rings).
pub fn fade(color: Color, alpha: f32) -> Color {
    Color { a: color.a * alpha, ..color }
}

/// Linear interpolation between two colors.
pub fn mix(from: Color, to: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color {
        r: from.r + (to.r - from.r) * t,
        g: from.g + (to.g - from.g) * t,
        b: from.b + (to.b - from.b) * t,
        a: from.a + (to.a - from.a) * t,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::color;

    #[test]
    fn fade_multiplies_alpha() {
        let faded = fade(color!(0x22ff84), 0.5);
        assert!((faded.a - 0.5).abs() < 1e-6);
        assert!((faded.r - color!(0x22ff84).r).abs() < 1e-6);
    }

    #[test]
    fn mix_hits_endpoints() {
        let a = color!(0x000000);
        let b = color!(0xffffff);
        assert_eq!(mix(a, b, 0.0), a);
        assert_eq!(mix(a, b, 1.0), b);
        let middle = mix(a, b, 0.5);
        assert!((middle.r - 0.5).abs() < 1e-6);
    }
}
