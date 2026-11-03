//! The Add server and Edit server dialog: a name, an address, and a favorite mark, saved to the list the Servers page shows.
//! The address is checked before it is saved, and an address already saved is refused.

use aurora_leaf::Icon;
use iced::widget::{Space, column, row, text, text_input};
use iced::{Element, Fill, Task};
use lodestone_server::address::ServerAddress;

use crate::app::components::modal::Modal;
use crate::app::components::typography;
use crate::app::components::{TextButton, Toggle, input};
use crate::app::database::servers::SavedServer;
use crate::app::lang;
use crate::app::theme::{color, fonts};
use crate::app::{LodestoneAppMessage, LodestoneLauncher};

/// The width of the dialog, which the Settings and New Instance dialogs share.
const WIDTH: f32 = 520.0;

/// The dialog's fields: the saved server it edits, if any, and what is typed into it.
#[derive(Debug, Clone)]
pub struct Dialog {
    /// The saved server being edited. `None` adds a new one.
    pub editing: Option<i64>,
    pub name: String,
    pub address: String,
    pub favorite: bool,
    /// Why the last save was refused, until the address or the list changes.
    pub error: Option<String>,
}

/// What the dialog's controls send.
#[derive(Debug, Clone)]
pub enum Message {
    Name(String),
    Address(String),
    Favorite(bool),
    Save,
    Close,
}

impl LodestoneLauncher {
    /// Opens the dialog to edit `server`, or to add one at `address` (empty for a blank dialog).
    pub(super) fn open_servers_dialog(&mut self, server: Option<SavedServer>, address: String) -> Task<LodestoneAppMessage> {
        self.servers_dialog = Some(match server {
            Some(server) => Dialog {
                editing: Some(server.id),
                name: server.name,
                address: server.address,
                favorite: server.favorite,
                error: None,
            },
            None => Dialog {
                editing: None,
                name: String::new(),
                address,
                favorite: false,
                error: None,
            },
        });
        Task::none()
    }

    /// Applies a message from the dialog.
    pub(crate) fn servers_dialog_message(&mut self, message: Message) -> Task<LodestoneAppMessage> {
        let Some(dialog) = self.servers_dialog.as_mut() else {
            return Task::none();
        };
        match message {
            Message::Name(name) => dialog.name = name,
            Message::Address(address) => {
                dialog.address = address;
                dialog.error = None;
            }
            Message::Favorite(favorite) => dialog.favorite = favorite,
            Message::Close => self.servers_dialog = None,
            Message::Save => return self.save_servers_dialog(),
        }
        Task::none()
    }

    /// Checks the dialog's address, and saves the server: a new one, or the edited one. A refused address keeps the dialog
    /// open with the reason.
    fn save_servers_dialog(&mut self) -> Task<LodestoneAppMessage> {
        let Some(dialog) = self.servers_dialog.as_mut() else {
            return Task::none();
        };
        let address = match ServerAddress::parse(&dialog.address) {
            Ok(address) => address.to_string(),
            Err(error) => {
                dialog.error = Some(error.to_string());
                return Task::none();
            }
        };
        let editing = dialog.editing;
        let name = dialog.name.trim().to_owned();
        let favorite = dialog.favorite;
        let taken = self
            .servers
            .saved()
            .iter()
            .any(|server| server.address == address && Some(server.id) != editing);
        if taken {
            if let Some(dialog) = self.servers_dialog.as_mut() {
                dialog.error = Some(lang::t("servers.dialog.taken").to_owned());
            }
            return Task::none();
        }
        self.servers_dialog = None;
        match editing {
            Some(id) => {
                let name = if name.is_empty() { address.clone() } else { name };
                self.write_server(id, name, address, favorite)
            }
            None => self.save_new_server(name, address, favorite),
        }
    }
}

/// The dialog on screen: its title, its fields, and the buttons that close it or save it.
pub fn view(dialog: &Dialog) -> Element<'_, Message> {
    let title = if dialog.editing.is_some() {
        lang::t("servers.dialog.edit.title")
    } else {
        lang::t("servers.dialog.add.title")
    };
    let mut fields = column![
        typography::eyebrow(lang::t("servers.dialog.name")),
        text_input(lang::t("servers.dialog.name_placeholder"), &dialog.name)
            .on_input(Message::Name)
            .style(input::style)
            .size(14)
            .padding(12)
            .width(Fill),
        Space::new().height(8.0),
        typography::eyebrow(lang::t("servers.dialog.address")),
        text_input(lang::t("servers.dialog.address_placeholder"), &dialog.address)
            .on_input(Message::Address)
            .style(input::style)
            .font(fonts::JetBrainsMono::REGULAR)
            .size(14)
            .padding(12)
            .width(Fill),
    ]
    .spacing(8);
    if let Some(error) = &dialog.error {
        fields = fields.push(text(error.clone()).size(12).font(fonts::Inter::REGULAR).color(color::pink()));
    }
    let favorite = row![
        text(lang::t("servers.dialog.favorite"))
            .size(14)
            .font(fonts::Inter::MEDIUM)
            .color(color::text_headline()),
        Space::new().width(Fill),
        Toggle::new(dialog.favorite).on_toggle(Message::Favorite(!dialog.favorite)),
    ]
    .align_y(iced::alignment::Vertical::Center);
    let body = column![fields, favorite].spacing(18).width(Fill);
    let footer = row![
        Space::new().width(Fill),
        TextButton::ghost(lang::t("common.cancel")).on_press(Message::Close),
        TextButton::content(
            text(lang::t("servers.dialog.save"))
                .size(13)
                .font(fonts::Inter::BOLD)
                .color(color::primary_on()),
        )
        .variant(crate::app::components::button::Variant::Primary)
        .on_press(Message::Save),
    ]
    .spacing(12)
    .align_y(iced::alignment::Vertical::Center);
    Modal::new(title, body)
        .subtitle(lang::t("servers.dialog.subtitle"))
        .icon(Icon::lucide().server(), color::primary(), color::primary_tint())
        .footer(footer)
        .on_close(Message::Close)
        .width(WIDTH)
        .into()
}
