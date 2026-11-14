//! The Folders step: the folders the launcher keeps its files in, listed the way the Settings page lists them.

use iced::Element;
use iced::widget::column;

use super::Message;
use crate::app::components::folder_rows;
use crate::app::data_folder;
use crate::app::paths::LauncherPaths;

/// The folder rows, each with its buttons. A change to instances or to the shared data moves them at once, and a change to the
/// logs or the cache takes effect the next time the launcher starts.
pub fn view<'a>(paths: &LauncherPaths) -> Element<'a, Message> {
    column(folder_rows::rows(&data_folder::rows(paths), Message::Folder))
        .spacing(14)
        .width(iced::Fill)
        .into()
}
