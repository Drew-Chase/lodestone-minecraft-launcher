//! Table columns that hide as a window narrows. A table lists its columns with a fixed width each and the order they
//! hide in. [`fit`] keeps the columns in that order until the name column, which takes the rest of the row, has its
//! least width. The columns that never hide are the name and the actions, so a row always keeps what it is for.

/// The least width the name column keeps before the columns beside it hide.
pub const NAME_MIN: f32 = 140.0;

/// One fixed-width column of a table.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Column {
    /// The column's width in pixels.
    pub width: f32,
    /// Its place in the hide order: the lowest hides first. `None` keeps the column at every width.
    pub hide: Option<u8>,
}

/// Which of `columns` show in a row `available` pixels wide, with the name column taking at least `name_min` and a `gap`
/// between each pair of neighbouring cells. The columns hide in their hide order until the rest fits. A column with no
/// hide order never hides, so a row that cannot fit even then keeps them all.
pub fn fit(columns: &[Column], available: f32, name_min: f32, gap: f32) -> Vec<bool> {
    let mut shown = vec![true; columns.len()];
    loop {
        let count = shown.iter().filter(|show| **show).count();
        let fixed: f32 = columns
            .iter()
            .zip(&shown)
            .filter(|(_, show)| **show)
            .map(|(column, _)| column.width)
            .sum();
        // The name column sits beside every shown column, so there is one gap per shown column.
        if fixed + name_min + gap * count as f32 <= available {
            return shown;
        }
        let next = columns
            .iter()
            .enumerate()
            .filter(|(index, column)| shown[*index] && column.hide.is_some())
            .min_by_key(|(_, column)| column.hide.unwrap_or(u8::MAX))
            .map(|(index, _)| index);
        match next {
            Some(index) => shown[index] = false,
            None => return shown,
        }
    }
}

#[cfg(test)]
#[path = "../../../tests/app/components/table.rs"]
mod tests;
