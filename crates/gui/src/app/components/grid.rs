//! Grids — equal-width columns of cells, the layout behind the tab grids.
//!
//! ```text
//! grid(cards, 2, 14.0)
//! ```

use iced::widget::{Space, column, container, row};
use iced::{Element, Fill, Length};

/// Lays `cells` out in rows of `columns`, each cell taking an equal share of
/// the width. A short last row is padded with empty space so its cells keep
/// the widths of the rows above.
pub fn grid<'a, M: 'a>(cells: Vec<Element<'a, M>>, columns: usize, gap: f32) -> Element<'a, M> {
    let columns = columns.max(1);
    let mut rows: Vec<Element<'a, M>> = Vec::with_capacity(cells.len().div_ceil(columns));
    let mut cells = cells.into_iter().peekable();

    while cells.peek().is_some() {
        let mut row_cells: Vec<Element<'a, M>> = cells.by_ref().take(columns).map(|cell| share(cell)).collect();
        row_cells.resize_with(columns, || share(Space::new()));
        rows.push(row(row_cells).spacing(gap).into());
    }

    column(rows).spacing(gap).width(Fill).into()
}

/// Wraps a cell so it takes one equal share of its row.
fn share<'a, M: 'a>(cell: impl Into<Element<'a, M>>) -> Element<'a, M> {
    container(cell).width(Length::FillPortion(1)).into()
}
