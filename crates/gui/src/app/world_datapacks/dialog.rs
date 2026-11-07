//! The datapack dialog: the datapacks a world holds, each one removable, and the shared datapacks, each one addable.

use aurora_leaf::Icon;
use iced::widget::{Text, column, container, row};
use iced::{Element, Fill, alignment};

use super::{Dialog, Message};
use crate::app::components::button::Variant;
use crate::app::components::hint::hint;
use crate::app::components::icon_button::ghost_button_style;
use crate::app::components::modal::Modal;
use crate::app::components::{IconButton, TextButton};
use crate::app::lang;
use crate::app::pages::instance::files::{PackFile, pack_name};
use crate::app::pages::instance::format::format_bytes;
use crate::app::shared_library::SharedEntry;
use crate::app::theme::{color, fonts};

/// The width of the dialog's card.
const WIDTH: f32 = 620.0;

/// The dialog, drawn over the page.
pub fn view(dialog: &Dialog) -> Element<'_, Message> {
    let installed: Element<'_, Message> = if dialog.installed.is_empty() {
        note(lang::t("instance.datapacks.none_installed"))
    } else {
        let mut rows = column![].spacing(6).width(Fill);
        for pack in &dialog.installed {
            rows = rows.push(installed_row(pack));
        }
        rows.into()
    };
    let shared: Element<'_, Message> = if dialog.shared.is_empty() {
        note(lang::t("instance.datapacks.none_shared"))
    } else {
        let mut rows = column![].spacing(6).width(Fill);
        for (index, (entry, present)) in dialog.shared.iter().enumerate() {
            rows = rows.push(shared_row(index, entry, *present));
        }
        rows.into()
    };
    let body = column![
        heading(lang::t("instance.datapacks.installed")),
        installed,
        heading(lang::t("instance.datapacks.shared")),
        shared,
        note(lang::t("instance.datapacks.reload_note")),
    ]
    .spacing(12)
    .width(Fill);
    let close: Element<'_, Message> = TextButton::ghost(lang::t("instance.packs.close")).on_press(Message::Close).into();
    Modal::new(lang::t_with("instance.datapacks.title", &[("name", dialog.name.as_str())]), body)
        .subtitle(lang::t("instance.datapacks.subtitle"))
        .icon(Icon::lucide().layers(), color::primary(), color::primary_tint())
        .footer(close)
        .on_close(Message::Close)
        .close_on_scrim(true)
        .width(WIDTH)
        .into()
}

/// A small heading over one of the dialog's lists.
fn heading<'a>(text: &'static str) -> Element<'a, Message> {
    Text::new(text).size(12).font(fonts::Inter::SEMI_BOLD).color(color::text_muted()).into()
}

/// A line of muted text, for an empty list or a note.
fn note<'a>(text: &'static str) -> Element<'a, Message> {
    Text::new(text).size(12).font(fonts::Inter::REGULAR).color(color::text_muted()).into()
}

/// A datapack the world holds: its name and size, and the button that moves it to the trash.
fn installed_row<'a>(pack: &PackFile) -> Element<'a, Message> {
    let size = pack.size.map_or_else(|| lang::t("instance.packs.folder").to_string(), format_bytes);
    let details = column![
        Text::new(pack.name.clone())
            .size(13)
            .font(fonts::Inter::MEDIUM)
            .color(color::text_default()),
        Text::new(size).size(11).font(fonts::JetBrainsMono::REGULAR).color(color::text_muted()),
    ]
    .spacing(2)
    .width(Fill);
    let remove = hint(
        IconButton::new(Icon::lucide().trash())
            .size(32.0)
            .radius(8.0)
            .icon_size(14.0)
            .style(ghost_button_style())
            .on_press(Message::Remove(pack.file_name.clone())),
        lang::t("instance.packs.remove_hint"),
    );
    row![details, remove].spacing(8).align_y(alignment::Vertical::Center).width(Fill).into()
}

/// A shared datapack: its name, and the button that adds it to the world, or says the world has it.
fn shared_row<'a>(index: usize, entry: &SharedEntry, present: bool) -> Element<'a, Message> {
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
            .on_press(Message::Add(index))
            .into()
    };
    let name = container(
        Text::new(pack_name(&entry.name))
            .size(13)
            .font(fonts::Inter::MEDIUM)
            .color(color::text_default()),
    )
    .width(Fill);
    row![name, action].spacing(12).align_y(alignment::Vertical::Center).width(Fill).into()
}
