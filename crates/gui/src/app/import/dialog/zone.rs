//! The drop zone of the file tabs: a dashed frame that files are dropped on or browsed to, and the file it holds once it is
//! checked. iced draws no dashed border, so the frame is stroked on a canvas.

use iced::widget::canvas::{self, Canvas};
use iced::widget::container::Style as ContainerStyle;
use iced::widget::{Column, Text, column, container, mouse_area, row, stack};
use iced::{Background, Border, Element, Fill, Length, Padding, Point, Rectangle, Renderer, Size, Theme, alignment, border, mouse};
use iced_color_extension::rgb;

use super::format_label;
use crate::app::components::button::TextButton;
use crate::app::import::Message;
use crate::app::import::prepare::Picked;
use crate::app::lang;
use crate::app::pages::instance::format::format_bytes;
use crate::app::theme::{color, fonts};

/// The height of the zone.
const HEIGHT: f32 = 190.0;
/// The width of the dashed line, and the lengths of its dashes and gaps.
const LINE: f32 = 1.5;
const DASHES: [f32; 2] = [8.0, 6.0];
/// The radius of the zone's corners.
const RADIUS: f32 = 16.0;

/// The zone of a file tab: the prompt while nothing is checked, or the file it holds. A press on it opens the file
/// picker, unless a read is under way. A file dragged over the window lights its frame.
pub fn view(backup: bool, picked: Option<&Picked>, hovered: bool, busy: bool) -> Element<'static, Message> {
    let content: Element<'static, Message> = match picked {
        Some(picked) => checked(picked, busy),
        None => prompt(backup, busy),
    };
    let frame = Canvas::new(Frame { hovered }).width(Fill).height(Length::Fixed(HEIGHT));
    let body = container(content).center(Fill);
    let zone = stack![frame, body];
    if busy {
        zone.into()
    } else {
        mouse_area(zone).on_press(Message::Browse).interaction(mouse::Interaction::Pointer).into()
    }
}

/// The prompt of an empty zone: the tiles of the formats it takes, the title, and the line that offers the picker.
fn prompt(backup: bool, busy: bool) -> Element<'static, Message> {
    let labels: &[&'static str] = if backup { &[".lodestone"] } else { &[".zip", ".mrpack", ".mcpack"] };
    let tiles = row(labels.iter().map(|label| tile(label))).spacing(12);
    let hint = row![
        Text::new(lang::t("import.drop.or"))
            .size(12)
            .font(fonts::Inter::REGULAR)
            .color(color::text_muted()),
        browse_link(lang::t("import.drop.browse"), busy),
        Text::new(lang::t("import.drop.limit"))
            .size(12)
            .font(fonts::JetBrainsMono::REGULAR)
            .color(color::text_muted()),
    ]
    .spacing(6)
    .align_y(alignment::Vertical::Center);
    let title = if backup {
        lang::t("import.drop.backup_title")
    } else {
        lang::t("import.drop.title")
    };
    column![
        tiles,
        Text::new(title).size(15).font(fonts::Inter::SEMI_BOLD).color(color::text_headline()),
        hint,
    ]
    .spacing(12)
    .align_x(alignment::Horizontal::Center)
    .into()
}

/// The zone once a file is checked: its name, its format and size, and the line that picks another.
fn checked(picked: &Picked, busy: bool) -> Element<'static, Message> {
    let detail = format!("{} · {}", format_label(picked.format), format_bytes(picked.bytes));
    let column: Column<'static, Message> = column![
        Text::new(picked.name.clone())
            .size(15)
            .font(fonts::Inter::SEMI_BOLD)
            .color(color::text_headline()),
        Text::new(detail).size(12).font(fonts::JetBrainsMono::REGULAR).color(color::text_muted()),
        browse_link(lang::t("import.drop.change"), busy),
    ]
    .spacing(8)
    .align_x(alignment::Horizontal::Center);
    column.into()
}

/// A format's tile in the prompt, labelled with its extension.
fn tile(label: &'static str) -> Element<'static, Message> {
    container(Text::new(label).size(11).font(fonts::JetBrainsMono::REGULAR).color(color::cyan()))
        .padding(Padding::new(0.0).left(12).right(12).top(8).bottom(8))
        .style(|_| ContainerStyle {
            background: Some(Background::Color(rgb!(255, 255, 255, 4%))),
            border: Border {
                radius: 8.0.into(),
                width: 1.0,
                color: rgb!(71, 217, 255, 35%),
            },
            ..ContainerStyle::default()
        })
        .into()
}

/// The text link that opens the file picker. It is inert while a read is under way.
fn browse_link(label: &'static str, busy: bool) -> Element<'static, Message> {
    let link = TextButton::ghost(label).padding(Padding::new(0.0).left(4).right(4).top(2).bottom(2));
    if busy { link.into() } else { link.on_press(Message::Browse).into() }
}

/// The dashed frame: a rounded outline, stroked in dashes, which brightens while a file is dragged over the window.
struct Frame {
    hovered: bool,
}

impl canvas::Program<Message> for Frame {
    type State = ();

    fn draw(&self, _state: &(), renderer: &Renderer, _theme: &Theme, bounds: Rectangle, _cursor: mouse::Cursor) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let inset = LINE / 2.0;
        let size = Size::new(bounds.width - LINE, bounds.height - LINE);
        let outline = canvas::Path::rounded_rectangle(Point::new(inset, inset), size, border::Radius::from(RADIUS));
        let ink = if self.hovered { color::primary() } else { color::cyan() };
        let stroke = canvas::Stroke {
            width: LINE,
            style: canvas::stroke::Style::Solid(ink),
            line_dash: canvas::LineDash {
                segments: &DASHES,
                offset: 0,
            },
            ..canvas::Stroke::default()
        };
        frame.stroke(&outline, stroke);
        vec![frame.into_geometry()]
    }
}
