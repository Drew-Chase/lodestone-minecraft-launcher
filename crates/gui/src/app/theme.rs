pub mod color {
    use iced::{Color, color};
    use iced_color_extension::rgb;

    /// Lode Green — `--mc-green`. The one loud color: primary actions, active
    /// states, online. Accents tag categories and never substitute for it.
    pub const PRIMARY: Color = color!(0x22FF84);

    /// Ink — text colors built from white at descending opacities so they
    /// adapt to any surface.
    pub struct Text;

    impl Text {
        /// `--ink-0` — headlines and active labels.
        pub const HEADLINE: Color = color!(0xFFFFFF);
        /// `--ink-1` — body copy; the app-wide default text color.
        pub const DEFAULT: Color = rgb!(255, 255, 255, 92%);
        /// `--ink-2` — secondary text, descriptions, meta.
        pub const MUTED: Color = rgb!(255, 255, 255, 68%);
        /// `--ink-3` — placeholders, captions, idle icons.
        pub const PLACEHOLDER: Color = rgb!(255, 255, 255, 45%);
        /// `--ink-4` — disabled only; never for readable text.
        pub const DISABLED: Color = rgb!(255, 255, 255, 28%);
    }

    /// Neutral surfaces — built up from black, one step per layer.
    pub struct Surface;
    impl Surface {
        /// `--bg-0` — app canvas.
        pub const BG0: Color = color!(0x08090A);
        /// `--bg-1` — sidebar.
        pub const BG1: Color = color!(0x0E1012);
        /// `--bg-2` — panels.
        pub const BG2: Color = color!(0x15181C);
        /// `--bg-3` — inputs, menus.
        pub const BG3: Color = color!(0x1D2127);
        /// `--bg-4` — hover, popovers.
        pub const BG4: Color = color!(0x272C34);
        /// `--line` — hairline borders.
        pub const LINE: Color = rgb!(255, 255, 255, 7%);
        /// `--line-strong` — emphasized borders.
        pub const LINE_STRONG: Color = rgb!(255, 255, 255, 12%);

        /// Whole-application background — the app canvas.
        pub const APPLICATION: Color = Self::BG0;
        /// Card / panel fill.
        pub const CARD: Color = Self::BG2;
        /// Titlebar chrome.
        pub const TITLEBAR: Color = Self::BG1;
    }

    /// Lode Green scale.
    pub struct Primary;
    impl Primary {
        /// `--mc-green` — primary CTA, active nav, toggles, progress, online.
        pub const DEFAULT: Color = color!(0x22FF84);
        /// `--mc-green-dim` — gradient stops, pressed states.
        pub const DIM: Color = color!(0x18C968);
        /// `--mc-green-deep` — tinted fills, badge grounds.
        pub const DEEP: Color = color!(0x0A5C33);
        /// `--mc-green-glow` — glows under CTAs, neon text, active bars.
        pub const GLOW: Color = rgb!(34, 255, 132, 35%);
        /// Tinted chip / fill — Lode Green at 12%.
        pub const TINT: Color = rgb!(34, 255, 132, 12%);
        /// On-green ink — text and icons placed on Lode Green.
        pub const ON: Color = color!(0x062814);
    }

    /// Accents — tag categories; never used for primary actions. Each pairs a
    /// full-strength color for text with a 14% tinted fill.
    pub struct Accents;
    impl Accents {
        // VIOLET — modded / Fabric, premium tier, aurora
        pub const VIOLET: Color = color!(0x9747FF);
        pub const VIOLET_TINT: Color = rgb!(151, 71, 255, 14%);
        /// Lightened violet — accent text on tinted fills.
        pub const VIOLET_LIGHT: Color = color!(0xB689FF);

        // AMBER — warnings, updates available, snapshots
        pub const AMBER: Color = color!(0xFFB545);
        pub const AMBER_TINT: Color = rgb!(255, 181, 69, 14%);

        // CYAN — info, servers, co-op, sync
        pub const CYAN: Color = color!(0x47D9FF);
        pub const CYAN_TINT: Color = rgb!(71, 217, 255, 14%);

        // PINK — errors, crashes, destructive, favorites
        pub const PINK: Color = color!(0xFF5EC8);
        pub const PINK_TINT: Color = rgb!(255, 94, 200, 14%);
    }

    /// Button palettes for [`crate::app::components::icon_button`], from the
    /// style guide's component specs.
    pub mod buttons {
        use iced::{Color, color};
        use iced_color_extension::{hsl, rgb};

        /// `.btn-primary` — green fill, on-green ink.
        pub struct Primary;
        impl Primary {
            pub const FOREGROUND: Color = super::Primary::ON;
            /// The guide fills this with a `#2aff8c → #16d96a` gradient; the
            /// solid base is Lode Green.
            pub const BACKGROUND: Color = super::Primary::DEFAULT;
            /// Hover — `brightness(1.06)`.
            pub const BACKGROUND_HOVER: Color = color!(0x24FF8C);
            /// Disabled — the guide fades the button to 40% opacity.
            pub const BACKGROUND_DISABLED: Color = rgb!(34, 255, 132, 40%);
        }

        /// `.btn-ghost` — white 4% fill, `--ink-1` label.
        pub struct Secondary;
        impl Secondary {
            pub const FOREGROUND: Color = super::Text::DEFAULT;
            pub const BACKGROUND: Color = rgb!(255, 255, 255, 4%);
            /// Hover — white 8%.
            pub const BACKGROUND_HOVER: Color = rgb!(255, 255, 255, 8%);
        }

        /// Destructive control (window close, discard) — ghost idle, `--pink`
        /// on hover.
        pub struct Danger;
        impl Danger {
            /// Idle icon — `--ink-2`, per the `.btn-icon` spec.
            pub const FOREGROUND: Color = super::Text::MUTED;
            /// Hover icon — `--ink-0`.
            pub const FOREGROUND_HOVER: Color = super::Text::HEADLINE;
            pub const BACKGROUND: Color = rgb!(255, 255, 255, 4%);
            /// Hover — `--pink`.
            pub const BACKGROUND_HOVER: Color = hsl!(356, 86%, 49%);
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
