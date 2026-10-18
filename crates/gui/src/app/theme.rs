pub mod color {
    use iced::{Color, color};

    pub struct Text;

    impl Text {
        pub const DEFAULT: Color = color!(0x18181b);
        pub const MUTED: Color = color!(0x87878e);
        pub const PLACEHOLDER: Color = color!(0x8e8e95);
    }

    pub struct Surface;
    impl Surface {
        pub const APPLICATION: Color = color!(0xe3e3ea);
        pub const CARD: Color = color!(0xffffff);
        pub const TITLEBAR: Color = color!(0xf3f4f6);
        pub const PRIMARY_DARK: Color = color!(0xaa3b3f);
        pub const PRIMARY: Color = color!(0xf13848);
    }

    pub mod buttons {
        use iced::{Color, color};

        pub struct Primary;
        impl Primary {
            pub const FOREGROUND: Color = Color::WHITE;
            pub const BACKGROUND: Color = color!(0xf1444e);
            pub const BACKGROUND_HOVER: Color = color!(0xf55c5f);
            pub const BACKGROUND_DISABLED: Color = color!(0xf8a1a6);
        }

        pub struct Secondary;
        impl Secondary {
            pub const FOREGROUND: Color = super::Text::DEFAULT;
            pub const BACKGROUND: Color = Color::TRANSPARENT;
            pub const BACKGROUND_HOVER: Color = color!(0xe5e7eb);
        }

        pub struct Danger;
        impl Danger {
            pub const FOREGROUND: Color = color!(0xa43a37);
            pub const FOREGROUND_HOVER: Color = Color::WHITE;
            pub const BACKGROUND: Color = Color::TRANSPARENT;
            pub const BACKGROUND_HOVER: Color = color!(0xff4644);
        }
    }
}

pub mod fonts {
    use iced::Font;
    use iced::font::{Family, Weight};

    pub struct Roboto;
    impl Roboto {
        pub const BOLD: Font = Font {
            family: Family::Name("Roboto"),
            weight: Weight::Bold,
            ..Font::DEFAULT
        };
        pub const MEDIUM: Font = Font {
            family: Family::Name("Roboto"),
            weight: Weight::Medium,
            ..Font::DEFAULT
        };
        pub const REGULAR: Font = Font {
            family: Family::Name("Roboto"),
            weight: Weight::Normal,
            ..Font::DEFAULT
        };
    }
}

pub mod utility {
    use iced::widget::svg;
    use iced::{Color, Element, Length};

    pub fn icon<'a, M: 'a>(
        markup: &'static str,
        size: impl Into<Length> + std::marker::Copy,
        colors: Option<(Color, Color)>,
    ) -> Element<'a, M> {
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
