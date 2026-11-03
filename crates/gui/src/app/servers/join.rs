//! The Join Server dialog: an address to connect to, the instance that launches, and the saved servers to pick from. Test
//! pings the address first. Connect launches the instance, and the game joins the server as it starts.

use std::time::Duration;

use aurora_leaf::Icon;
use iced::widget::{Space, Text, column, container, image, mouse_area, row, text, text_input};
use iced::{Element, Fill, Task, mouse};
use lodestone_server::address::ServerAddress;
use lodestone_server::status;

use crate::app::components::modal::Modal;
use crate::app::components::panel::panel_with_padding;
use crate::app::components::typography;
use crate::app::components::{Select, TextButton, input};
use crate::app::lang;
use crate::app::launch::prepare::QuickPlay;
use crate::app::pages::servers::{Reply, State as ServersState};
use crate::app::theme::{color, fonts};
use crate::app::{LodestoneAppMessage, LodestoneLauncher};

/// The width of the dialog, wider than the Add server dialog for its saved list.
const WIDTH: f32 = 760.0;
/// How long a test ping may take before the address counts as not answering.
const TEST_TIMEOUT: Duration = Duration::from_secs(5);

/// The dialog's fields: the address, the instance that launches, and what the test found.
#[derive(Debug, Clone)]
pub struct JoinDialog {
    pub address: String,
    /// The instances that can launch, as their id and name, in the library's order.
    pub instances: Vec<(String, String)>,
    /// The place in `instances` of the instance that launches.
    pub instance: Option<usize>,
    pub test: Test,
    /// Why the address cannot be joined, when it cannot be read.
    pub error: Option<String>,
}

/// The state of the address test.
#[derive(Debug, Clone)]
pub enum Test {
    /// No test has run for the address as it is now.
    Idle,
    /// The test ping is under way.
    Testing,
    /// The test ping came back, with what the server said or why it did not answer.
    Answered(Result<Reply, String>),
}

/// What the dialog's controls send, and what the test ping sends back.
#[derive(Debug, Clone)]
pub enum Message {
    Address(String),
    Instance(usize),
    Test,
    Tested(Result<Reply, String>),
    /// A saved server was picked, so its address fills the field.
    SelectSaved(String),
    /// The link that opens the Add server dialog, at the address typed here.
    AddServer,
    Connect,
    Close,
}

impl LodestoneLauncher {
    /// Opens the dialog at `address`, with the instance that was played last as the one that launches.
    pub(crate) fn open_join_dialog(&mut self, address: String) -> Task<LodestoneAppMessage> {
        let launchable: Vec<_> = self.library.instances().iter().filter(|instance| instance.version_id.is_some()).collect();
        let instances = launchable.iter().map(|instance| (instance.id.clone(), instance.name.clone())).collect();
        let instance = launchable
            .iter()
            .enumerate()
            .max_by_key(|(_, instance)| instance.recency())
            .map(|(index, _)| index);
        self.join_dialog = Some(JoinDialog {
            address,
            instances,
            instance,
            test: Test::Idle,
            error: None,
        });
        // The saved servers the dialog lists are read; their pings follow once they are.
        self.load_saved_servers()
    }

    /// Joins the server at `address`: the instance played last launches and joins it. With no instance installed, the
    /// Join Server dialog opens instead, at that address.
    pub(super) fn join_server(&mut self, address: String) -> Task<LodestoneAppMessage> {
        let Ok(server) = ServerAddress::parse(&address) else {
            return Task::none();
        };
        let last_played = self
            .library
            .instances()
            .iter()
            .filter(|instance| instance.version_id.is_some())
            .max_by_key(|instance| instance.recency())
            .map(|instance| instance.id.clone());
        match last_played {
            Some(id) => self.launch_run(&id, Some(QuickPlay::Server(server))),
            None => self.open_join_dialog(address),
        }
    }

    /// Applies a message from the Join Server dialog.
    pub(crate) fn join_dialog_message(&mut self, message: Message) -> Task<LodestoneAppMessage> {
        let Some(dialog) = self.join_dialog.as_mut() else {
            return Task::none();
        };
        match message {
            Message::Address(address) => {
                dialog.address = address;
                dialog.test = Test::Idle;
                dialog.error = None;
                Task::none()
            }
            Message::Instance(index) => {
                dialog.instance = Some(index);
                Task::none()
            }
            Message::Test => match ServerAddress::parse(&dialog.address) {
                Ok(address) => {
                    dialog.test = Test::Testing;
                    test_ping(address)
                }
                Err(error) => {
                    dialog.error = Some(error.to_string());
                    Task::none()
                }
            },
            Message::Tested(result) => {
                dialog.test = Test::Answered(result);
                Task::none()
            }
            Message::SelectSaved(address) => {
                dialog.address = address;
                dialog.test = Test::Idle;
                dialog.error = None;
                Task::none()
            }
            Message::AddServer => {
                let address = dialog.address.clone();
                self.join_dialog = None;
                self.open_servers_dialog(None, address)
            }
            Message::Connect => self.connect_join_dialog(),
            Message::Close => {
                self.join_dialog = None;
                Task::none()
            }
        }
    }

    /// Launches the instance the dialog chose, joining the server it names. A dialog that cannot connect stays open with
    /// the reason.
    fn connect_join_dialog(&mut self) -> Task<LodestoneAppMessage> {
        let Some(dialog) = self.join_dialog.as_mut() else {
            return Task::none();
        };
        let server = match ServerAddress::parse(&dialog.address) {
            Ok(server) => server,
            Err(error) => {
                dialog.error = Some(error.to_string());
                return Task::none();
            }
        };
        let Some((id, _)) = dialog.instance.and_then(|index| dialog.instances.get(index)).cloned() else {
            return Task::none();
        };
        self.join_dialog = None;
        self.launch_run(&id, Some(QuickPlay::Server(server)))
    }

    /// The dialog on screen. `servers` supplies the saved list and the pings it already has.
    pub fn join_dialog_view<'a>(&'a self, dialog: &'a JoinDialog) -> Element<'a, Message> {
        Modal::new(lang::t("servers.join_dialog.title"), body(dialog, &self.servers))
            .subtitle(lang::t("servers.join_dialog.subtitle"))
            .icon(Icon::lucide().server(), color::amber(), color::amber_tint())
            .footer(footer(dialog))
            .on_close(Message::Close)
            .width(WIDTH)
            .into()
    }
}

/// The dialog's body: the address and its test, the instance, and the saved servers.
fn body<'a>(dialog: &'a JoinDialog, servers: &'a ServersState) -> Element<'a, Message> {
    let address = row![
        text_input(lang::t("servers.join_dialog.placeholder"), &dialog.address)
            .on_input(Message::Address)
            .on_submit(Message::Test)
            .style(input::style)
            .font(fonts::JetBrainsMono::REGULAR)
            .size(14)
            .padding(12)
            .width(Fill),
        TextButton::ghost(lang::t("servers.join_dialog.test")).on_press(Message::Test),
    ]
    .spacing(12);
    let mut column_of_fields = column![typography::eyebrow(lang::t("servers.join_dialog.address")), address].spacing(8);
    if let Some(line) = test_line(dialog) {
        column_of_fields = column_of_fields.push(line);
    }
    if let Some(error) = &dialog.error {
        column_of_fields = column_of_fields.push(text(error.clone()).size(12).font(fonts::Inter::REGULAR).color(color::pink()));
    }
    let names: Vec<&str> = dialog.instances.iter().map(|(_, name)| name.as_str()).collect();
    let launch = row![
        typography::eyebrow(lang::t("servers.join_dialog.launch_with")),
        Select::new(names, dialog.instance).on_select(Message::Instance).width(320.0),
    ]
    .spacing(16)
    .align_y(iced::alignment::Vertical::Center);
    let saved = saved_list(dialog, servers);
    column![column_of_fields, launch, saved].spacing(20).width(Fill).into()
}

/// The line under the address that says what the test found, or `None` before a test has run.
fn test_line(dialog: &JoinDialog) -> Option<Element<'_, Message>> {
    let (line, ink) = match &dialog.test {
        Test::Idle => return None,
        Test::Testing => (lang::t("servers.join_dialog.testing").to_owned(), color::text_muted()),
        Test::Answered(Ok(reply)) => {
            let online = reply.status.online.to_string();
            let max = reply.status.max.to_string();
            let ms = reply.status.latency.as_millis().to_string();
            (
                lang::t_with(
                    "servers.join_dialog.answered",
                    &[
                        ("online", online.as_str()),
                        ("max", max.as_str()),
                        ("ms", ms.as_str()),
                        ("version", reply.status.version.as_str()),
                    ],
                ),
                color::primary(),
            )
        }
        Test::Answered(Err(error)) => (lang::t_with("servers.join_dialog.silent", &[("error", error.as_str())]), color::pink()),
    };
    Some(text(line).size(12).font(fonts::JetBrainsMono::REGULAR).color(ink).into())
}

/// The saved servers, each a row that fills the address when it is pressed, and a link that adds a new one.
fn saved_list<'a>(dialog: &'a JoinDialog, servers: &'a ServersState) -> Element<'a, Message> {
    let header = row![
        typography::eyebrow(lang::t("servers.join_dialog.saved")),
        Space::new().width(Fill),
        TextButton::ghost(lang::t("servers.join_dialog.add_server")).on_press(Message::AddServer),
    ]
    .align_y(iced::alignment::Vertical::Center);
    let rows: Vec<Element<'a, Message>> = servers
        .saved()
        .iter()
        .map(|server| {
            let reply = match servers.probe(&server.address) {
                Some(crate::app::pages::servers::Probe::Online(reply)) => Some(reply.as_ref()),
                _ => None,
            };
            let selected = server.address == dialog.address;
            let identity = column![
                Text::new(server.name.clone())
                    .size(14)
                    .font(fonts::Inter::SEMI_BOLD)
                    .color(color::text_headline()),
                Text::new(server.address.clone())
                    .size(12)
                    .font(fonts::JetBrainsMono::REGULAR)
                    .color(color::text_muted()),
            ]
            .spacing(4)
            .width(Fill);
            let players = reply.map_or_else(String::new, |reply| format!("{} / {}", reply.status.online, reply.status.max));
            let ping = reply.map_or_else(String::new, |reply| format!("{} ms", reply.status.latency.as_millis()));
            let icon = reply.and_then(|reply| reply.icon.clone());
            let mut line = row![].spacing(12).align_y(iced::alignment::Vertical::Center);
            if let Some(icon) = icon {
                line = line.push(image(icon).width(36.0).height(36.0));
            }
            line = line.push(identity).push(
                column![
                    Text::new(ping).size(12).font(fonts::JetBrainsMono::SEMI_BOLD).color(color::primary()),
                    Text::new(players).size(12).font(fonts::JetBrainsMono::REGULAR).color(color::text_muted()),
                ]
                .align_x(iced::alignment::Horizontal::Right),
            );
            let card = panel_with_padding(line, 12.0);
            let card = if selected {
                container(card)
                    .style(|_| container::Style {
                        border: iced::Border {
                            color: color::primary(),
                            width: 1.0,
                            radius: 14.0.into(),
                        },
                        ..container::Style::default()
                    })
                    .into()
            } else {
                card
            };
            mouse_area(card)
                .on_press(Message::SelectSaved(server.address.clone()))
                .interaction(mouse::Interaction::Pointer)
                .into()
        })
        .collect();
    let list = if rows.is_empty() {
        Element::from(
            text(lang::t("servers.join_dialog.none"))
                .size(13)
                .font(fonts::Inter::REGULAR)
                .color(color::text_muted()),
        )
    } else {
        column(rows).spacing(8).width(Fill).into()
    };
    column![header, list].spacing(10).width(Fill).into()
}

/// The footer: Cancel, and Connect, which needs a valid address and an instance to launch with.
fn footer(dialog: &JoinDialog) -> Element<'_, Message> {
    let ready = ServerAddress::parse(&dialog.address).is_ok() && dialog.instance.is_some();
    let connect = TextButton::content(
        text(lang::t("servers.join_dialog.connect"))
            .size(13)
            .font(fonts::Inter::BOLD)
            .color(color::primary_on()),
    )
    .variant(crate::app::components::button::Variant::Primary)
    .disabled(!ready)
    .on_press(Message::Connect);
    row![
        Space::new().width(Fill),
        TextButton::ghost(lang::t("common.cancel")).on_press(Message::Close),
        connect,
    ]
    .spacing(12)
    .align_y(iced::alignment::Vertical::Center)
    .into()
}

/// Pings `address` for the dialog's test, and answers with what it said or why it did not answer.
fn test_ping(address: ServerAddress) -> Task<LodestoneAppMessage> {
    Task::perform(
        async move {
            status::ping(&address, TEST_TIMEOUT)
                .await
                .map(|status| {
                    let icon = status.favicon.as_deref().and_then(status::favicon_png).map(image::Handle::from_bytes);
                    Reply { status, icon }
                })
                .map_err(|error| error.to_string())
        },
        |result| LodestoneAppMessage::JoinDialog(Message::Tested(result)),
    )
}
