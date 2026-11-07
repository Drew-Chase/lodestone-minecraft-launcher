//! The dialog that adds packs from the shared folders to the instance: each pack in the shared folder, and whether the
//! instance has it already.

use aurora_leaf::Icon;
use iced::widget::{Text, column, row};
use iced::{Element, Fill, alignment};
use lodestone_platforms::ContentType;

use super::Message;
use crate::app::components::TextButton;
use crate::app::components::button::Variant;
use crate::app::components::modal::Modal;
use crate::app::lang;
use crate::app::pages::instance::files::pack_name;
use crate::app::shared_library::SharedEntry;
use crate::app::theme::{color, fonts};

/// The width of the dialog's card.
const WIDTH: f32 = 560.0;

/// The dialog's state: the kind of pack it adds, and each pack in the shared folder with whether the instance has it.
#[derive(Debug, Clone)]
pub struct Dialog {
    pub kind: ContentType,
    pub entries: Vec<(SharedEntry, bool)>,
}

/// The dialog, drawn over the Content tab.
pub fn view(dialog: &Dialog) -> Element<'_, Message> {
    let title = match dialog.kind {
        ContentType::ResourcePack => lang::t("instance.packs.shared.resource_title"),
        _ => lang::t("instance.packs.shared.shader_title"),
    };
    let body: Element<'_, Message> = if dialog.entries.is_empty() {
        Text::new(lang::t("instance.packs.shared.empty"))
            .size(13)
            .font(fonts::Inter::REGULAR)
            .color(color::text_muted())
            .into()
    } else {
        let mut rows = column![].spacing(6).width(Fill);
        for (index, (entry, present)) in dialog.entries.iter().enumerate() {
            rows = rows.push(entry_row(dialog.kind, index, entry, *present));
        }
        rows.into()
    };
    let close: Element<'_, Message> = TextButton::ghost(lang::t("instance.packs.close")).on_press(Message::CloseShared).into();
    Modal::new(title, body)
        .subtitle(lang::t("instance.packs.shared.subtitle"))
        .icon(Icon::lucide().layers(), color::primary(), color::primary_tint())
        .footer(close)
        .on_close(Message::CloseShared)
        .close_on_scrim(true)
        .width(WIDTH)
        .into()
}

/// One pack in the shared folder: its name, where it came from, and the button that adds it, or says it is there.
fn entry_row<'a>(kind: ContentType, index: usize, entry: &'a SharedEntry, present: bool) -> Element<'a, Message> {
    let origin = if entry.download.is_some() {
        lang::t("instance.packs.shared.downloaded")
    } else {
        lang::t("instance.packs.shared.loose")
    };
    let details = column![
        Text::new(pack_name(&entry.name))
            .size(13)
            .font(fonts::Inter::MEDIUM)
            .color(color::text_default()),
        Text::new(origin).size(11).font(fonts::Inter::REGULAR).color(color::text_muted()),
    ]
    .spacing(2)
    .width(Fill);
    let action: Element<'a, Message> = if present {
        TextButton::content(
            Text::new(lang::t("instance.packs.shared.added"))
                .size(12)
                .font(fonts::Inter::MEDIUM)
                .color(color::text_muted()),
        )
        .variant(Variant::Ghost)
        .disabled(true)
        .into()
    } else {
        TextButton::labeled_icon(Icon::lucide().plus(), lang::t("instance.packs.shared.add"), Variant::Primary)
            .on_press(Message::Add { kind, index })
            .into()
    };
    row![details, action].spacing(12).align_y(alignment::Vertical::Center).width(Fill).into()
}
