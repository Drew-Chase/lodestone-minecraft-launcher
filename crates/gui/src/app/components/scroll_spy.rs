//! Scroll-spy for a page of sections. A sidebar lights the section at the top of the page, and pressing an entry scrolls the
//! page to that section. Sections are measured from the containers that carry their anchor ids, relative to the column that
//! holds them, so a scroll does not move them.

use iced::advanced::widget::operate;
use iced::advanced::widget::operation::{Operation, Outcome};
use iced::widget::Id;
use iced::widget::operation::{self, AbsoluteOffset};
use iced::widget::scrollable::Viewport;
use iced::{Rectangle, Task};

/// A section is the active one once its top has reached this many pixels below the page's top edge.
const SPY_LINE: f32 = 40.0;

/// The scroll's offset, and whether the page is at its bottom with more above it.
pub fn position(viewport: Viewport) -> (f32, bool) {
    let offset = viewport.absolute_offset().y;
    let overflow = viewport.content_bounds().height - viewport.bounds().height;
    (offset, overflow > 0.0 && offset >= overflow - 1.0)
}

/// The section at the top of the page: the last whose top has reached the spy line. At the bottom of the page it is the
/// last section, since the page cannot scroll the last one up to the top.
pub fn active<T: Copy>(offsets: &[(T, f32)], scroll: f32, at_end: bool) -> Option<T> {
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

/// Measures where each section's top sits relative to `column`, the container that holds every section.
pub fn measure<T>(column: Id, sections: Vec<(T, Id)>) -> Task<Vec<(T, f32)>>
where
    T: Copy + Send + 'static,
{
    operate(Measure {
        column,
        sections,
        top: None,
        found: Vec::new(),
    })
}

/// Scrolls `content` so the section `target` sits at its top. The sections are measured first, so the position is current.
pub fn scroll_to<T, M>(target: T, content: Id, column: Id, sections: Vec<(T, Id)>) -> Task<M>
where
    T: Copy + PartialEq + Send + 'static,
    M: Send + 'static,
{
    measure(column, sections).then(move |offsets| {
        let top = offsets.iter().find(|(found, _)| *found == target).map_or(0.0, |(_, top)| *top);
        operation::scroll_to(content.clone(), AbsoluteOffset { x: None, y: Some(top) })
    })
}

/// Collects the bounds of the column and of each section's anchor. The sections are placed relative to the column, so the
/// scroll does not change their offsets.
struct Measure<T> {
    column: Id,
    sections: Vec<(T, Id)>,
    top: Option<f32>,
    found: Vec<(T, f32)>,
}

impl<T: Copy + Send + 'static> Operation<Vec<(T, f32)>> for Measure<T> {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<Vec<(T, f32)>>)) {
        operate(self);
    }

    fn container(&mut self, id: Option<&Id>, bounds: Rectangle) {
        let Some(id) = id else {
            return;
        };
        if *id == self.column {
            self.top = Some(bounds.y);
        }
        if let Some((section, _)) = self.sections.iter().find(|(_, anchor)| anchor == id) {
            self.found.push((*section, bounds.y));
        }
    }

    fn finish(&self) -> Outcome<Vec<(T, f32)>> {
        let top = self.top.unwrap_or(0.0);
        Outcome::Some(self.found.iter().map(|(section, y)| (*section, y - top)).collect())
    }
}

#[cfg(test)]
#[path = "../../../tests/app/components/scroll_spy.rs"]
mod tests;
