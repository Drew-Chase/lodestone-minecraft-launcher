//! The page chrome around the sign-in card: the top bar with the brand tag and the language
//! picker, the card frame with its logo row and headline, the footer, and the status bar.

use iced::widget::{Column, Space, Text, column, container, mouse_area, rich_text, row, span, stack, svg};
use iced::{Background, Border, Element, Fill, Padding, Radians, Shadow, Vector, gradient};
use iced_color_extension::rgb;

use crate::app::LodestoneLauncher;
use crate::app::components::PulseDot;
use crate::app::components::backdrop::logo_glow_handle;
use crate::app::components::brand_tag::brand_tag;
use crate::app::components::language_picker::{LanguagePicker, PickerStyle};
use crate::app::lang;
use crate::app::theme::{color as palette, fonts};

use super::widgets::{logo_handle, mono, rgb_line, rule};
use super::{AuthStatus, Login, Message};

/// The height of the strip the brand tag sits in. The Windows titlebar is the same height, so
/// the picker lands at the same spot on every platform: 12px below the strip.
const TAG_STRIP_HEIGHT: f32 = 32.0;
/// The picker's inset from the window's left edge, and from the bottom of the tag strip.
const PICKER_LEFT: f32 = 24.0;
const PICKER_TOP: f32 = 12.0;

impl Login {
    /// The login's top bar: the brand tag centred in a strip, with the language picker below it
    /// at the left. Windows draws the tag in its own titlebar, so only the picker row is left
    /// there.
    pub(super) fn top_bar(&self) -> Element<'_, Message> {
        let picker_row = container(LanguagePicker::new(Message::SelectLanguage).style(PickerStyle::Chip))
            .width(Fill)
            .padding(Padding::new(0.0).left(PICKER_LEFT).top(PICKER_TOP));
        let mut bar = Column::new().width(Fill);
        if !cfg!(windows) {
            bar = bar.push(
                container(brand_tag())
                    .width(Fill)
                    .height(TAG_STRIP_HEIGHT)
                    .align_x(iced::alignment::Horizontal::Center)
                    .align_y(iced::alignment::Vertical::Center),
            );
        }
        bar.push(picker_row).into()
    }

    /// The floating sign-in card.
    pub(super) fn card(&self) -> Element<'_, Message> {
        container(stack![
            // Green glow washing the top-right corner of the card.
            container(svg(logo_glow_handle().clone()).width(220).height(220))
                .width(Fill)
                .height(Fill)
                .align_x(iced::alignment::Horizontal::Right)
                .align_y(iced::alignment::Vertical::Top),
            self.card_content(),
        ])
        .width(440)
        .height(450)
        .style(|_| container::Style {
            background: Some(Background::Gradient(
                gradient::Linear::new(Radians(std::f32::consts::FRAC_PI_2))
                    .add_stop(0.0, rgb!(21, 24, 28, 85%))
                    .add_stop(1.0, rgb!(14, 16, 18, 92%))
                    .into(),
            )),
            border: Border {
                radius: 20.0.into(),
                width: 1.0,
                color: rgb_line(),
            },
            shadow: Shadow {
                color: rgb!(0, 0, 0, 60%),
                offset: Vector::new(0.0, 30.0),
                blur_radius: 80.0,
            },
            ..container::Style::default()
        })
        .into()
    }

    /// Everything inside the card. The trailing `Fill` space pins the footer
    /// to the card's bottom edge like the reference.
    fn card_content(&self) -> Element<'_, Message> {
        column![
            self.logo_row(),
            Space::new().height(22),
            self.headline(),
            Text::new(lang::t("login.subtitle"))
                .size(13)
                .font(fonts::Inter::REGULAR)
                .color(palette::Text::MUTED)
                .line_height(1.5),
            Space::new().height(22),
            self.sign_in_area(),
            Space::new().height(Fill),
            rule(),
            Space::new().height(12),
            self.footer(),
        ]
        .spacing(6)
        .padding(Padding::new(0.0).left(36).right(36).top(40).bottom(32))
        .into()
    }

    /// The Lodestone mark with its glow halo, plus the wordmark.
    fn logo_row(&self) -> Element<'_, Message> {
        row![
            stack![
                container(svg(logo_glow_handle().clone()).width(96).height(96))
                    .width(Fill)
                    .height(Fill)
                    .align_x(iced::alignment::Horizontal::Center)
                    .align_y(iced::alignment::Vertical::Center),
                svg(logo_handle().clone()).width(52).height(52),
            ]
            .width(52)
            .height(52),
            column![
                Text::new(lang::t("application.title"))
                    .size(22)
                    .font(fonts::Inter::EXTRA_BOLD)
                    .color(palette::Text::HEADLINE),
                mono(lang::t_with("login.brand_subtitle", &[("VERSION", LodestoneLauncher::VERSION)]), 11.0),
            ]
            .spacing(2),
        ]
        .spacing(12)
        .align_y(iced::Alignment::Center)
        .into()
    }

    /// The headline from `login.title`, with its `%P` run in Lode Green.
    fn headline(&self) -> Element<'_, Message> {
        let spans: Vec<_> = lang::rich_segments("login.title")
            .into_iter()
            .map(|segment| {
                let ink = if segment.primary {
                    palette::Primary::DEFAULT
                } else {
                    palette::Text::HEADLINE
                };
                span::<(), iced::Font>(segment.text).font(fonts::Inter::EXTRA_BOLD).size(26).color(ink)
            })
            .collect();
        rich_text(spans).into()
    }

    /// The card's legal line.
    fn footer(&self) -> Element<'_, Message> {
        row![
            Text::new(lang::t("login.not_affiliated"))
                .size(11)
                .font(fonts::Inter::REGULAR)
                .color(palette::Text::PLACEHOLDER),
            Space::new().width(Fill),
            mouse_area(
                Text::new(lang::t("login.learn_more"))
                    .size(11)
                    .font(fonts::Inter::MEDIUM)
                    .color(palette::Primary::DEFAULT)
            )
            .on_press(Message::LearnMore)
            .interaction(iced::mouse::Interaction::Pointer),
        ]
        .align_y(iced::Alignment::Center)
        .into()
    }

    /// The service status line pinned to the bottom of the screen, fed by
    /// the real auth-services probe: the dot and label carry reachability,
    /// the middle segment the measured round-trip, and (when Microsoft
    /// reports one for this address) the region scope.
    pub(super) fn status_bar(&self, auth_status: Option<AuthStatus>) -> Element<'_, Message> {
        let (label, dot, delay, region) = match auth_status {
            Some(status) if status.online => (
                lang::t("login.status_online"),
                PulseDot::new().size(6.0).pulsing(true),
                Some(status.delay_ms),
                status.region,
            ),
            Some(_) => (
                lang::t("login.status_unreachable"),
                PulseDot::with_color(palette::Accents::AMBER).size(6.0).pulsing(true),
                None,
                None,
            ),
            None => (
                lang::t("login.status_checking"),
                PulseDot::with_color(palette::Text::DISABLED).size(6.0).pulsing(false),
                None,
                None,
            ),
        };

        let mut bar = row![dot, mono(label, 11.0),];
        if let Some(delay_ms) = delay {
            let ms = delay_ms.to_string();
            bar = bar
                .push(mono("·", 11.0))
                .push(mono(lang::t_with("login.status_delay", &[("ms", ms.as_str())]), 11.0));
        }
        if let Some(region) = region {
            bar = bar.push(mono("·", 11.0)).push(mono(region.to_uppercase(), 11.0));
        }
        bar.push(Space::new().width(Fill))
            .push(mono(format!("v{} · {}", LodestoneLauncher::VERSION, LodestoneLauncher::BUILD), 11.0))
            .spacing(14)
            .align_y(iced::Alignment::Center)
            .padding(Padding::new(0.0).left(24).right(24).bottom(18))
            .width(Fill)
            .into()
    }
}
