//! Motion presets — the animation language of the style guide (`§08`).
//!
//! The guide defines two interaction speeds — *micro* (120ms ease) and *lift*
//! (200ms cubic-bezier(.2,.7,.3,1)) — plus long ambient loops that "never
//! compete with content". Every preset is a time-based [`Easing`] transition:
//!
//! **Why not springs?** `iced_anim`'s spring integrator is a semi-implicit
//! Euler step with the delta-time clamped at 33ms (`MAX_DURATION`). That clamp
//! makes springs numerically unstable for fast responses — the damping term
//! `|1 − ζω·dt|` crosses 1 at dt ≈ 22ms for a 120ms response, so any slow
//! frame amplifies the velocity and the animation oscillates forever. The
//! library's own 500ms default sits safely inside the stable envelope; the
//! guide's 120–200ms timings do not. Transitions integrate `progress` against
//! a fixed duration (clamped to [0, 1] and lerped between endpoints), so they
//! terminate exactly at any frame rate — the correct primitive for these
//! CSS-style timings.
//!
//! | Preset    | Guide reference                        | Used by                          |
//! |-----------|----------------------------------------|----------------------------------|
//! | [`micro`] | §08 Micro · 120ms ease                 | toggles, hover tints, chevrons   |
//! | [`lift`]  | §08 Lift · 200ms cubic-bezier(.2,.7,.3,1) | buttons, cards, menus        |
//! | [`glide`] | §08 lift family, longer travel         | tab pills, nav indicator, fills  |

use std::time::Duration;

use iced::Color;
use iced_anim::transition::bezier::Bezier;
use iced_anim::transition::{Curve, Easing};

/// Micro interactions — near-instant response, tiny travel (§08 "Micro").
pub fn micro() -> Easing {
    Easing::EASE_OUT.with_duration(Duration::from_millis(120))
}

/// Lift — hover elevation and popover entrances (§08 "Lift · 200ms
/// cubic-bezier(.2,.7,.3,1)"). The guide's exact curve.
pub fn lift() -> Easing {
    Easing::new(Curve::Bezier(Bezier::new(0.2, 0.7, 0.3, 1.0))).with_duration(Duration::from_millis(200))
}

/// Glide — longer travels: the tab pill, nav indicator and progress fill.
pub fn glide() -> Easing {
    Easing::EASE_IN_OUT.with_duration(Duration::from_millis(300))
}

/// Menus — dropdown/select open and close (§08 Lift family). Fast fade and
/// slide; time-based so a popover always lands exactly, at any frame rate.
pub fn menu() -> Easing {
    Easing::EASE_OUT.with_duration(Duration::from_millis(180))
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
    fn presets_are_time_based_transitions() {
        // All presets must be easings — springs are numerically unstable at
        // these durations (see the module docs).
        let micro = micro();
        let lift = lift();
        let glide = glide();
        let menu = menu();

        assert_eq!(micro.duration, Duration::from_millis(120));
        assert_eq!(lift.duration, Duration::from_millis(200));
        assert_eq!(glide.duration, Duration::from_millis(300));
        assert_eq!(menu.duration, Duration::from_millis(180));
    }

    #[test]
    fn lift_matches_the_guide_curve() {
        // §08 — cubic-bezier(.2,.7,.3,1).
        let lift = lift();
        assert!(matches!(lift.curve, Curve::Bezier(_)));
    }

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
