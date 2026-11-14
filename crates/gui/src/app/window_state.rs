//! The main window's size, position and maximized state. They are saved when the launcher closes, and the window is created
//! with them the next time.
//!
//! A maximized window reports its maximized size and place, so the window keeps the geometry it had before it was maximized
//! (`LodestoneLauncher::window_normal`) and that is what is saved. The live size and position are kept only while the window
//! is not maximized.

use iced::window::Position;
use iced::{Point, Size, Task};
use tracing::warn;

use super::preferences::{Preferences, WindowState};
use super::{LodestoneAppMessage, LodestoneLauncher};

/// The size the window opens at when no state is saved.
pub const DEFAULT_SIZE: Size = Size::new(1280.0, 720.0);

/// The smallest size the window may be. The window is created at no less than this.
pub const MIN_SIZE: Size = Size::new(800.0, 600.0);

/// The largest dimension a saved state may restore. A value past it is treated as corrupt.
const MAX_DIMENSION: u32 = 16_384;

/// Where the window is and how large it is while it is not maximized.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Geometry {
    pub size: Size,
    /// `None` when the platform does not report the window's place (Wayland).
    pub position: Option<Point>,
}

impl Default for Geometry {
    fn default() -> Self {
        Self {
            size: DEFAULT_SIZE,
            position: None,
        }
    }
}

/// How the window opens: its size, whether it is maximized, and its place on screen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Opening {
    pub size: Size,
    pub maximized: bool,
    pub position: Option<Point>,
}

impl Default for Opening {
    fn default() -> Self {
        let geometry = Geometry::default();
        Self {
            size: geometry.size,
            maximized: false,
            position: geometry.position,
        }
    }
}

impl Opening {
    /// Reads the opening from a saved state. With no state the window opens at the default size, centred. A saved size below
    /// the minimum or above the largest dimension is clamped into range. A position is used only when both coordinates are
    /// saved.
    pub fn from_saved(saved: Option<WindowState>) -> Self {
        let Some(saved) = saved else {
            return Self::default();
        };
        let size = Size::new(
            saved.width.clamp(MIN_SIZE.width as u32, MAX_DIMENSION) as f32,
            saved.height.clamp(MIN_SIZE.height as u32, MAX_DIMENSION) as f32,
        );
        let position = match (saved.x, saved.y) {
            (Some(x), Some(y)) => Some(Point::new(x as f32, y as f32)),
            _ => None,
        };
        Self {
            size,
            maximized: saved.maximized,
            position,
        }
    }

    /// The window position to open at. Without a saved place the window is centred.
    pub fn position(&self) -> Position {
        self.position.map_or(Position::Centered, Position::Specific)
    }

    /// The geometry the window keeps while it is not maximized, which a maximized window falls back to when it is saved.
    pub fn geometry(&self) -> Geometry {
        Geometry {
            size: self.size,
            position: self.position,
        }
    }
}

/// The saved form of a window that is `maximized` and whose restore geometry is `geometry`. Sizes and places round to whole
/// pixels, and a value out of range saturates.
pub fn to_saved(maximized: bool, geometry: Geometry) -> WindowState {
    WindowState {
        maximized,
        width: geometry.size.width.round() as u32,
        height: geometry.size.height.round() as u32,
        x: geometry.position.map(|point| point.x.round() as i32),
        y: geometry.position.map(|point| point.y.round() as i32),
    }
}

impl LodestoneLauncher {
    /// The window's state as it is to be saved now.
    fn window_to_save(&self) -> WindowState {
        let geometry = if self.maximized {
            self.window_normal
        } else {
            Geometry {
                size: self.window_size,
                position: self.window_position,
            }
        };
        to_saved(self.maximized, geometry)
    }

    /// Saves the window's state, then exits. The state is written before the process ends, so the next launch opens the window
    /// where this one closed.
    pub(super) fn exit_launcher(&self) -> Task<LodestoneAppMessage> {
        let state = self.window_to_save();
        let save = Task::future(async move {
            match tokio::task::spawn_blocking(move || Preferences::update(|preferences| preferences.window = Some(state))).await {
                Ok(Ok(())) => {}
                Ok(Err(error)) => warn!("Saving the window state failed: {error}"),
                Err(error) => warn!("Saving the window state failed: {error}"),
            }
        })
        .discard();
        save.chain(iced::exit())
    }
}

#[cfg(test)]
#[path = "../../tests/app/window_state.rs"]
mod tests;
