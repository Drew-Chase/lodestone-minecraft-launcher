//! The Scan and Exporting screens: a title, a bar, the figures so far, and a button that stops the work.

use iced::alignment::Horizontal;
use iced::widget::{Space, column, container, row, text};
use iced::{Alignment, Background, Border, Element, Fill, Padding};

use super::{Message, State};
use crate::app::components::{ProgressBar, TextButton, typography};
use crate::app::lang;
use crate::app::theme::{color, fonts};

/// The widest the column of a progress screen grows.
const COLUMN_WIDTH: f32 = 640.0;

/// The scan screen: which jar is read, how far along the scan is, and how the jars so far fell on each side.
pub fn scanning(state: &State) -> Element<'_, Message> {
    let (index, total) = (state.scan.index, state.scan.total);
    let line = if total == 0 {
        lang::t("server_pack.scan.opening").to_string()
    } else {
        let (index, total) = (index.to_string(), total.to_string());
        lang::t_with(
            "server_pack.scan.reading",
            &[("INDEX", index.as_str()), ("TOTAL", total.as_str()), ("NAME", state.scan.name.as_str())],
        )
    };
    let fraction = if total == 0 { 0.0 } else { index as f32 / total as f32 };
    let tally = state.tally;
    let chips = row![
        chip(color::cyan(), tally.client, lang::t("server_pack.scan.tally.client")),
        chip(color::primary(), tally.server, lang::t("server_pack.scan.tally.server")),
        chip(color::text_muted(), tally.both, lang::t("server_pack.scan.tally.both")),
        chip(color::amber(), tally.unsure, lang::t("server_pack.scan.tally.unsure")),
    ]
    .spacing(12);
    screen(
        lang::t("server_pack.scan.title"),
        line,
        ProgressBar::new(fraction).indeterminate(total == 0).width(Fill).into(),
        Some(chips.into()),
        TextButton::ghost(lang::t("server_pack.scan.cancel")).on_press(Message::Cancel).into(),
    )
}

/// The export screen: how many files are written, and the button that stops the export.
pub fn exporting(state: &State) -> Element<'_, Message> {
    let (done, total) = (state.export.done, state.export.total);
    let fraction = if total == 0 { 0.0 } else { done as f32 / total as f32 };
    let line = if total == 0 {
        let stage = if state.installing {
            "server_pack.exporting.installing"
        } else {
            "server_pack.exporting.subtitle"
        };
        lang::t(stage).to_string()
    } else {
        let (done, total) = (done.to_string(), total.to_string());
        lang::t_with("server_pack.exporting.progress", &[("DONE", done.as_str()), ("TOTAL", total.as_str())])
    };
    screen(
        lang::t("server_pack.exporting.title"),
        line,
        ProgressBar::new(fraction).indeterminate(total == 0).width(Fill).into(),
        None,
        TextButton::ghost(lang::t("server_pack.exporting.cancel"))
            .on_press(Message::Cancel)
            .into(),
    )
}

/// A screen with a title, a line under it, a bar, an optional row of figures, and a button.
fn screen<'a>(
    title: &'static str,
    line: String,
    bar: Element<'a, Message>,
    figures: Option<Element<'a, Message>>,
    button: Element<'a, Message>,
) -> Element<'a, Message> {
    let mut body = column![
        typography::h2(title),
        text(line).size(12.5).font(fonts::JetBrainsMono::REGULAR).color(color::text_muted()),
        bar,
    ]
    .spacing(14)
    .width(Fill);
    if let Some(figures) = figures {
        body = body.push(figures);
    }
    body = body.push(Space::new().height(8));
    body = body.push(button);
    container(
        container(body)
            .max_width(COLUMN_WIDTH)
            .width(Fill)
            .padding(Padding::new(0.0).left(8.0).right(8.0)),
    )
    .width(Fill)
    .height(Fill)
    .align_x(Horizontal::Center)
    .align_y(Alignment::Center)
    .padding(Padding::new(32.0))
    .into()
}

/// A count with its side's dot and label, in a bordered chip.
fn chip<'a>(dot: iced::Color, count: usize, label: &'static str) -> Element<'a, Message> {
    let figure = count.to_string();
    container(
        row![
            container(Space::new()).width(8).height(8).style(move |_| container::Style {
                background: Some(Background::Color(dot)),
                border: Border {
                    radius: 2.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }),
            text(figure).size(14.0).font(fonts::Inter::BOLD).color(color::text_default()),
            text(label).size(12.0).font(fonts::Inter::MEDIUM).color(color::text_muted()),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .padding(Padding::new(0.0).left(14.0).right(14.0).top(9.0).bottom(9.0))
    .style(|_| container::Style {
        background: Some(Background::Color(color::surface_bg2())),
        border: Border {
            color: color::surface_line(),
            width: 1.0,
            radius: 8.0.into(),
        },
        ..Default::default()
    })
    .into()
}
