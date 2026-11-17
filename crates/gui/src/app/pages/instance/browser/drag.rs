//! Dragging rows of the Files tab onto a folder, the way a file manager moves files.
//!
//! A press on a row arms the drag. It starts once the pointer has moved [`THRESHOLD`] pixels with the button held, so a
//! click that wobbles is still a click. The rows that travel are the selection when the pressed row is part of it, and
//! the pressed row alone otherwise. A release over a folder that can take them moves them there.

use iced::Point;

/// How far the pointer moves, with the button held, before a press becomes a drag.
pub const THRESHOLD: f32 = 6.0;

/// Where a drag stands.
#[derive(Debug, Clone, Default, PartialEq)]
pub enum Drag {
    /// No button is held on a row.
    #[default]
    Idle,
    /// A row is pressed and the pointer has not moved far yet. `origin` is where the pointer was first seen after the
    /// press, since the press itself carries no position.
    Pressed { path: String, origin: Option<Point> },
    /// The rows at `paths` follow the pointer, which is at `at`.
    Dragging { paths: Vec<String>, at: Point },
}

impl Drag {
    /// Arms a drag of the row at `path`.
    pub fn press(path: String) -> Self {
        Self::Pressed { path, origin: None }
    }

    /// Whether a button is held on a row, so the pointer has to be followed.
    pub fn is_armed(&self) -> bool {
        !matches!(self, Self::Idle)
    }

    /// The rows that follow the pointer, while a drag is under way.
    pub fn dragged(&self) -> Option<&[String]> {
        match self {
            Self::Dragging { paths, .. } => Some(paths),
            _ => None,
        }
    }

    /// Where the pointer is, while a drag is under way.
    pub fn pointer(&self) -> Option<Point> {
        match self {
            Self::Dragging { at, .. } => Some(*at),
            _ => None,
        }
    }

    /// The pointer moved to `at`. `selection` is the selected rows, which travel together when the pressed row is one
    /// of them.
    pub fn moved(&mut self, at: Point, selection: &[String]) {
        match self {
            Self::Idle => {}
            Self::Pressed { origin: origin @ None, .. } => *origin = Some(at),
            Self::Pressed { path, origin: Some(origin) } => {
                if origin.distance(at) >= THRESHOLD {
                    let paths = if selection.contains(path) {
                        selection.to_vec()
                    } else {
                        vec![std::mem::take(path)]
                    };
                    *self = Self::Dragging { paths, at };
                }
            }
            Self::Dragging { at: pointer, .. } => *pointer = at,
        }
    }

    /// The button was released. Returns what the press turned out to be, and goes back to idle.
    pub fn release(&mut self) -> Released {
        match std::mem::take(self) {
            Self::Idle => Released::Nothing,
            Self::Pressed { path, .. } => Released::Click(path),
            Self::Dragging { paths, .. } => Released::Drop(paths),
        }
    }
}

/// What a press turned out to be once the button came up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Released {
    /// No row was pressed.
    Nothing,
    /// The row was pressed and released where it was.
    Click(String),
    /// These rows were dragged, and dropped where the pointer is.
    Drop(Vec<String>),
}

/// Whether the folder at `target` can take the rows at `dragged`: it is not one of them, not inside one of them, and
/// not the folder they already sit in. The empty path is the instance folder.
pub fn accepts(target: &str, dragged: &[String]) -> bool {
    dragged.iter().all(|path| {
        let inside_itself = target == path || target.strip_prefix(path.as_str()).is_some_and(|rest| rest.starts_with('/'));
        !inside_itself && parent(path) != target
    })
}

/// The folder that holds the entry at `path`. The empty path is the instance folder.
pub fn parent(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(folder, _)| folder)
}

#[cfg(test)]
#[path = "../../../../../tests/app/pages/instance/browser/drag.rs"]
mod tests;
