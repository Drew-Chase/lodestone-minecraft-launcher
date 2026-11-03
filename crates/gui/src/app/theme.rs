//! The interface's colors, fonts and icon helpers.
//!
//! The colors come from the active [`color::Palette`]. The accent sets the `Primary` scale, and the surface theme sets
//! the canvas, the panels and the hairlines. Views read the palette on every draw, so an Appearance change shows on the
//! next frame.

use iced::Color;
use serde::{Deserialize, Serialize};

/// The accent the interface highlights with: the launcher's green, one of the swatches the Appearance tab offers, or a
/// custom `#rrggbb` value.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Accent {
    #[default]
    Green,
    Violet,
    Amber,
    Cyan,
    Pink,
    Orange,
    /// A `#rrggbb` color the player typed in.
    Custom {
        hex: String,
    },
}

impl Accent {
    /// The color the accent highlights with. A custom value that is not `#rrggbb` falls back to green.
    pub fn color(&self) -> Color {
        match self {
            Accent::Green => iced::color!(0x22FF84),
            Accent::Violet => iced::color!(0x9747FF),
            Accent::Amber => iced::color!(0xFFB545),
            Accent::Cyan => iced::color!(0x47D9FF),
            Accent::Pink => iced::color!(0xFF5EC8),
            Accent::Orange => iced::color!(0xFF6B3A),
            Accent::Custom { hex } => parse_hex(hex).unwrap_or(iced::color!(0x22FF84)),
        }
    }
}

/// The surface treatment: the canvas, the panels and the hairlines.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceTheme {
    #[default]
    Void,
    Slate,
    Obsidian,
}

/// Reads a `#rrggbb` color. The `#` is optional. Returns `None` unless exactly six hex digits follow.
pub fn parse_hex(hex: &str) -> Option<Color> {
    let digits = hex.strip_prefix('#').unwrap_or(hex);
    if digits.len() != 6 || !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let value = u32::from_str_radix(digits, 16).ok()?;
    Some(Color::from_rgb8((value >> 16) as u8, (value >> 8) as u8, value as u8))
}

pub mod color {
    use std::sync::{PoisonError, RwLock};

    use iced::{Color, color};
    use iced_color_extension::rgb;

    use super::{Accent, SurfaceTheme};

    /// The accent family: its base and the shades the style guide derives from it.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct Scale {
        /// `--mc-green`: primary actions, active nav, toggles, progress and online.
        base: Color,
        /// `--mc-green-dim`: gradient stops and pressed states.
        dim: Color,
        /// `--mc-green-deep`: tinted fills and badge grounds.
        deep: Color,
        /// `--mc-green-glow`: glows under CTAs, neon text and active bars.
        glow: Color,
        /// The accent at 12%: a tinted chip or fill.
        tint: Color,
        /// On-accent ink: text and icons placed on the accent.
        on: Color,
        /// The primary button's hover fill.
        hover: Color,
        /// The primary button's disabled fill.
        disabled: Color,
    }

    impl Scale {
        /// The launcher's own green, with the shades the style guide specifies.
        pub const GREEN: Scale = Scale {
            base: color!(0x22FF84),
            dim: color!(0x18C968),
            deep: color!(0x0A5C33),
            glow: rgb!(34, 255, 132, 35%),
            tint: rgb!(34, 255, 132, 12%),
            on: color!(0x062814),
            hover: color!(0x24FF8C),
            disabled: rgb!(34, 255, 132, 40%),
        };

        /// The scale of another accent, with each shade derived from its base.
        fn from_base(base: Color) -> Scale {
            Scale {
                base,
                dim: shade(base, 0.8),
                deep: shade(base, 0.36),
                glow: fade(base, 0.35),
                tint: fade(base, 0.12),
                on: shade(base, 0.16),
                hover: shade(base, 1.06),
                disabled: fade(base, 0.40),
            }
        }
    }

    /// `base` with each channel scaled by `factor`, kept within the range of a channel.
    fn shade(base: Color, factor: f32) -> Color {
        Color {
            r: (base.r * factor).min(1.0),
            g: (base.g * factor).min(1.0),
            b: (base.b * factor).min(1.0),
            a: base.a,
        }
    }

    /// `base` at the given opacity.
    fn fade(base: Color, alpha: f32) -> Color {
        Color { a: alpha, ..base }
    }

    /// The surfaces: the canvas, the layers over it, and the hairlines between them.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct Surfaces {
        /// `--bg-0`: the app canvas.
        bg0: Color,
        /// `--bg-1`: the sidebar and the titlebar.
        bg1: Color,
        /// `--bg-2`: panels.
        bg2: Color,
        /// `--bg-3`: inputs and menus.
        bg3: Color,
        /// `--bg-4`: hover and popovers.
        bg4: Color,
        /// `--line`: hairline borders.
        line: Color,
        /// `--line-strong`: emphasized borders.
        line_strong: Color,
    }

    impl Surfaces {
        /// The launcher's own surfaces, built up from black one step per layer.
        const VOID: Surfaces = Surfaces {
            bg0: color!(0x08090A),
            bg1: color!(0x0E1012),
            bg2: color!(0x15181C),
            bg3: color!(0x1D2127),
            bg4: color!(0x272C34),
            line: rgb!(255, 255, 255, 7%),
            line_strong: rgb!(255, 255, 255, 12%),
        };

        /// Blue-grey surfaces.
        const SLATE: Surfaces = Surfaces {
            bg0: color!(0x0B0E13),
            bg1: color!(0x10141B),
            bg2: color!(0x161C25),
            bg3: color!(0x1E2631),
            bg4: color!(0x283140),
            line: rgb!(255, 255, 255, 7%),
            line_strong: rgb!(255, 255, 255, 12%),
        };

        /// Plum-black surfaces.
        const OBSIDIAN: Surfaces = Surfaces {
            bg0: color!(0x0A0609),
            bg1: color!(0x110B12),
            bg2: color!(0x190F18),
            bg3: color!(0x231623),
            bg4: color!(0x2F1E30),
            line: rgb!(255, 255, 255, 7%),
            line_strong: rgb!(255, 255, 255, 12%),
        };

        fn of(theme: SurfaceTheme) -> Surfaces {
            match theme {
                SurfaceTheme::Void => Surfaces::VOID,
                SurfaceTheme::Slate => Surfaces::SLATE,
                SurfaceTheme::Obsidian => Surfaces::OBSIDIAN,
            }
        }
    }

    /// Every color the interface draws with, for one accent and one surface theme.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct Palette {
        scale: Scale,
        surfaces: Surfaces,
    }

    impl Palette {
        /// The launcher's own look: Lode Green on the Void surfaces.
        pub const DEFAULT: Palette = Palette {
            scale: Scale::GREEN,
            surfaces: Surfaces::VOID,
        };

        /// The palette for an accent and a surface theme. The green accent keeps its own shades.
        pub fn new(accent: &Accent, theme: SurfaceTheme) -> Palette {
            let scale = match accent {
                Accent::Green => Scale::GREEN,
                other => Scale::from_base(other.color()),
            };
            Palette {
                scale,
                surfaces: Surfaces::of(theme),
            }
        }
    }

    /// The palette the colors are read from. It starts as the launcher's own look, and [`set`] replaces it.
    static ACTIVE: RwLock<Palette> = RwLock::new(Palette::DEFAULT);

    /// Makes `palette` the one every color function returns. Views read colors on each draw, so the next frame uses it.
    pub fn set(palette: Palette) {
        *ACTIVE.write().unwrap_or_else(PoisonError::into_inner) = palette;
    }

    /// Reads a value from the active palette.
    fn with<T>(read: impl FnOnce(&Palette) -> T) -> T {
        let palette = ACTIVE.read().unwrap_or_else(PoisonError::into_inner);
        read(&palette)
    }

    /// The canvas and the panel fill of a surface theme, for its preview.
    pub fn surface_preview(theme: SurfaceTheme) -> (Color, Color) {
        let surfaces = Surfaces::of(theme);
        (surfaces.bg0, surfaces.bg2)
    }

    /// `--ink-0`: headlines and active labels.
    pub fn text_headline() -> Color {
        color!(0xFFFFFF)
    }

    /// `--ink-1`: body copy, the app-wide default text color.
    pub fn text_default() -> Color {
        rgb!(255, 255, 255, 92%)
    }

    /// `--ink-2`: secondary text, descriptions and meta.
    pub fn text_muted() -> Color {
        rgb!(255, 255, 255, 68%)
    }

    /// `--ink-3`: placeholders, captions and idle icons.
    pub fn text_placeholder() -> Color {
        rgb!(255, 255, 255, 45%)
    }

    /// `--ink-4`: disabled only, never for readable text.
    pub fn text_disabled() -> Color {
        rgb!(255, 255, 255, 28%)
    }

    /// `--bg-0`: the app canvas.
    pub fn surface_bg0() -> Color {
        with(|palette| palette.surfaces.bg0)
    }

    /// `--bg-1`: the sidebar.
    pub fn surface_bg1() -> Color {
        with(|palette| palette.surfaces.bg1)
    }

    /// `--bg-2`: panels.
    pub fn surface_bg2() -> Color {
        with(|palette| palette.surfaces.bg2)
    }

    /// `--bg-3`: inputs and menus.
    pub fn surface_bg3() -> Color {
        with(|palette| palette.surfaces.bg3)
    }

    /// `--bg-4`: hover and popovers.
    pub fn surface_bg4() -> Color {
        with(|palette| palette.surfaces.bg4)
    }

    /// `--line`: hairline borders.
    pub fn surface_line() -> Color {
        with(|palette| palette.surfaces.line)
    }

    /// `--line-strong`: emphasized borders.
    pub fn surface_line_strong() -> Color {
        with(|palette| palette.surfaces.line_strong)
    }

    /// The whole application's background: the app canvas.
    pub fn surface_application() -> Color {
        surface_bg0()
    }

    /// Titlebar chrome. Only the Windows titlebar draws it, so other targets leave it unused.
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    pub fn surface_titlebar() -> Color {
        surface_bg1()
    }

    /// `--mc-green`: primary CTA, active nav, toggles, progress and online.
    pub fn primary() -> Color {
        with(|palette| palette.scale.base)
    }

    /// `--mc-green-dim`: gradient stops and pressed states.
    pub fn primary_dim() -> Color {
        with(|palette| palette.scale.dim)
    }

    /// `--mc-green-glow`: glows under CTAs, neon text and active bars.
    pub fn primary_glow() -> Color {
        with(|palette| palette.scale.glow)
    }

    /// The accent at 12%: a tinted chip or fill.
    pub fn primary_tint() -> Color {
        with(|palette| palette.scale.tint)
    }

    /// On-accent ink: text and icons placed on the accent.
    pub fn primary_on() -> Color {
        with(|palette| palette.scale.on)
    }

    /// Lightened violet: accent text on tinted fills.
    pub fn violet_light() -> Color {
        color!(0xB689FF)
    }

    /// Violet at 14%.
    pub fn violet_tint() -> Color {
        rgb!(151, 71, 255, 14%)
    }

    /// Amber, for warnings, available updates and snapshots.
    pub fn amber() -> Color {
        color!(0xFFB545)
    }

    /// Amber at 14%.
    pub fn amber_tint() -> Color {
        rgb!(255, 181, 69, 14%)
    }

    /// Cyan, for info, servers, co-op and sync.
    pub fn cyan() -> Color {
        color!(0x47D9FF)
    }

    /// Cyan at 14%.
    pub fn cyan_tint() -> Color {
        rgb!(71, 217, 255, 14%)
    }

    /// Pink, for errors, crashes, destructive actions and favorites.
    pub fn pink() -> Color {
        color!(0xFF5EC8)
    }

    /// Pink at 14%.
    pub fn pink_tint() -> Color {
        rgb!(255, 94, 200, 14%)
    }

    /// Button palettes for [`crate::app::components::icon_button`], from the style guide's component specs.
    pub mod buttons {
        use iced::Color;
        use iced_color_extension::{hsl, rgb};

        /// `.btn-ghost`: a `--ink-1` label.
        pub fn secondary_foreground() -> Color {
            super::text_default()
        }

        /// `.btn-ghost`: white 4% fill.
        pub fn secondary_background() -> Color {
            rgb!(255, 255, 255, 4%)
        }

        /// Hover: white 8%.
        pub fn secondary_background_hover() -> Color {
            rgb!(255, 255, 255, 8%)
        }

        /// Destructive control (window close, discard): the idle icon is `--ink-2`, per the `.btn-icon` spec.
        pub fn danger_foreground() -> Color {
            super::text_muted()
        }

        /// The hover icon is `--ink-0`.
        pub fn danger_foreground_hover() -> Color {
            super::text_headline()
        }

        /// Ghost idle fill.
        pub fn danger_background() -> Color {
            rgb!(255, 255, 255, 4%)
        }

        /// Hover: `--pink`.
        pub fn danger_background_hover() -> Color {
            hsl!(356, 86%, 49%)
        }
    }
}

pub mod fonts {
    use iced::Font;
    use iced::font::{Family, Weight};

    /// Inter — the interface sans (`--font`).
    pub struct Inter;
    impl Inter {
        pub const REGULAR: Font = Font {
            family: Family::Name("Inter"),
            weight: Weight::Normal,
            ..Font::DEFAULT
        };
        pub const MEDIUM: Font = Font {
            family: Family::Name("Inter"),
            weight: Weight::Medium,
            ..Font::DEFAULT
        };
        pub const SEMI_BOLD: Font = Font {
            family: Family::Name("Inter"),
            weight: Weight::Semibold,
            ..Font::DEFAULT
        };
        pub const BOLD: Font = Font {
            family: Family::Name("Inter"),
            weight: Weight::Bold,
            ..Font::DEFAULT
        };
        pub const EXTRA_BOLD: Font = Font {
            family: Family::Name("Inter"),
            weight: Weight::ExtraBold,
            ..Font::DEFAULT
        };
    }

    /// JetBrains Mono — anything technical (`--mono`): versions, chips, logs,
    /// keys, numbers that line up.
    pub struct JetBrainsMono;
    impl JetBrainsMono {
        pub const REGULAR: Font = Font {
            family: Family::Name("JetBrains Mono"),
            weight: Weight::Normal,
            ..Font::DEFAULT
        };
        pub const SEMI_BOLD: Font = Font {
            family: Family::Name("JetBrains Mono"),
            weight: Weight::Semibold,
            ..Font::DEFAULT
        };
    }
}

pub mod utility {
    use iced::widget::svg;
    use iced::{Color, Element, Length};

    pub fn icon<'a, M: 'a>(markup: &'static str, size: impl Into<Length> + std::marker::Copy, colors: Option<(Color, Color)>) -> Element<'a, M> {
        let colors = colors.unwrap_or((Color::WHITE, Color::WHITE));
        svg(svg::Handle::from_memory(markup.as_bytes()))
            .width(size)
            .height(size)
            .style(move |_theme, status| svg::Style {
                color: Some(match status {
                    svg::Status::Hovered => colors.0,
                    _ => colors.1,
                }),
            })
            .into()
    }
}

#[cfg(test)]
#[path = "../../tests/app/theme.rs"]
mod tests;

/// A shadow's blur radius, or none when the glass effect is off.
pub fn blur(radius: f32) -> f32 {
    if effects::glass() { radius } else { 0.0 }
}

/// The Appearance choices the views read while they draw. The Settings page sets them when it saves, and at boot.
pub mod effects {
    use std::sync::atomic::{AtomicBool, Ordering};

    use crate::app::preferences::Appearance;

    static ANIMATIONS: AtomicBool = AtomicBool::new(true);
    static REDUCE_MOTION: AtomicBool = AtomicBool::new(false);
    static PARTICLES: AtomicBool = AtomicBool::new(true);
    static AURORA: AtomicBool = AtomicBool::new(true);
    static GLASS: AtomicBool = AtomicBool::new(true);

    /// Makes `appearance` the one the views read.
    pub fn set(appearance: &Appearance) {
        ANIMATIONS.store(appearance.interface_animations, Ordering::Relaxed);
        REDUCE_MOTION.store(appearance.reduce_motion, Ordering::Relaxed);
        PARTICLES.store(appearance.ambient_particles, Ordering::Relaxed);
        AURORA.store(appearance.aurora_backgrounds, Ordering::Relaxed);
        GLASS.store(appearance.glass_effect, Ordering::Relaxed);
    }

    /// Whether transitions and micro-interactions run: they are instant when animations are off or reduce motion is set.
    pub fn animations() -> bool {
        ANIMATIONS.load(Ordering::Relaxed) && !REDUCE_MOTION.load(Ordering::Relaxed)
    }

    /// Whether the looping motion runs: the progress sweeps, the status pulses and the login backdrop. Reduce motion stops it.
    pub fn ambient() -> bool {
        !REDUCE_MOTION.load(Ordering::Relaxed)
    }

    /// Whether the login screen's embers show.
    pub fn particles() -> bool {
        PARTICLES.load(Ordering::Relaxed)
    }

    /// Whether the login screen's aurora glows show.
    pub fn aurora() -> bool {
        AURORA.load(Ordering::Relaxed)
    }

    /// Whether panels, cards and popovers cast their soft shadows.
    pub fn glass() -> bool {
        GLASS.load(Ordering::Relaxed)
    }
}
