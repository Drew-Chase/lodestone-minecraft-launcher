//! The recent imports: the ones queued in the last seven days, each with a Reimport button.

use aurora_leaf::Icon;
use chrono::{DateTime, Local};
use iced::widget::{Space, Text, column, container, row, svg};
use iced::{Background, Border, Element, Fill, Padding, alignment};
use iced_color_extension::rgb;

use super::format_label;
use crate::app::components::button::{TextButton, Variant};
use crate::app::import::Message;
use crate::app::import::record::ImportRecord;
use crate::app::lang;
use crate::app::pages::instance::format::format_bytes;
use crate::app::pages::worlds::format::format_ago;
use crate::app::theme::{color, fonts};

/// The side of a row's icon tile.
const TILE: f32 = 36.0;

/// The section: its label and window, then a row per import, or the line that says there are none.
pub fn view(recent: &[ImportRecord], busy: bool) -> Element<'static, Message> {
    let header = row![
        Text::new(lang::t("import.recent.label"))
            .size(11)
            .font(fonts::JetBrainsMono::REGULAR)
            .color(color::text_muted()),
        Space::new().width(Fill),
        Text::new(lang::t("import.recent.window"))
            .size(11)
            .font(fonts::JetBrainsMono::REGULAR)
            .color(color::text_muted()),
    ]
    .align_y(alignment::Vertical::Center);
    let now = Local::now();
    let list: Element<'static, Message> = if recent.is_empty() {
        Text::new(lang::t("import.recent.empty"))
            .size(12)
            .font(fonts::Inter::REGULAR)
            .color(color::text_muted())
            .into()
    } else {
        column(recent.iter().enumerate().map(|(index, record)| entry(index, record, busy, now)))
            .spacing(8)
            .into()
    };
    column![header, list].spacing(10).into()
}

/// One import: its folder tile, its name, its format, size and age, and the button that starts it again.
fn entry(index: usize, record: &ImportRecord, busy: bool, now: DateTime<Local>) -> Element<'static, Message> {
    let glyph = Icon::lucide().folder();
    let icon = svg(svg::Handle::from_memory(glyph.as_bytes()))
        .width(16.0)
        .height(16.0)
        .style(|_, _| svg::Style { color: Some(color::cyan()) });
    let tile = container(icon)
        .width(TILE)
        .height(TILE)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .style(|_| container::Style {
            background: Some(Background::Color(color::cyan_tint())),
            border: Border {
                radius: 10.0.into(),
                ..Border::default()
            },
            ..container::Style::default()
        });
    let meta = format!(
        "{} · {} · {}",
        format_label(record.format),
        format_bytes(record.bytes),
        format_ago(record.imported_at, now)
    );
    let details = column![
        Text::new(record.name.clone())
            .size(14)
            .font(fonts::Inter::SEMI_BOLD)
            .color(color::text_headline()),
        Text::new(meta).size(11).font(fonts::JetBrainsMono::REGULAR).color(color::text_muted()),
    ]
    .spacing(3)
    .width(Fill);
    let mut reimport = TextButton::content(
        Text::new(lang::t("import.recent.reimport"))
            .size(13)
            .font(fonts::Inter::SEMI_BOLD)
            .color(color::text_default()),
    )
    .variant(Variant::Ghost)
    .padding(Padding::new(0.0).left(14).right(14).top(7).bottom(7));
    if !busy {
        reimport = reimport.on_press(Message::Reimport(index));
    }
    container(row![tile, details, reimport].spacing(14).align_y(alignment::Vertical::Center))
        .padding(Padding::new(0.0).left(12).right(12).top(10).bottom(10))
        .width(Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(rgb!(255, 255, 255, 3%))),
            border: Border {
                radius: 12.0.into(),
                width: 1.0,
                color: color::surface_line(),
            },
            ..container::Style::default()
        })
        .into()
}
