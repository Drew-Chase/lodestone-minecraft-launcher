//! The sidebar of the Settings tab, and the scroll that keeps it in step with the page. Pressing an entry scrolls the page
//! to its section. As the page scrolls, the entry of the section at the top lights up.

use iced::advanced::widget::operate;
use iced::advanced::widget::operation::{Operation, Outcome};
use iced::widget::operation::AbsoluteOffset;
use iced::widget::scrollable::Viewport;
use iced::widget::{Id, Text, column, container, operation, row};
use iced::{Background, Border, Element, Fill, Padding, Rectangle, Task};
use iced_color_extension::rgb;

use super::{Message, Section};
use crate::app::components::TextButton;
use crate::app::components::button::Variant;
use crate::app::lang;
use crate::app::theme::{color, fonts, utility};

/// The sidebar's width, per the reference.
const SIDEBAR_WIDTH: f32 = 180.0;
/// A section is the one shown at the top once its top has reached this many pixels below the page's top edge.
const SPY_LINE: f32 = 40.0;

/// The id of the page's scrollable.
pub fn content_id() -> Id {
    Id::new("instance-settings-content")
}

/// The id of the column of sections inside the scrollable. Its top is the zero the sections are measured from.
pub fn column_id() -> Id {
    Id::new("instance-settings-column")
}

/// Reads a scroll into the page's message: the offset, and whether the page is at its bottom with more above.
pub fn on_scroll(viewport: Viewport) -> Message {
    let offset = viewport.absolute_offset().y;
    let overflow = viewport.content_bounds().height - viewport.bounds().height;
    Message::Scrolled {
        offset,
        at_end: overflow > 0.0 && offset >= overflow - 1.0,
    }
}

/// The section shown at the top of the page: the last one whose top has reached the spy line. At the bottom of the
/// page it is the last section, since the page cannot scroll the last one up to the top.
pub fn active(offsets: &[(Section, f32)], scroll: f32, at_end: bool) -> Option<Section> {
    if at_end {
        return offsets.last().map(|(section, _)| *section);
    }
    offsets
        .iter()
        .rev()
        .find(|(_, top)| *top <= scroll + SPY_LINE)
        .or_else(|| offsets.first())
        .map(|(section, _)| *section)
}

/// Scrolls the page so `section` sits at its top, measuring the sections first so the position is current.
pub fn scroll_to(section: Section) -> Task<Message> {
    measure().then(move |offsets| {
        let top = offsets.iter().find(|(found, _)| *found == section).map_or(0.0, |(_, top)| *top);
        operation::scroll_to::<Message>(content_id(), AbsoluteOffset { x: None, y: Some(top) })
    })
}

/// Measures where each section's top sits, relative to the top of the page's content.
pub fn measure() -> Task<Vec<(Section, f32)>> {
    operate(Measure {
        content: column_id(),
        top: None,
        found: Vec::new(),
    })
}

/// Collects the bounds of the page's content and of each section's container. The sections are placed relative to the
/// content, so the scroll does not change their offsets.
struct Measure {
    content: Id,
    top: Option<f32>,
    found: Vec<(Section, f32)>,
}

impl Operation<Vec<(Section, f32)>> for Measure {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<Vec<(Section, f32)>>)) {
        operate(self);
    }

    fn container(&mut self, id: Option<&Id>, bounds: Rectangle) {
        let Some(id) = id else {
            return;
        };
        if *id == self.content {
            self.top = Some(bounds.y);
        }
        if let Some(section) = Section::ALL.into_iter().find(|section| section.anchor_id() == *id) {
            self.found.push((section, bounds.y));
        }
    }

    fn finish(&self) -> Outcome<Vec<(Section, f32)>> {
        let top = self.top.unwrap_or(0.0);
        Outcome::Some(self.found.iter().map(|(section, y)| (*section, y - top)).collect())
    }
}

/// The sidebar: an entry per section, the active one lit, and the note on how many settings are inherited.
pub fn sidebar<'a>(active: Section, inherited: usize) -> Element<'a, Message> {
    let entries = column(Section::ALL.map(|section| entry(section, section == active)))
        .spacing(2)
        .width(Fill);
    let mut sidebar = column![entries].spacing(18).width(SIDEBAR_WIDTH);
    if inherited > 0 {
        sidebar = sidebar.push(note(inherited));
    }
    sidebar.into()
}

/// One sidebar entry: its glyph and label, lit green when its section is the active one.
fn entry<'a>(section: Section, active: bool) -> Element<'a, Message> {
    let ink = if active { color::Primary::DEFAULT } else { color::Text::MUTED };
    container(
        TextButton::content(
            row![
                utility::icon(section.icon(), 14.0, Some((ink, ink))),
                Text::new(section.label()).size(13).font(fonts::Inter::MEDIUM).color(ink),
            ]
            .spacing(10)
            .align_y(iced::alignment::Vertical::Center),
        )
        .variant(Variant::Tab)
        .width(Fill)
        .padding(Padding::new(0.0).left(12).right(12).top(8).bottom(8))
        .on_press(Message::Section(section)),
    )
    .width(Fill)
    .style(move |_| container::Style {
        background: active.then_some(Background::Color(color::Primary::TINT)),
        border: Border {
            radius: 10.0.into(),
            ..Border::default()
        },
        ..container::Style::default()
    })
    .into()
}

/// The sidebar's note: the label, and how many settings take the global default.
fn note<'a>(count: usize) -> Element<'a, Message> {
    container(
        column![
            Text::new(lang::t("instance.settings.inherited_label"))
                .size(10)
                .font(fonts::JetBrainsMono::SEMI_BOLD)
                .color(color::Text::PLACEHOLDER),
            Text::new(lang::t_with(
                "instance.settings.inherited",
                &[("settings", lang::t_count("instance.settings_count", count).as_str())],
            ))
            .size(11)
            .font(fonts::Inter::REGULAR)
            .color(color::Text::MUTED)
            .line_height(1.5),
        ]
        .spacing(8),
    )
    .width(Fill)
    .padding(14)
    .style(|_| container::Style {
        background: Some(Background::Color(rgb!(255, 255, 255, 2%))),
        border: Border {
            radius: 12.0.into(),
            width: 1.0,
            color: color::Surface::LINE,
        },
        ..container::Style::default()
    })
    .into()
}

#[cfg(test)]
#[path = "../../../../../tests/app/pages/instance/settings/nav.rs"]
mod tests;
