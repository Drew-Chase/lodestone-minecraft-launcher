//! A carousel: a row that shows a fixed number of items, with a button at each end to page
//! through the rest. The row never scrolls on its own, so it never grows a scrollbar.

use aurora_leaf::Icon;
use iced::widget::{container, row};
use iced::{Element, alignment};

use crate::app::components::IconButton;

/// The space between the buttons, the items and the cells.
const GAP: f32 = 8.0;
/// The edge of a navigation button, and the size of its glyph.
const BUTTON: f32 = 28.0;
const GLYPH: f32 = 14.0;

/// Which way a carousel pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Back,
    Forward,
}

/// The first item shown for `offset`. The window holds a full page of `visible` items, so
/// an offset past the end settles on the last full page.
pub fn window_start(offset: usize, len: usize, visible: usize) -> usize {
    offset.min(len.saturating_sub(visible))
}

/// The offset one page `step` away from `offset`, held to the last full page.
pub fn stepped(offset: usize, len: usize, visible: usize, step: Step) -> usize {
    let start = window_start(offset, len, visible);
    match step {
        Step::Back => window_start(start.saturating_sub(visible), len, visible),
        Step::Forward => window_start(start + visible, len, visible),
    }
}

/// The offset whose page shows item `index`.
pub fn revealing(index: usize, len: usize, visible: usize) -> usize {
    window_start(index / visible * visible, len, visible)
}

/// Whether a page `step` away from `offset` would show anything not already on screen.
pub fn can_step(offset: usize, len: usize, visible: usize, step: Step) -> bool {
    let start = window_start(offset, len, visible);
    match step {
        Step::Back => start > 0,
        Step::Forward => start + visible < len,
    }
}

/// The carousel over `items`: `visible` of them from `offset`, each in a `cell`-pixel square,
/// between a back and a forward button. `on_step` builds the message for a button.
pub fn carousel<'a, M: Clone + 'a>(
    items: Vec<Element<'a, M>>,
    offset: usize,
    visible: usize,
    cell: f32,
    on_step: impl Fn(Step) -> M,
) -> Element<'a, M> {
    let len = items.len();
    let shown: Vec<Element<'a, M>> = items
        .into_iter()
        .skip(window_start(offset, len, visible))
        .take(visible)
        .map(|item| container(item).width(cell).height(cell).into())
        .collect();
    let back = nav(
        Icon::lucide().chevron_left(),
        can_step(offset, len, visible, Step::Back),
        on_step(Step::Back),
    );
    let forward = nav(
        Icon::lucide().chevron_right(),
        can_step(offset, len, visible, Step::Forward),
        on_step(Step::Forward),
    );
    row![back, row(shown).spacing(GAP), forward]
        .spacing(GAP)
        .align_y(alignment::Vertical::Center)
        .into()
}

/// A navigation button. It is inert and dimmed when `enabled` is false.
fn nav<'a, M: Clone + 'a>(glyph: &'static str, enabled: bool, message: M) -> Element<'a, M> {
    IconButton::new(glyph)
        .width(BUTTON)
        .height(BUTTON)
        .icon_size(GLYPH)
        .disabled(!enabled)
        .on_press(message)
        .into()
}

#[cfg(test)]
#[path = "../../../tests/app/components/carousel.rs"]
mod tests;
