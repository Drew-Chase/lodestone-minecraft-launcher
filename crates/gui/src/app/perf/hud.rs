//! The F3 overlay: the last second's figures in the window's top-right corner. It takes no pointer
//! events of its own, so the widgets beneath it keep their clicks and cursors.

use std::time::Duration;

use iced::keyboard::key::Named;
use iced::widget::{Space, column, container, row, text};
use iced::{Background, Border, Color, Element, Event, Fill, Length, Subscription, alignment, event, keyboard, window};

use super::Message;
use super::recorder::Stat;
use super::summary::{self, Figures};
use crate::app::lang;
use crate::app::theme::{color, fonts};

/// The overlay's state: whether it is shown.
#[derive(Debug, Default)]
pub struct Hud {
    visible: bool,
}

impl Hud {
    pub fn update(&mut self, message: Message) {
        if message == Message::Toggle {
            self.visible = !self.visible;
        }
    }

    fn visible(&self) -> bool {
        self.visible
    }
}

/// The F3 key, and the one-second refresh while the overlay is shown.
pub fn subscription(hud: &Hud) -> Subscription<Message> {
    let toggle = event::listen_with(toggle_on_f3);
    if hud.visible() {
        Subscription::batch([toggle, iced::time::every(Duration::from_secs(1)).map(|_| Message::Refresh)])
    } else {
        toggle
    }
}

fn toggle_on_f3(event: Event, _status: event::Status, _window: window::Id) -> Option<Message> {
    match event {
        Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Named(Named::F3),
            ..
        }) => Some(Message::Toggle),
        _ => None,
    }
}

/// The overlay for the window: nothing while hidden, and a waiting line until the first report.
pub fn overlay<'a, M: 'a>(hud: &Hud) -> Element<'a, M> {
    if !hud.visible() {
        return Space::new().into();
    }
    let panel: Element<'a, M> = match summary::latest() {
        Some(figures) => figures_panel(&figures),
        None => text(lang::t("perf.hud.waiting"))
            .size(12)
            .font(fonts::JetBrainsMono::REGULAR)
            .color(color::Text::MUTED)
            .into(),
    };
    container(panel)
        .width(Fill)
        .height(Fill)
        .align_x(alignment::Horizontal::Right)
        .align_y(alignment::Vertical::Top)
        .padding(12)
        .into()
}

/// The figures as labelled rows, in a dark panel.
fn figures_panel<'a, M: 'a>(figures: &Figures) -> Element<'a, M> {
    let slowest = match figures.slowest.first() {
        Some((name, stat)) => lang::t_with("perf.hud.slowest", &[("NAME", name), ("MEAN", &millis(stat.mean()))]),
        None => lang::t("perf.hud.none").to_owned(),
    };
    let rows: Vec<Element<'a, M>> = vec![
        text(lang::t("perf.hud.title"))
            .size(13)
            .font(fonts::Inter::SEMI_BOLD)
            .color(color::Text::HEADLINE)
            .into(),
        line(lang::t("perf.hud.frames"), rate(figures.frames_per_s)),
        line(lang::t("perf.hud.draw"), mean_and_max(figures.draw)),
        line(lang::t("perf.hud.layout"), mean_and_max(figures.layout)),
        line(
            lang::t("perf.hud.updates"),
            lang::t_with(
                "perf.hud.updates_value",
                &[("RATE", &rate(figures.updates_per_s)), ("MAX", &millis(figures.update.max))],
            ),
        ),
        line(lang::t("perf.hud.view"), mean_and_max(figures.view)),
        line(
            lang::t("perf.hud.svg"),
            lang::t_with(
                "perf.hud.svg_value",
                &[
                    ("RATE", &rate(figures.svg_per_s)),
                    ("MPX", &format!("{:.2}", figures.svg_pixels_per_s / 1_000_000.0)),
                ],
            ),
        ),
        line(
            lang::t("perf.hud.atlas"),
            lang::t_with(
                "perf.hud.atlas_value",
                &[("ALLOC", &rate(figures.atlas_allocs_per_s)), ("EVICT", &rate(figures.atlas_evicts_per_s))],
            ),
        ),
        line(
            lang::t("perf.hud.art"),
            lang::t_with(
                "perf.hud.art_value",
                &[("HITS", &figures.art_hits.to_string()), ("MISSES", &figures.art_misses.to_string())],
            ),
        ),
        line(lang::t("perf.hud.slowest_scope"), slowest),
        line(
            lang::t("perf.hud.dropped"),
            lang::t_with(
                "perf.hud.dropped_value",
                &[
                    ("LINES", &figures.dropped_lines.to_string()),
                    ("EVENTS", &figures.dropped_trace_events.to_string()),
                ],
            ),
        ),
    ];
    container(column(rows).spacing(6))
        .padding(12)
        .width(Length::Fixed(380.0))
        .style(|_| container::Style {
            background: Some(Background::Color(Color::from_rgba(0.02, 0.03, 0.04, 0.88))),
            border: Border {
                radius: 10.0.into(),
                ..Border::default()
            },
            ..container::Style::default()
        })
        .into()
}

/// One labelled row: the label on the left, its value on the right.
fn line<'a, M: 'a>(label: &'static str, value: String) -> Element<'a, M> {
    row![
        text(label).size(12).font(fonts::Inter::REGULAR).color(color::Text::MUTED),
        Space::new().width(Fill),
        text(value).size(12).font(fonts::JetBrainsMono::REGULAR).color(color::Text::HEADLINE),
    ]
    .width(Fill)
    .align_y(alignment::Vertical::Center)
    .into()
}

/// A per-second rate, such as `60/s`.
fn rate(per_second: f64) -> String {
    lang::t_with("perf.hud.rate", &[("VALUE", &format!("{per_second:.0}"))])
}

/// A duration in nanoseconds, in milliseconds.
fn millis(nanos: u64) -> String {
    lang::t_with("perf.hud.ms", &[("VALUE", &format!("{:.2}", nanos as f64 / 1_000_000.0))])
}

/// The mean and the largest value of a timing.
fn mean_and_max(stat: Stat) -> String {
    lang::t_with("perf.hud.mean_max", &[("MEAN", &millis(stat.mean())), ("MAX", &millis(stat.max))])
}
