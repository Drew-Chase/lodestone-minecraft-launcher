//! The Import step: the instances the other launchers keep on this machine, which the player checks to copy into Lodestone. The
//! launchers' rows are the Import dialog's own. The footer's Next copies the checked instances in, as the dialog's Import does.

use iced::Element;

use super::Message;
use crate::app::import::Message as ImportMessage;
use crate::app::import::launcher_picker::LauncherPicker;

/// The launchers' rows, with a checkbox for each instance that can be imported.
pub fn view<'a>(picker: &'a LauncherPicker, importing: bool) -> Element<'a, Message> {
    picker.view(importing).map(|message| Message::Import(ImportMessage::Picker(message)))
}
