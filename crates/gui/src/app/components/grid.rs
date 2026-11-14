//! Grids — equal-width columns of cells, the layout behind the tab grids, and the single lines the Discover results window.
//!
//! ```text
//! grid(cards, 2, 14.0)
//! line(cells, 2, 14.0)
//! ```

use iced::widget::{Space, column, container, row};
use iced::{Element, Fill, Length};

/// Lays `cells` out in rows of `columns`, each cell taking an equal share of
/// the width. A short last row is padded with empty space so its cells keep
/// the widths of the rows above.
pub fn grid<'a, M: 'a>(cells: Vec<Element<'a, M>>, columns: usize, gap: f32) -> Element<'a, M> {
    let columns = columns.max(1);
    let mut cells = cells.into_iter().peekable();
    let mut rows: Vec<Element<'a, M>> = Vec::new();
    while cells.peek().is_some() {
        let chunk: Vec<Element<'a, M>> = cells.by_ref().take(columns).collect();
        rows.push(line(chunk, columns, gap));
    }
    column(rows).spacing(gap).width(Fill).into()
}

/// One row of `columns` cells, each taking an equal share of the width. A short row is padded with empty space, so its cells
/// keep the widths of the rows beside it. Cells past `columns` are not shown.
pub fn line<'a, M: 'a>(cells: Vec<Element<'a, M>>, columns: usize, gap: f32) -> Element<'a, M> {
    let columns = columns.max(1);
    let mut shares: Vec<Element<'a, M>> = cells.into_iter().take(columns).map(|cell| share(cell)).collect();
    shares.resize_with(columns, || share(Space::new()));
    row(shares).spacing(gap).into()
}

/// Wraps a cell so it takes one equal share of its row.
fn share<'a, M: 'a>(cell: impl Into<Element<'a, M>>) -> Element<'a, M> {
    container(cell).width(Length::FillPortion(1)).into()
}
