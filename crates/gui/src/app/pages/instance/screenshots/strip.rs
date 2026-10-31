//! The quick-navigation row under the viewer: small thumbnails of the captures around the one shown, with
//! that one in the middle and ringed in green. A press on a thumbnail shows its capture.

use iced::widget::{container, image, row};
use iced::{ContentFit, Element, Fill, Size, alignment};

use super::viewer;
use super::{Message, State};
use crate::app::components::screenshot_tile::placeholder_art;
use crate::app::components::tile::tile;

/// A thumbnail's width and height, about sixteen by nine, and the gap between thumbnails.
const THUMB_WIDTH: f32 = 56.0;
const THUMB_HEIGHT: f32 = 32.0;
const GAP: f32 = 6.0;
/// A thumbnail's corner radius.
const RADIUS: f32 = 6.0;
/// The fewest and most thumbnails on the row. The count is odd, so the shown capture sits in the middle.
const MIN_THUMBS: usize = 3;
const MAX_THUMBS: usize = 11;

/// How many thumbnails fit a row `width` wide: an odd count from 3 to 11.
pub fn visible_thumbnails(width: f32) -> usize {
    let fit = ((width + GAP) / (THUMB_WIDTH + GAP)).floor().max(0.0) as usize;
    let odd = if fit.is_multiple_of(2) { fit.saturating_sub(1) } else { fit };
    odd.clamp(MIN_THUMBS, MAX_THUMBS)
}

/// The first capture on the row, so that `selected` sits in the middle of `visible` thumbnails, held
/// inside the `len` captures.
pub fn window_start(selected: usize, len: usize, visible: usize) -> usize {
    selected.saturating_sub(visible / 2).min(len.saturating_sub(visible))
}

/// The row for the capture at `index`, laid out for a viewer `size` big.
pub fn view(state: &State, index: usize, size: Size) -> Element<'_, Message> {
    let visible = visible_thumbnails(size.width - 2.0 * viewer::PADDING);
    let len = state.shots().len();
    let start = window_start(index, len, visible);
    let end = (start + visible).min(len);
    let thumbs: Vec<Element<'_, Message>> = (start..end).map(|position| thumb(state, position, position == index)).collect();

    container(row(thumbs).spacing(GAP))
        .width(Fill)
        .align_x(alignment::Horizontal::Center)
        .into()
}

/// One thumbnail: the capture's picture in a tile, which a press shows.
fn thumb(state: &State, position: usize, shown: bool) -> Element<'_, Message> {
    let shot = &state.shots()[position];
    let picture: Element<'_, Message> = match state.thumbnail(&shot.path) {
        Some(handle) => image(handle.clone())
            .content_fit(ContentFit::Cover)
            .width(Fill)
            .height(Fill)
            .border_radius(RADIUS)
            .into(),
        None => placeholder_art(RADIUS),
    };
    tile(picture, THUMB_WIDTH, THUMB_HEIGHT, RADIUS, shown, Message::Show(position))
}

#[cfg(test)]
#[path = "../../../../../tests/app/pages/instance/screenshots/strip.rs"]
mod tests;
