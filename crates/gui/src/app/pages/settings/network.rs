//! The Network tab: the download speed, the requests every client makes, and the proxy they go through.

pub mod speed;

use iced::widget::{Space, canvas, column, container, row, text, text_input};
use iced::{Background, Border, Element, Fill, Length, Padding, Point, Rectangle, Renderer, Theme, alignment, mouse};

use self::speed::Meter;
use super::edit::Edit;
use super::{Message, card, plain_card};
use crate::app::components::input;
use crate::app::components::panel::divided;
use crate::app::components::setting_row::described_setting;
use crate::app::components::{Select, Toggle};
use crate::app::lang;
use crate::app::preferences::{CONCURRENT_DOWNLOADS, DownloadMirror, Network, SettingsValues, SourceOrder};
use crate::app::theme::{color, fonts};

/// The connection counts the select offers.
const CONNECTIONS: [u16; 6] = [1, 2, 4, 8, 16, 32];
/// The timeouts the select offers, in seconds.
const TIMEOUTS: [u32; 4] = [10, 30, 60, 120];
/// The source orders the select offers, in order.
const SOURCE_ORDERS: [SourceOrder; 2] = [SourceOrder::ModrinthFirst, SourceOrder::CurseforgeFirst];
/// The width of the selects on this tab.
const SELECT_WIDTH: f32 = 240.0;
/// The height of the speed chart.
const CHART_HEIGHT: f32 = 96.0;
/// Bytes in a megabyte, which the speed is shown in.
const MEGABYTE: f64 = 1_000_000.0;

/// The Network tab: the speed card, the request settings, and the proxy card.
pub fn view<'a>(settings: &'a SettingsValues, meter: &'a Meter, connected: Option<bool>) -> Element<'a, Message> {
    column![
        card(
            lang::t("settings.network.speed_title"),
            lang::t("settings.network.speed_subtitle"),
            speed_body(meter, connected),
        ),
        plain_card(divided(request_rows(&settings.network))),
        card(
            lang::t("settings.network.proxy_title"),
            lang::t("settings.network.proxy_subtitle"),
            divided(proxy_rows(&settings.network)),
        ),
    ]
    .spacing(24)
    .width(Fill)
    .into()
}

/// The average speed in megabytes per second, the connection state, and the chart of the minutes.
fn speed_body<'a>(meter: &Meter, connected: Option<bool>) -> Element<'a, Message> {
    let rate = meter.average() / MEGABYTE;
    let figure = row![
        text(format!("{rate:.1}"))
            .size(44)
            .font(fonts::JetBrainsMono::SEMI_BOLD)
            .color(color::primary()),
        text(lang::t("settings.network.speed_unit"))
            .size(14)
            .font(fonts::JetBrainsMono::REGULAR)
            .color(color::text_muted()),
    ]
    .spacing(10)
    .align_y(alignment::Vertical::Bottom);
    let header = row![figure, Space::new().width(Fill), connection_badge(connected)].align_y(alignment::Vertical::Center);
    let chart = canvas(Sparkline { points: meter.minutes() })
        .width(Fill)
        .height(Length::Fixed(CHART_HEIGHT));
    column![header, chart].spacing(18).into()
}

/// Whether the launcher reached its services in the last probe. Until a probe lands, it says it is checking.
fn connection_badge<'a>(connected: Option<bool>) -> Element<'a, Message> {
    let (label, ink) = match connected {
        Some(true) => (lang::t("settings.network.connected"), color::primary()),
        Some(false) => (lang::t("settings.network.offline"), color::buttons::danger_background_hover()),
        None => (lang::t("settings.network.checking"), color::text_muted()),
    };
    container(text(label).size(10).font(fonts::JetBrainsMono::SEMI_BOLD).color(ink))
        .padding(Padding::new(0.0).left(10.0).right(10.0).top(4.0).bottom(4.0))
        .style(|_| container::Style {
            background: Some(Background::Color(color::surface_bg3())),
            border: Border {
                radius: 999.0.into(),
                width: 1.0,
                color: color::surface_line(),
            },
            ..container::Style::default()
        })
        .into()
}

/// The chart: a line through the minute averages, with the area under it washed in the accent.
struct Sparkline {
    points: Vec<f32>,
}

impl<Message> canvas::Program<Message> for Sparkline {
    type State = ();

    fn draw(&self, _state: &(), renderer: &Renderer, _theme: &Theme, bounds: Rectangle, _cursor: mouse::Cursor) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        if self.points.len() >= 2 {
            let top = self.points.iter().copied().fold(1.0_f32, f32::max);
            let step = bounds.width / (self.points.len() - 1) as f32;
            let at = |index: usize| Point::new(index as f32 * step, bounds.height - self.points[index] / top * bounds.height);
            let line = canvas::Path::new(|path| {
                path.move_to(at(0));
                for index in 1..self.points.len() {
                    path.line_to(at(index));
                }
            });
            let area = canvas::Path::new(|path| {
                path.move_to(Point::new(0.0, bounds.height));
                path.line_to(at(0));
                for index in 1..self.points.len() {
                    path.line_to(at(index));
                }
                path.line_to(Point::new(bounds.width, bounds.height));
                path.close();
            });
            frame.fill(&area, color::primary_tint());
            frame.stroke(&line, canvas::Stroke::default().with_color(color::primary()).with_width(2.0));
        }
        vec![frame.into_geometry()]
    }
}

/// The request settings: the files fetched at once, the connections kept per host, the source order, the mirror, and the timeout.
fn request_rows<'a>(network: &Network) -> Vec<Element<'a, Message>> {
    vec![
        described_setting(
            lang::t("settings.general.downloads"),
            lang::t("settings.general.downloads_subtitle"),
            concurrency_slider(network.concurrent_downloads),
        ),
        described_setting(
            lang::t("settings.network.connections"),
            lang::t("settings.network.connections_hint"),
            connections_select(network.connections_per_host),
        ),
        described_setting(
            lang::t("settings.network.source_order"),
            lang::t("settings.network.source_order_hint"),
            source_select(network.source_order),
        ),
        described_setting(
            lang::t("settings.network.mirror"),
            lang::t("settings.network.mirror_hint"),
            mirror_select(),
        ),
        described_setting(
            lang::t("settings.network.timeout"),
            lang::t("settings.network.timeout_hint"),
            timeout_select(network.timeout_secs),
        ),
    ]
}

/// The slider for how many files an install fetches at once, with its value.
fn concurrency_slider<'a>(count: u16) -> Element<'a, Message> {
    let (low, high) = (*CONCURRENT_DOWNLOADS.start(), *CONCURRENT_DOWNLOADS.end());
    let bar = iced::widget::slider(f32::from(low)..=f32::from(high), f32::from(count), |value| {
        Message::Edit(Edit::ConcurrentDownloads(value.round() as u16))
    })
    .step(1.0_f32)
    .style(crate::app::components::slider::style)
    .width(Fill);
    row![
        bar,
        text(count.to_string())
            .size(14)
            .font(fonts::JetBrainsMono::SEMI_BOLD)
            .color(color::primary()),
    ]
    .spacing(16)
    .align_y(alignment::Vertical::Center)
    .into()
}

/// The connections select: how many idle connections a host keeps.
fn connections_select<'a>(current: u16) -> Element<'a, Message> {
    let labels: Vec<String> = CONNECTIONS.iter().map(u16::to_string).collect();
    Select::new(labels, CONNECTIONS.iter().position(|count| *count == current))
        .on_select(|index| Message::Edit(Edit::ConnectionsPerHost(CONNECTIONS[index])))
        .width(SELECT_WIDTH)
        .into()
}

/// The source order select: which platform a file is looked up on first.
fn source_select<'a>(current: SourceOrder) -> Element<'a, Message> {
    Select::new(
        [lang::t("settings.network.source_modrinth"), lang::t("settings.network.source_curseforge")],
        SOURCE_ORDERS.iter().position(|order| *order == current),
    )
    .on_select(|index| Message::Edit(Edit::SourceOrder(SOURCE_ORDERS[index])))
    .width(SELECT_WIDTH)
    .into()
}

/// The mirror select. Mojang's servers are the only source, so the select has the one choice.
fn mirror_select<'a>() -> Element<'a, Message> {
    Select::new([lang::t("settings.network.mirror_auto")], Some(0))
        .on_select(|_| Message::Edit(Edit::Mirror(DownloadMirror::Auto)))
        .width(SELECT_WIDTH)
        .into()
}

/// The timeout select, in seconds.
fn timeout_select<'a>(current: u32) -> Element<'a, Message> {
    let labels: Vec<String> = TIMEOUTS
        .iter()
        .map(|seconds| lang::t_with("settings.network.seconds", &[("s", seconds.to_string().as_str())]))
        .collect();
    Select::new(labels, TIMEOUTS.iter().position(|seconds| *seconds == current))
        .on_select(|index| Message::Edit(Edit::TimeoutSecs(TIMEOUTS[index])))
        .width(SELECT_WIDTH)
        .into()
}

/// The proxy rows: the system proxy switch, the custom proxy switch, and the address when the custom proxy is on.
fn proxy_rows<'a>(network: &Network) -> Vec<Element<'a, Message>> {
    let mut rows = vec![
        described_setting(
            lang::t("settings.network.system_proxy"),
            lang::t("settings.network.system_proxy_hint"),
            Toggle::new(network.use_system_proxy).on_toggle(Message::Edit(Edit::UseSystemProxy(!network.use_system_proxy))),
        ),
        described_setting(
            lang::t("settings.network.custom_proxy"),
            lang::t("settings.network.custom_proxy_hint"),
            Toggle::new(network.custom_proxy_enabled).on_toggle(Message::Edit(Edit::CustomProxyEnabled(!network.custom_proxy_enabled))),
        ),
    ];
    if network.custom_proxy_enabled {
        rows.push(described_setting(
            lang::t("settings.network.proxy_address"),
            lang::t("settings.network.proxy_address_hint"),
            proxy_field(&network.custom_proxy),
        ));
    }
    rows
}

/// The field that types the proxy address.
fn proxy_field<'a>(address: &str) -> Element<'a, Message> {
    text_input(lang::t("settings.network.proxy_placeholder"), address)
        .on_input(|text| Message::Edit(Edit::CustomProxy(text)))
        .style(input::style)
        .font(fonts::JetBrainsMono::REGULAR)
        .size(13)
        .padding(Padding::new(0.0).left(12.0).right(12.0).top(8.0).bottom(8.0))
        .width(300)
        .into()
}
