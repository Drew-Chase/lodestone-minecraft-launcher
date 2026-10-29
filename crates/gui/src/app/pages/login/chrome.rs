//! The page chrome around the sign-in card: top bar, language chip, the card
//! frame with its logo row and headline, the footer, and the status bar.

use aurora_leaf::Icon;
use iced::widget::{Space, Text, column, container, mouse_area, row, stack, svg};
use iced::{Background, Border, Element, Fill, Padding, Radians, Shadow, Vector, gradient};
use iced_color_extension::rgb;

use crate::app::LodestoneLauncher;
use crate::app::components::PulseDot;
use crate::app::components::backdrop::logo_glow_handle;
use crate::app::theme::{color as palette, fonts};

use super::widgets::{icon, logo_handle, mono, rgb_line, rule};
use super::{AuthStatus, Login, Message};

impl Login {
    /// Language chip left, window brand centered — the reference's top edge.
    pub(super) fn top_bar(&self) -> Element<'_, Message> {
        #[cfg(not(windows))]
        {
            stack![
                container(self.language_chip()).width(Fill).align_x(iced::alignment::Horizontal::Left),
                container(mono(format!("LODESTONE · v{}", LodestoneLauncher::VERSION), 11.0))
                    .width(Fill)
                    .align_x(iced::alignment::Horizontal::Center),
            ]
            .into()
        }
        #[cfg(windows)]
        row![].into()
    }

    /// The "English · US" chip — decorative until localization lands.
    fn language_chip(&self) -> Element<'_, Message> {
        container(
            row![
                icon(Icon::lucide().globe(), 14.0, palette::Text::MUTED, 0.68),
                Text::new("English · US")
                    .size(12)
                    .font(fonts::Inter::MEDIUM)
                    .color(palette::Text::DEFAULT),
                icon(Icon::lucide().chevron_down(), 12.0, palette::Text::MUTED, 0.68),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
        )
        .padding(Padding::new(0.0).left(16).right(12).top(10).bottom(10))
        .style(|_| container::Style {
            background: Some(Background::Color(rgb!(0, 0, 0, 40%))),
            border: Border {
                radius: 12.0.into(),
                ..Border::default()
            },
            ..container::Style::default()
        })
        .into()
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
            Text::new("Sign in to sync your worlds, instances, and friends across devices.")
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
                Text::new("Lodestone")
                    .size(22)
                    .font(fonts::Inter::EXTRA_BOLD)
                    .color(palette::Text::HEADLINE),
                mono(format!("THIRD-PARTY LAUNCHER · v{}", LodestoneLauncher::VERSION), 11.0),
            ]
            .spacing(2),
        ]
        .spacing(12)
        .align_y(iced::Alignment::Center)
        .into()
    }

    /// "Welcome back, crafter" — Inter ExtraBold, per the reference.
    fn headline(&self) -> Element<'_, Message> {
        row![
            Text::new("Welcome back, ")
                .size(26)
                .font(fonts::Inter::EXTRA_BOLD)
                .color(palette::Text::HEADLINE),
            Text::new("crafter")
                .size(26)
                .font(fonts::Inter::EXTRA_BOLD)
                .color(palette::Primary::DEFAULT),
        ]
        .align_y(iced::Alignment::Center)
        .into()
    }

    /// The card's legal line.
    fn footer(&self) -> Element<'_, Message> {
        row![
            Text::new("Not affiliated with Mojang or Microsoft")
                .size(11)
                .font(fonts::Inter::REGULAR)
                .color(palette::Text::PLACEHOLDER),
            Space::new().width(Fill),
            mouse_area(
                Text::new("Learn more ↗")
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
                "AUTH SERVICES ONLINE",
                PulseDot::new().size(6.0).pulsing(true),
                Some(status.delay_ms),
                status.region,
            ),
            Some(_) => (
                "AUTH SERVICES UNREACHABLE",
                PulseDot::with_color(palette::Accents::AMBER).size(6.0).pulsing(true),
                None,
                None,
            ),
            None => (
                "CHECKING AUTH SERVICES",
                PulseDot::with_color(palette::Text::DISABLED).size(6.0).pulsing(false),
                None,
                None,
            ),
        };

        let mut bar = row![dot, mono(label, 11.0),];
        if let Some(delay_ms) = delay {
            bar = bar.push(mono("·", 11.0)).push(mono(format!("{delay_ms}ms"), 11.0));
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
