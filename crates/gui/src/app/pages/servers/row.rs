//! A row of the Servers page: a saved server, or a game heard on the network. A saved server shows its art, its name and
//! address, its message of the day, its ping and players, and its menu and Join button. A game heard on the network shows
//! its title and address, with buttons to save it and join it.

use std::hash::{DefaultHasher, Hash, Hasher};
use std::time::Duration;

use aurora_leaf::Icon;
use iced::widget::{Text, column, container, image, row, svg};
use iced::{ContentFit, Element, Fill, Length, alignment};
use lodestone_server::lan::Announcement;

use super::{ART_SIZE, Message, Probe, Reply, RowAction};
use crate::app::components::badge::Badge;
use crate::app::components::icon_button::{IconButton, ghost_button_style};
use crate::app::components::panel::panel_with_padding;
use crate::app::components::scene_art::card_art_handle;
use crate::app::components::{Dropdown, MenuItem, ProgressBar, TextButton};
use crate::app::database::servers::SavedServer;
use crate::app::lang;
use crate::app::pages::library::instance::Biome;
use crate::app::theme::{color, fonts};

/// The scenes a row's art shows when its server has no icon. The address's hash picks one of them.
const BIOMES: [Biome; 8] = [
    Biome::End,
    Biome::Forest,
    Biome::Cherry,
    Biome::Ocean,
    Biome::Desert,
    Biome::Snow,
    Biome::Nether,
    Biome::Mushroom,
];

/// A saved server's row: its art, its name and address, its message of the day, its ping and players, its menu, and its
/// Join button.
pub fn saved<'a>(server: &'a SavedServer, probe: Option<&'a Probe>) -> Element<'a, Message> {
    let reply = match probe {
        Some(Probe::Online(reply)) => Some(reply.as_ref()),
        _ => None,
    };
    let mut name = row![
        Text::new(server.name.clone())
            .size(15)
            .font(fonts::Inter::SEMI_BOLD)
            .color(color::text_headline()),
        IconButton::new(Icon::lucide().star())
            .size(24)
            .radius(8.0)
            .icon_size(12.0)
            .style(ghost_button_style())
            .on_press(Message::ToggleFavorite(server.id)),
    ]
    .spacing(8)
    .align_y(alignment::Vertical::Center);
    if server.favorite {
        name = name.push(Badge::new(lang::t("servers.favorite"), color::amber(), color::amber_tint()).compact());
    }
    let mut address = row![
        Text::new(server.address.clone())
            .size(12)
            .font(fonts::JetBrainsMono::REGULAR)
            .color(color::text_muted()),
    ]
    .spacing(8)
    .align_y(alignment::Vertical::Center);
    if let Some(reply) = reply {
        address = address.push(Badge::new(reply.status.version.clone(), color::text_muted(), color::surface_bg3()).compact());
    }
    let mut identity = column![name, address].spacing(6).width(Fill);
    if let Some(reply) = reply.filter(|reply| !reply.status.motd.is_empty()) {
        identity = identity.push(
            Text::new(reply.status.motd.clone())
                .size(12)
                .font(fonts::Inter::REGULAR)
                .color(color::text_muted()),
        );
    }
    // A server that did not answer says why, in place of its message of the day.
    if let Some(Probe::Offline(reason)) = probe {
        identity = identity.push(Text::new(reason.clone()).size(12).font(fonts::Inter::REGULAR).color(color::pink()));
    }
    let join = TextButton::content(
        row![
            Text::new(lang::t("servers.join"))
                .size(13)
                .font(fonts::Inter::BOLD)
                .color(color::primary_on()),
        ]
        .align_y(alignment::Vertical::Center),
    )
    .variant(crate::app::components::button::Variant::Primary)
    .on_press(Message::Join(server.address.clone()));
    card(
        row![art(reply, &server.address), identity, stats(probe, reply), menu(server), join,]
            .spacing(16)
            .align_y(alignment::Vertical::Center),
    )
}

/// A game heard on the local network: its art, its title and address, and the buttons that save it and join it.
pub fn heard(game: &Announcement) -> Element<'_, Message> {
    let address = game.address.to_string();
    let identity = column![
        Text::new(game.title.clone())
            .size(15)
            .font(fonts::Inter::SEMI_BOLD)
            .color(color::text_headline()),
        Text::new(address.clone())
            .size(12)
            .font(fonts::JetBrainsMono::REGULAR)
            .color(color::text_muted()),
        Text::new(lang::t("servers.lan.announced"))
            .size(12)
            .font(fonts::Inter::REGULAR)
            .color(color::text_muted()),
    ]
    .spacing(6)
    .width(Fill);
    let save = TextButton::ghost(lang::t("servers.lan.save")).on_press(Message::SaveHeard {
        name: game.title.clone(),
        address: address.clone(),
    });
    let join = TextButton::content(
        Text::new(lang::t("servers.join"))
            .size(13)
            .font(fonts::Inter::BOLD)
            .color(color::primary_on()),
    )
    .variant(crate::app::components::button::Variant::Primary)
    .on_press(Message::Join(address.clone()));
    card(
        row![art(None, &address), identity, save, join]
            .spacing(16)
            .align_y(alignment::Vertical::Center),
    )
}

/// The card a row sits in: the panel, with its padding.
fn card<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    panel_with_padding(content, 14.0)
}

/// The row's art at the size the page gives it: the server's icon, or a scene chosen by the address when it has none.
fn art<'a>(reply: Option<&'a Reply>, address: &str) -> Element<'a, Message> {
    let width = Length::Fixed(ART_SIZE.width);
    let height = Length::Fixed(ART_SIZE.height);
    let picture: Element<'a, Message> = match reply.and_then(|reply| reply.icon.clone()) {
        Some(icon) => image(icon).width(width).height(height).content_fit(ContentFit::Cover).into(),
        None => {
            let (biome, seed) = scene(address);
            svg(card_art_handle(biome, seed, ART_SIZE)).width(width).height(height).into()
        }
    };
    container(picture).width(width).height(height).into()
}

/// The scene a server's art shows, from a hash of its address, so the same server keeps its scene.
fn scene(address: &str) -> (Biome, u64) {
    let mut hasher = DefaultHasher::new();
    address.hash(&mut hasher);
    let hash = hasher.finish();
    (BIOMES[(hash % BIOMES.len() as u64) as usize], hash >> 8)
}

/// The ping and the players: the round trip in milliseconds, the players on the server out of its most, and a bar for
/// how full the server is. A server that has not answered shows its state in place of the numbers.
fn stats<'a>(probe: Option<&'a Probe>, reply: Option<&'a Reply>) -> Element<'a, Message> {
    let (ping, ping_ink) = match probe {
        Some(Probe::Online(reply)) => (
            lang::t_with("servers.ping", &[("ms", reply.status.latency.as_millis().to_string().as_str())]),
            latency_ink(reply.status.latency),
        ),
        Some(Probe::Offline(_)) => (lang::t("servers.offline").to_owned(), color::pink()),
        Some(Probe::Waiting) | None => ("…".to_owned(), color::text_muted()),
    };
    let (players, fraction) = match reply {
        Some(reply) => {
            let max = reply.status.max.max(1);
            let fraction = (reply.status.online as f32 / max as f32).clamp(0.0, 1.0);
            (format!("{} / {}", reply.status.online, reply.status.max), fraction)
        }
        None => ("—".to_owned(), 0.0),
    };
    column![
        Text::new(ping).size(13).font(fonts::JetBrainsMono::SEMI_BOLD).color(ping_ink),
        Text::new(players)
            .size(12)
            .font(fonts::JetBrainsMono::REGULAR)
            .color(color::text_headline()),
        ProgressBar::new(fraction).width(Length::Fixed(120.0)),
    ]
    .spacing(6)
    .align_x(alignment::Horizontal::Right)
    .into()
}

/// The ink of a ping: green for a quick answer, amber for a slower one, and pink for a slow one.
fn latency_ink(latency: Duration) -> iced::Color {
    match latency.as_millis() {
        0..80 => color::primary(),
        80..160 => color::amber(),
        _ => color::pink(),
    }
}

/// The row's menu: the actions, in the order [`RowAction::MENU`] lists them, with the favorite's label as its mark stands.
fn menu(server: &SavedServer) -> Element<'_, Message> {
    let labels = RowAction::MENU.map(|action| match action {
        RowAction::Join => (lang::t("servers.action.join"), false),
        RowAction::Copy => (lang::t("servers.action.copy"), false),
        RowAction::Favorite if server.favorite => (lang::t("servers.action.unfavorite"), false),
        RowAction::Favorite => (lang::t("servers.action.favorite"), false),
        RowAction::Edit => (lang::t("servers.action.edit"), false),
        RowAction::Remove => (lang::t("servers.action.remove"), true),
    });
    let items = labels.map(|(label, danger)| MenuItem::Action {
        label: label.to_owned(),
        icon: None,
        danger,
    });
    let address = server.address.clone();
    Dropdown::new(Icon::lucide().ellipsis_vertical())
        .items(items)
        .on_select(move |index| Message::RowMenu {
            address: address.clone(),
            index,
        })
        .into()
}
