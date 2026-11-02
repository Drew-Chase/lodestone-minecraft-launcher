//! The General section: the instance's name, its banner and its icon. A change to any of them leaves as an [`Edit`].

use iced::widget::{column, container, text_input};
use iced::{Element, Padding, alignment};

use super::{Edit, Message, scene_index};
use crate::app::components::launch_rows;
use crate::app::components::panel::divided_panel;
use crate::app::components::setting_row::described_setting;
use crate::app::components::{TextButton, input, instance_art, instance_icon};
use crate::app::lang;
use crate::app::pages::library::instance::Instance;
use crate::app::pages::library::new_instance::banner;
use crate::app::theme::fonts;

/// The banner's preview width. It is drawn in the 16:9 frame the banner uses.
const PREVIEW_WIDTH: f32 = 280.0;
const PREVIEW_HEIGHT: f32 = 158.0;
/// The icon's preview edge.
const ICON_SIZE: f32 = 72.0;

/// The rows of the General section: the name field, the banner preview and the icon preview, each with its button.
pub fn body(instance: &Instance) -> Element<'_, Message> {
    let name = text_input(lang::t("instance.settings.name_placeholder"), &instance.name)
        .on_input(|text| Message::Edit(Edit::Name(text)))
        .style(input::style)
        .font(fonts::Inter::REGULAR)
        .size(13)
        .padding(Padding::new(0.0).left(12).right(12).top(8).bottom(8))
        .width(320);

    let picture: Element<'_, Message> = match &instance.art_path {
        Some(path) => instance_art::custom_banner(path),
        None => banner::art(scene_index(instance.biome, instance.art_seed)),
    };
    let banner_control = column![
        container(picture).width(PREVIEW_WIDTH).height(PREVIEW_HEIGHT).clip(true),
        TextButton::ghost(lang::t("instance.settings.change_banner")).on_press(Message::OpenBannerEditor),
    ]
    .spacing(10)
    .align_x(alignment::Horizontal::Right);

    let icon_control = column![
        instance_icon::icon(&instance.icon, ICON_SIZE),
        TextButton::ghost(lang::t("instance.settings.change_icon")).on_press(Message::OpenIconEditor),
    ]
    .spacing(10)
    .align_x(alignment::Horizontal::Right);

    divided_panel(
        vec![
            described_setting(lang::t("instance.settings.name"), lang::t("instance.settings.name_hint"), name),
            described_setting(
                lang::t("instance.settings.banner"),
                lang::t("instance.settings.banner_hint"),
                banner_control,
            ),
            described_setting(lang::t("instance.settings.icon"), lang::t("instance.settings.icon_hint"), icon_control),
        ],
        launch_rows::panel_padding(),
    )
}
