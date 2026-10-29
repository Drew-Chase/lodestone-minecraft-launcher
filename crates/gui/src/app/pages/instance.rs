//! The instance detail page — the immersive per-instance view from
//! `04-instance.png`.
//!
//! The page is a fixed masthead plus tab row over one scrolling content
//! region. The masthead is the generated biome scene (cropped, not
//! stretched), the instance identity block and the session controls; the
//! Overview pairs About/sync/screenshots with session telemetry and
//! friends. Geometry and type sizes mirror the bundled design source
//! (`res/reference-images/Lodestone Launcher (standalone).html`).
//!
//! Telemetry values and the running session are demo state until the game
//! launcher is wired in; the instance model's own data (name, loader,
//! version, mods, playtime, folder, biome) is real.

use crate::app::components::button::Variant;
use crate::app::components::scene_art::{banner_handle, icon_handle, screenshot_handle};
use crate::app::components::{IconButton, ProgressBar, PulseDot, Tabs, TextButton};
use crate::app::pages::Page;
use crate::app::pages::library::instance::{Biome, Instance, InstanceState, Loader};
use crate::app::theme::color;
use crate::app::theme::fonts;
use aurora_leaf::Icon;
use iced::widget::{Space, Text, column, container, responsive, row, scrollable, stack, svg};
use iced::{Background, Border, Color, ContentFit, Element, Fill, Gradient, Length, Padding, Shadow, Size, alignment, gradient};
use iced_color_extension::rgb;

/// The masthead's height — the design's 230px scene band.
const HERO_HEIGHT: f32 = 230.0;
/// The page's horizontal gutter — 28px, per the design.
const GUTTER: f32 = 28.0;
/// The instance icon's edge.
const ICON_SIZE: f32 = 110.0;
/// The icon hangs 40px below the masthead (`translateY(40px)`).
const ICON_TOP: f32 = HERO_HEIGHT - ICON_SIZE + 40.0;
/// Left edge of the identity text — icon + the design's 20px gap.
const IDENTITY_LEFT: f32 = GUTTER + ICON_SIZE + 20.0;

/// The page's tabs. Only Overview is designed in the reference; the rest
/// switch to their own lightweight panels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Overview,
    Mods,
    Worlds,
    Screenshots,
    Logs,
    Settings,
}

impl Tab {
    pub const ALL: [Tab; 6] = [Tab::Overview, Tab::Mods, Tab::Worlds, Tab::Screenshots, Tab::Logs, Tab::Settings];

    pub fn label(self) -> &'static str {
        match self {
            Tab::Overview => "Overview",
            Tab::Mods => "Mods",
            Tab::Worlds => "Worlds",
            Tab::Screenshots => "Screenshots",
            Tab::Logs => "Logs",
            Tab::Settings => "Settings",
        }
    }
}

/// Messages the instance page handles.
#[derive(Debug, Clone)]
pub enum Message {
    /// Leaves the detail view — the root steps the router back.
    Back,
    /// Selects a tab.
    TabSelected(Tab),
    /// Favourites the instance (display state only for now).
    ToggleFavorite,
    /// Reveals the instance folder (the root owns IO).
    RevealFolder,
    /// The kebab menu — display-only until the instance actions land.
    More,
    /// Pauses or resumes the running session (display state only).
    TogglePause,
    /// Requests navigation — the root intercepts this before the page's
    /// `update` runs (the `<Link>` of this framework).
    Navigate(Page),
}

/// The instance detail page state.
#[derive(Debug, Default)]
pub struct InstancePage {
    instance: Option<Instance>,
    tab: Tab,
    favorite: bool,
    paused: bool,
}

impl InstancePage {
    /// Points the page at `instance`, resetting transient controls.
    pub fn show(&mut self, instance: Instance) {
        self.instance = Some(instance);
        self.tab = Tab::Overview;
        self.paused = false;
    }

    /// The instance currently on display.
    pub fn instance(&self) -> Option<&Instance> {
        self.instance.as_ref()
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::TabSelected(tab) => self.tab = tab,
            Message::ToggleFavorite => self.favorite = !self.favorite,
            Message::TogglePause => self.paused = !self.paused,
            // Navigation is intercepted by the root; the remaining actions
            // are display-only until the instance manager lands.
            Message::Back | Message::RevealFolder | Message::More | Message::Navigate(_) => {}
        }
    }

    /// The page: fixed masthead, fixed tab row, scrolling content.
    pub fn view(&self, viewport: Size) -> Element<'_, Message> {
        let Some(instance) = &self.instance else {
            return self.empty_state();
        };

        let page = column![
            self.hero(instance, viewport),
            self.tab_bar(),
            scrollable(self.tab_content(instance)).width(Fill).height(Fill),
        ]
        .width(Fill)
        .height(Fill);

        // The icon hangs 40px below the masthead, so it is layered over the
        // whole page: inside the masthead it would be clamped to the 70px
        // left beneath its top offset.
        stack![page, hero_icon(instance)].width(Fill).height(Fill).into()
    }

    fn empty_state(&self) -> Element<'_, Message> {
        container(
            column![
                Text::new("No instance selected")
                    .size(18)
                    .font(fonts::Inter::EXTRA_BOLD)
                    .color(color::Text::HEADLINE),
                Text::new("Choose an instance in the Library to open its detail view.")
                    .size(12)
                    .font(fonts::Inter::REGULAR)
                    .color(color::Text::MUTED),
                TextButton::ghost("Back to Library").on_press(Message::Back),
            ]
            .spacing(12)
            .align_x(alignment::Horizontal::Center),
        )
        .width(Fill)
        .height(Fill)
        .center_x(Fill)
        .center_y(Fill)
        .into()
    }

    /// The masthead: scene band, darkening gradient, chrome buttons, identity
    /// and the session controls. The icon is layered over the page by `view`.
    fn hero<'a>(&'a self, instance: &'a Instance, viewport: Size) -> Element<'a, Message> {
        // The art is generated at the masthead's exact pixel size, so it fills
        // the band without stretching.
        let (biome, seed) = (instance.biome, instance.art_seed);
        let art = responsive(move |size| {
            svg(banner_handle(biome, seed, size))
                .content_fit(ContentFit::Fill)
                .width(Fill)
                .height(Fill)
                .into()
        })
        .height(HERO_HEIGHT);

        // `linear-gradient(180deg, rgba(8,9,10,.3), rgba(8,9,10,.85) 85%, bg)`
        let fade = container(Space::new().width(Fill).height(Fill))
            .width(Fill)
            .height(Fill)
            .style(|_| container::Style {
                background: Some(Background::Gradient(Gradient::Linear(
                    gradient::Linear::new(std::f32::consts::PI)
                        .add_stop(0.0, rgb!(8, 9, 10, 30%))
                        .add_stop(0.85, rgb!(8, 9, 10, 85%))
                        .add_stop(1.0, color::Surface::APPLICATION),
                ))),
                ..container::Style::default()
            });

        let back = layer(
            glass_button(Icon::lucide().chevron_left(), Message::Back).into(),
            alignment::Horizontal::Left,
            alignment::Vertical::Top,
            Padding::new(0.0).left(24.0).top(16.0),
        );

        let favorite_markup = Icon::lucide().heart();
        let favorite = if self.favorite {
            glass_button_with_style(favorite_markup, favorite_style(), Message::ToggleFavorite)
        } else {
            glass_button(favorite_markup, Message::ToggleFavorite)
        };
        let top_right = layer(
            row![
                favorite,
                glass_button(Icon::lucide().external_link(), Message::RevealFolder),
                glass_button(Icon::lucide().ellipsis_vertical(), Message::More),
            ]
            .spacing(8)
            .into(),
            alignment::Horizontal::Right,
            alignment::Vertical::Top,
            Padding::new(0.0).right(24.0).top(16.0),
        );

        let identity = column![
            row![
                status_badge(instance.state),
                loader_badge(instance),
                badge(format!("MC {}", instance.mc_version), color::Text::MUTED, rgb!(255, 255, 255, 6%)),
                badge(format!("{} MODS", instance.mod_count), color::Accents::AMBER, color::Accents::AMBER_TINT),
            ]
            .spacing(6),
            Text::new(instance.name.clone())
                .size(32)
                .line_height(1.0)
                .font(fonts::Inter::EXTRA_BOLD)
                .color(color::Text::HEADLINE),
            Text::new(meta_line(instance))
                .size(12)
                .line_height(1.2)
                .font(fonts::JetBrainsMono::REGULAR)
                .color(color::Text::PLACEHOLDER),
        ]
        .spacing(7);

        let narrow = viewport.width < 1020.0;
        let mut identity = identity;
        let actions_layer = if narrow {
            identity = identity.push(self.session_controls(instance)).spacing(12);
            None
        } else {
            Some(layer(
                self.session_controls(instance).into(),
                alignment::Horizontal::Right,
                alignment::Vertical::Bottom,
                Padding::new(0.0).right(GUTTER).bottom(18.0),
            ))
        };
        let identity_layer = layer(
            identity.into(),
            alignment::Horizontal::Left,
            alignment::Vertical::Bottom,
            Padding::new(0.0).left(IDENTITY_LEFT).bottom(18.0),
        );

        let mut surface = stack![art, fade, back, top_right, identity_layer];
        if let Some(actions) = actions_layer {
            surface = surface.push(actions);
        }

        container(surface.width(Fill).height(HERO_HEIGHT)).width(Fill).height(HERO_HEIGHT).into()
    }

    /// Pause plus the running-session button.
    fn session_controls(&self, instance: &Instance) -> Element<'_, Message> {
        let playing = instance.state == InstanceState::Playing;
        let pause_label = if self.paused { "Resume" } else { "Pause" };

        row![
            TextButton::content(
                row![
                    icon(Icon::lucide().pause(), 13.0, color::Text::DEFAULT),
                    Text::new(pause_label).size(13).font(fonts::Inter::MEDIUM).color(color::Text::DEFAULT),
                ]
                .spacing(8)
                .align_y(alignment::Vertical::Center),
            )
            .variant(Variant::Ghost)
            .padding(Padding::new(0.0).left(16).right(16).top(10).bottom(10))
            .on_press(Message::TogglePause),
            TextButton::content(
                row![
                    icon(Icon::lucide().play(), 12.0, color::Primary::ON),
                    Text::new(if playing { "Running · 12m" } else { "Launch" })
                        .size(14)
                        .font(fonts::Inter::BOLD)
                        .color(color::Primary::ON),
                ]
                .spacing(8)
                .align_y(alignment::Vertical::Center),
            )
            .variant(Variant::Primary)
            .padding(Padding::new(0.0).left(22).right(22).top(11).bottom(11))
            .on_press(Message::TogglePause),
        ]
        .spacing(8)
        .align_y(alignment::Vertical::Center)
        .into()
    }

    /// The underlined tab row. `Tabs` draws the full-width rule and the active
    /// bar itself, so the bar paints over the rule and every tab's whole slot
    /// is clickable.
    fn tab_bar(&self) -> Element<'_, Message> {
        let active = Tab::ALL.iter().position(|tab| *tab == self.tab).unwrap_or_default();

        container(
            Tabs::new(Tab::ALL.map(Tab::label))
                .underline()
                .inset(GUTTER)
                .width(Fill)
                .active(active)
                .on_select(|index| Message::TabSelected(Tab::ALL[index])),
        )
        .width(Fill)
        .padding(Padding::new(0.0).top(52.0))
        .into()
    }

    /// The selected tab's panel.
    fn tab_content<'a>(&'a self, instance: &'a Instance) -> Element<'a, Message> {
        match self.tab {
            Tab::Overview => overview(instance),
            Tab::Screenshots => container(screenshots_section())
                .width(Fill)
                .padding(Padding::new(0.0).left(GUTTER).right(GUTTER).top(22.0).bottom(40.0))
                .into(),
            tab => placeholder(tab),
        }
    }
}

/// The Overview tab: About / sync / screenshots beside session / friends.
fn overview(instance: &Instance) -> Element<'_, Message> {
    let left = column![about_panel(instance), sync_panel(), screenshots_section()]
        .spacing(16)
        .width(Length::FillPortion(2));

    let right = column![session_panel(instance), players_panel()]
        .spacing(14)
        .width(Length::FillPortion(1));

    container(row![left, right].spacing(18).align_y(alignment::Vertical::Top).width(Fill))
        .width(Fill)
        .padding(Padding::new(0.0).left(GUTTER).right(GUTTER).top(22.0).bottom(40.0))
        .into()
}

fn placeholder(tab: Tab) -> Element<'static, Message> {
    container(
        column![
            Text::new(tab.label()).size(13).font(fonts::Inter::SEMI_BOLD).color(color::Text::HEADLINE),
            Text::new(format!("The {} view lands with the instance manager.", tab.label()))
                .size(12)
                .font(fonts::Inter::REGULAR)
                .color(color::Text::MUTED),
        ]
        .spacing(8),
    )
    .width(Fill)
    .padding(Padding::new(0.0).left(GUTTER).right(GUTTER).top(22.0).bottom(40.0))
    .into()
}

/// Positions a child inside its parent: the container fills the parent and
/// the padding/alignment pin the child to one edge.
fn layer<'a, M: 'a>(content: Element<'a, M>, align_x: alignment::Horizontal, align_y: alignment::Vertical, padding: Padding) -> Element<'a, M> {
    container(content)
        .width(Fill)
        .height(Fill)
        .align_x(align_x)
        .align_y(align_y)
        .padding(padding)
        .into()
}

/// The instance icon: 110px square, hanging 40px below the masthead.
fn hero_icon(instance: &Instance) -> Element<'static, Message> {
    let icon = container(
        svg(icon_handle(instance.biome, instance.art_seed))
            .content_fit(ContentFit::Fill)
            .width(Fill)
            .height(Fill),
    )
    .width(ICON_SIZE)
    .height(ICON_SIZE)
    .style(|_| container::Style {
        background: Some(Background::Color(color::Surface::APPLICATION)),
        border: Border {
            radius: 18.0.into(),
            width: 3.0,
            color: color::Surface::APPLICATION,
        },
        shadow: Shadow {
            color: rgb!(0, 0, 0, 80%),
            offset: iced::Vector::new(0.0, 20.0),
            blur_radius: 40.0,
        },
        ..container::Style::default()
    });

    layer(
        icon.into(),
        alignment::Horizontal::Left,
        alignment::Vertical::Top,
        Padding::new(0.0).left(GUTTER).top(ICON_TOP),
    )
}

/// A 36px glass chrome button over the masthead art.
fn glass_button(markup: &'static str, message: Message) -> Element<'static, Message> {
    glass_button_with_style(markup, glass_style(), message)
}

fn glass_button_with_style(
    markup: &'static str,
    style: crate::app::components::icon_button::IconButtonStyle,
    message: Message,
) -> Element<'static, Message> {
    container(
        IconButton::new(markup)
            .size(36)
            .radius(12.0)
            .icon_size(14.0)
            .style(style)
            .on_press(message),
    )
    .style(|_| container::Style {
        background: Some(Background::Color(rgb!(0, 0, 0, 40%))),
        border: Border {
            radius: 12.0.into(),
            width: 1.0,
            color: color::Surface::LINE,
        },
        ..container::Style::default()
    })
    .into()
}

fn glass_style() -> crate::app::components::icon_button::IconButtonStyle {
    crate::app::components::icon_button::IconButtonStyle {
        foreground: color::Text::MUTED,
        foreground_hover: color::Text::HEADLINE,
        background: Color::TRANSPARENT,
        background_hover: rgb!(255, 255, 255, 8%),
        background_disabled: Color::TRANSPARENT,
        foreground_disabled: color::Text::DISABLED,
        background_pressed: rgb!(255, 255, 255, 12%),
        foreground_pressed: color::Text::HEADLINE,
    }
}

fn favorite_style() -> crate::app::components::icon_button::IconButtonStyle {
    crate::app::components::icon_button::IconButtonStyle {
        foreground: color::Accents::PINK,
        foreground_hover: color::Accents::PINK,
        background: Color::TRANSPARENT,
        background_hover: color::Accents::PINK_TINT,
        ..glass_style()
    }
}

/// The launched/idle badge.
fn status_badge(state: InstanceState) -> Element<'static, Message> {
    let (label, ink, fill, pulsing) = match state {
        InstanceState::Playing => ("LAUNCHED", color::Primary::DEFAULT, color::Primary::TINT, true),
        InstanceState::Idle => ("IDLE", color::Text::MUTED, rgb!(255, 255, 255, 6%), false),
    };
    badge_with_dot(label, ink, fill, pulsing)
}

fn badge_with_dot(label: &'static str, ink: Color, fill: Color, pulsing: bool) -> Element<'static, Message> {
    container(
        row![
            PulseDot::with_color(ink).size(5.0).pulsing(pulsing),
            Text::new(label).size(11).font(fonts::JetBrainsMono::SEMI_BOLD).color(ink),
        ]
        .spacing(6)
        .align_y(alignment::Vertical::Center),
    )
    .padding(Padding::new(0.0).left(8).right(8).top(4).bottom(4))
    .style(move |_| container::Style {
        background: Some(Background::Color(fill)),
        border: Border {
            radius: 999.0.into(),
            ..Border::default()
        },
        ..container::Style::default()
    })
    .into()
}

fn loader_badge(instance: &Instance) -> Element<'static, Message> {
    let (ink, fill) = match instance.loader {
        Loader::Vanilla => (color::Primary::DEFAULT, color::Primary::TINT),
        Loader::Fabric => (color::Accents::VIOLET_LIGHT, color::Accents::VIOLET_TINT),
        Loader::Forge => (color::Accents::AMBER, color::Accents::AMBER_TINT),
        Loader::Quilt => (color::Accents::CYAN, color::Accents::CYAN_TINT),
        Loader::NeoForge => (color::Accents::PINK, color::Accents::PINK_TINT),
    };
    let label = match &instance.loader_version {
        Some(version) => format!("{} {version}", instance.loader.label()),
        None => instance.loader.label().to_string(),
    };
    badge(label, ink, fill)
}

fn badge(label: String, ink: Color, fill: Color) -> Element<'static, Message> {
    container(Text::new(label).size(11).font(fonts::JetBrainsMono::SEMI_BOLD).color(ink))
        .padding(Padding::new(0.0).left(8).right(8).top(4).bottom(4))
        .style(move |_| container::Style {
            background: Some(Background::Color(fill)),
            border: Border {
                radius: 999.0.into(),
                ..Border::default()
            },
            ..container::Style::default()
        })
        .into()
}

/// The mono identity line: folder, footprint, last save.
fn meta_line(instance: &Instance) -> String {
    format!("/instances/{} · {:.1} GB · last saved 02:14", instance.directory_name, size_gb(instance))
}

/// The instance's on-disk footprint — a deterministic demo figure derived
/// from the mod count (87 mods → 4.8 GB).
fn size_gb(instance: &Instance) -> f32 {
    0.7 + instance.mod_count as f32 * 0.047
}

/// Playtime, humanized — "24h 12m", "12 min", "—".
pub fn format_playtime(seconds: u32) -> String {
    let minutes = seconds / 60;
    let hours = minutes / 60;
    if hours > 0 {
        let rest = minutes % 60;
        if rest == 0 { format!("{hours}h") } else { format!("{hours}h {rest}m") }
    } else if minutes > 0 {
        format!("{minutes} min")
    } else {
        "—".to_string()
    }
}

fn about_text(instance: &Instance) -> String {
    if instance.name == "Aether Legacy" {
        "A re-imagining of the classic Aether mod — soar between floating isles, battle Slider bosses, and uncover \
         the secrets of a continent suspended in the void."
            .to_string()
    } else {
        format!(
            "A {} instance running Minecraft {}{}. Launch it to continue where you left off — session telemetry, \
             screenshots and friends will appear here while it is running.",
            instance.loader.label(),
            instance.mc_version,
            instance
                .loader_version
                .as_ref()
                .map(|version| format!(" with {} {version}", instance.loader.label()))
                .unwrap_or_default()
        )
    }
}

/// The static card surface — a subtle white gradient over the page with a
/// hairline ring (`--r-lg`).
fn panel<'a, M: 'a>(content: impl Into<Element<'a, M>>) -> Element<'a, M> {
    container(content)
        .width(Fill)
        .padding(18)
        .style(|_| container::Style {
            background: Some(Background::Gradient(Gradient::Linear(
                gradient::Linear::new(std::f32::consts::PI)
                    .add_stop(0.0, rgb!(255, 255, 255, 4%))
                    .add_stop(1.0, rgb!(255, 255, 255, 2%)),
            ))),
            border: Border {
                radius: 18.0.into(),
                width: 1.0,
                color: color::Surface::LINE,
            },
            ..container::Style::default()
        })
        .into()
}

fn about_panel(instance: &Instance) -> Element<'static, Message> {
    panel(
        column![
            Text::new("About").size(13).font(fonts::Inter::SEMI_BOLD).color(color::Text::HEADLINE),
            Text::new(about_text(instance))
                .size(12)
                .line_height(1.6)
                .font(fonts::Inter::REGULAR)
                .color(color::Text::MUTED)
                .width(Fill),
        ]
        .spacing(10),
    )
}

fn sync_panel() -> Element<'static, Message> {
    panel(
        column![
            row![
                icon(Icon::lucide().download(), 14.0, color::Primary::DEFAULT),
                Text::new("Syncing mod updates")
                    .size(13)
                    .font(fonts::Inter::SEMI_BOLD)
                    .color(color::Text::HEADLINE),
                Space::new().width(Fill),
                Text::new("63%")
                    .size(11)
                    .font(fonts::JetBrainsMono::SEMI_BOLD)
                    .color(color::Primary::DEFAULT),
            ]
            .spacing(10)
            .align_y(alignment::Vertical::Center),
            ProgressBar::new(0.63).height(6),
            Text::new("Downloading iris-mc1.20.1-1.7.1.jar · 12.4 MB/s")
                .size(11)
                .font(fonts::JetBrainsMono::REGULAR)
                .color(color::Text::PLACEHOLDER),
        ]
        .spacing(10),
    )
}

fn session_panel(instance: &Instance) -> Element<'static, Message> {
    let playing = instance.state == InstanceState::Playing;
    let playtime = if playing { "12 min".to_string() } else { "—".to_string() };
    let memory = if playing { "2.1 / 6 GB" } else { "—" };
    let framerate = if playing { "58 fps" } else { "—" };
    let tick = if playing { "18.2 ms" } else { "—" };

    panel(
        column![
            section_label("SESSION"),
            Space::new().height(2),
            session_row("Playtime", playtime, color::Primary::DEFAULT),
            session_row("Total", format_playtime(instance.playtime_seconds), color::Text::HEADLINE),
            session_row("Memory", memory.to_string(), color::Accents::CYAN),
            session_row("Framerate", framerate.to_string(), color::Accents::AMBER),
            session_row("Tick time", tick.to_string(), color::Text::DEFAULT),
        ]
        .spacing(0),
    )
}

fn session_row(label: &'static str, value: String, ink: Color) -> Element<'static, Message> {
    column![
        container(
            row![
                Text::new(label).size(12).font(fonts::Inter::REGULAR).color(color::Text::MUTED),
                Space::new().width(Fill),
                Text::new(value).size(12).font(fonts::JetBrainsMono::SEMI_BOLD).color(ink),
            ]
            .align_y(alignment::Vertical::Center),
        )
        .width(Fill)
        .padding(Padding::new(0.0).top(8).bottom(8)),
        container(Space::new().width(Fill).height(1)).width(Fill).style(|_| container::Style {
            background: Some(Background::Color(color::Surface::LINE)),
            ..container::Style::default()
        }),
    ]
    .into()
}

fn section_label(label: &'static str) -> Element<'static, Message> {
    Text::new(label)
        .size(12)
        .font(fonts::JetBrainsMono::SEMI_BOLD)
        .color(color::Text::PLACEHOLDER)
        .into()
}

fn players_panel() -> Element<'static, Message> {
    const PLAYERS: [(&str, &str, Color); 3] = [
        ("PI", "pixelpete", color::Accents::PINK),
        ("CA", "cavesworn", color::Accents::CYAN),
        ("SU", "sundayknight", color::Accents::AMBER),
    ];

    panel(
        column![
            section_label("PLAYING WITH"),
            Space::new().height(2),
            column(PLAYERS.into_iter().map(player_row)).spacing(0),
        ]
        .spacing(0),
    )
}

fn player_row((initials, name, ink): (&'static str, &'static str, Color)) -> Element<'static, Message> {
    row![
        container(Text::new(initials).size(10).font(fonts::Inter::EXTRA_BOLD).color(rgb!(0, 0, 0, 100%)))
            .width(24)
            .height(24)
            .align_x(alignment::Horizontal::Center)
            .align_y(alignment::Vertical::Center)
            .style(move |_| container::Style {
                background: Some(Background::Color(ink)),
                border: Border {
                    radius: 6.0.into(),
                    ..Border::default()
                },
                ..container::Style::default()
            }),
        Text::new(name).size(12).font(fonts::Inter::MEDIUM).color(color::Text::DEFAULT),
        Space::new().width(Fill),
        PulseDot::with_color(color::Primary::DEFAULT).size(6.0),
    ]
    .spacing(10)
    .align_y(alignment::Vertical::Center)
    .padding(Padding::new(0.0).top(6).bottom(6))
    .into()
}

fn screenshots_section() -> Element<'static, Message> {
    const SHOTS: [(Biome, u64, &str); 6] = [
        (Biome::End, 0, "2026-04-10_18.00.png"),
        (Biome::Nether, 1, "2026-04-11_18.01.png"),
        (Biome::Mushroom, 2, "2026-04-12_18.02.png"),
        (Biome::Ocean, 3, "2026-04-13_18.03.png"),
        (Biome::Cherry, 4, "2026-04-14_18.04.png"),
        (Biome::Snow, 5, "2026-04-15_18.05.png"),
    ];
    let shot = |(biome, seed, filename): (Biome, u64, &'static str)| screenshot_card(biome, seed, filename);

    column![
        Text::new("Screenshots")
            .size(13)
            .font(fonts::Inter::SEMI_BOLD)
            .color(color::Text::HEADLINE),
        row![shot(SHOTS[0]), shot(SHOTS[1]), shot(SHOTS[2])].spacing(10),
        row![shot(SHOTS[3]), shot(SHOTS[4]), shot(SHOTS[5])].spacing(10),
    ]
    .spacing(10)
    .into()
}

fn screenshot_card(biome: Biome, seed: u64, filename: &'static str) -> Element<'static, Message> {
    // Generated at the tile's exact pixel size, so the rounded corners stay circular.
    responsive(move |size| {
        stack![
            svg(screenshot_handle(biome, seed, size))
                .content_fit(ContentFit::Fill)
                .width(Fill)
                .height(Fill),
            container(
                Text::new(filename)
                    .size(9)
                    .font(fonts::JetBrainsMono::REGULAR)
                    .color(rgb!(255, 255, 255, 70%)),
            )
            .width(Fill)
            .height(Fill)
            .align_x(alignment::Horizontal::Left)
            .align_y(alignment::Vertical::Bottom)
            .padding(Padding::new(0.0).left(8).bottom(6)),
        ]
        .width(Fill)
        .height(Fill)
        .into()
    })
    .width(Fill)
    .height(100)
    .into()
}

fn icon(markup: &'static str, size: f32, tint: Color) -> Element<'static, Message> {
    svg(iced::widget::svg::Handle::from_memory(markup.as_bytes()))
        .width(size)
        .height(size)
        .style(move |_, _| iced::widget::svg::Style { color: Some(tint) })
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn aether() -> Instance {
        Instance::seed_pack(chrono::Local::now()).remove(0)
    }

    #[test]
    fn all_tabs_have_labels() {
        let labels: Vec<_> = Tab::ALL.iter().map(|tab| tab.label()).collect();
        assert_eq!(labels, ["Overview", "Mods", "Worlds", "Screenshots", "Logs", "Settings"]);
    }

    #[test]
    fn show_resets_the_tab_and_controls() {
        let mut page = InstancePage::default();
        assert!(page.instance().is_none());

        page.show(aether());
        page.update(Message::TabSelected(Tab::Logs));
        page.update(Message::TogglePause);
        assert_eq!(page.tab, Tab::Logs);
        assert!(page.paused, "pause is display state");

        page.show(aether());
        assert_eq!(page.tab, Tab::Overview, "opening another instance starts on Overview");
        assert!(!page.paused);
    }

    #[test]
    fn favorite_toggles() {
        let mut page = InstancePage::default();
        assert!(!page.favorite);
        page.update(Message::ToggleFavorite);
        assert!(page.favorite);
        page.update(Message::ToggleFavorite);
        assert!(!page.favorite);
    }

    #[test]
    fn playtime_format_matches_the_reference() {
        assert_eq!(format_playtime(24 * 3600 + 12 * 60), "24h 12m");
        assert_eq!(format_playtime(12 * 60), "12 min");
        assert_eq!(format_playtime(3600), "1h");
        assert_eq!(format_playtime(0), "—");
    }

    #[test]
    fn aether_meta_matches_the_reference() {
        let instance = aether();
        assert_eq!(instance.name, "Aether Legacy");
        assert_eq!(instance.mod_count, 87);
        assert_eq!(instance.directory_name, "aether-legacy");
        assert_eq!(meta_line(&instance), "/instances/aether-legacy · 4.8 GB · last saved 02:14");
    }
}
