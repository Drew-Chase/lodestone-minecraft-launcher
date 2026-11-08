//! The dialog that adds worlds from the shared folder to the instance: each download in the shared `worlds/` folder, and
//! whether the instance already has a save of its name.

use aurora_leaf::Icon;
use iced::widget::{Text, column, row};
use iced::{Element, Fill, alignment};

use super::Message;
use crate::app::components::button::Variant;
use crate::app::components::hint::hint;
use crate::app::components::icon_button::ghost_button_style;
use crate::app::components::modal::Modal;
use crate::app::components::{IconButton, TextButton};
use crate::app::lang;
use crate::app::shared_library::{SharedEntry, world_name};
use crate::app::theme::{color, fonts};

/// The width of the dialog's card.
const WIDTH: f32 = 560.0;

/// The dialog's state: each world in the shared folder, with whether the instance has a save of its name already.
#[derive(Debug, Clone)]
pub struct Dialog {
    pub entries: Vec<(SharedEntry, bool)>,
}

/// The dialog, drawn over the Worlds tab.
pub fn view(dialog: &Dialog) -> Element<'_, Message> {
    let body: Element<'_, Message> = if dialog.entries.is_empty() {
        Text::new(lang::t("instance.worlds.shared.empty"))
            .size(13)
            .font(fonts::Inter::REGULAR)
            .color(color::text_muted())
            .into()
    } else {
        let mut rows = column![].spacing(6).width(Fill);
        for (index, (entry, present)) in dialog.entries.iter().enumerate() {
            rows = rows.push(entry_row(index, entry, *present));
        }
        rows.into()
    };
    let close: Element<'_, Message> = TextButton::ghost(lang::t("instance.packs.close")).on_press(Message::CloseShared).into();
    Modal::new(lang::t("instance.worlds.shared.title"), body)
        .subtitle(lang::t("instance.worlds.shared.subtitle"))
        .icon(Icon::lucide().globe(), color::primary(), color::primary_tint())
        .footer(close)
        .on_close(Message::CloseShared)
        .close_on_scrim(true)
        .width(WIDTH)
        .into()
}

/// One world in the shared folder: its name, where it came from, the button that adds it or says it is there, and the
/// button that moves it to the trash.
fn entry_row<'a>(index: usize, entry: &'a SharedEntry, present: bool) -> Element<'a, Message> {
    let origin = if entry.download.is_some() {
        lang::t("instance.packs.shared.downloaded")
    } else {
        lang::t("instance.packs.shared.loose")
    };
    let details = column![
        Text::new(world_name(entry))
            .size(13)
            .font(fonts::Inter::MEDIUM)
            .color(color::text_default()),
        Text::new(origin).size(11).font(fonts::Inter::REGULAR).color(color::text_muted()),
    ]
    .spacing(2)
    .width(Fill);
    let action: Element<'a, Message> = if present {
        TextButton::content(
            Text::new(lang::t("instance.worlds.shared.added"))
                .size(12)
                .font(fonts::Inter::MEDIUM)
                .color(color::text_muted()),
        )
        .variant(Variant::Ghost)
        .disabled(true)
        .into()
    } else {
        TextButton::labeled_icon(Icon::lucide().plus(), lang::t("instance.worlds.shared.add"), Variant::Primary)
            .on_press(Message::AddShared { index })
            .into()
    };
    let delete = hint(
        IconButton::new(Icon::lucide().trash())
            .size(32.0)
            .radius(8.0)
            .icon_size(14.0)
            .style(ghost_button_style())
            .on_press(Message::DeleteShared { index }),
        lang::t("instance.worlds.shared.delete_hint"),
    );
    row![details, action, delete]
        .spacing(12)
        .align_y(alignment::Vertical::Center)
        .width(Fill)
        .into()
}
