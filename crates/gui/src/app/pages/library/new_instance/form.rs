//! The New Instance form: the name, the version and Java pickers, the loader cards, the
//! loader build, the banner, and the footer that shows the install's progress.

use aurora_leaf::Icon;
use iced::widget::{Space, Text, button, column, container, row, stack, svg, text_input};
use iced::{Background, Border, ContentFit, Element, Fill, Padding, Theme, alignment};
use iced_color_extension::rgb;

use super::naming;
use super::{BannerChoice, IconChoice, Message, NewInstance, banner};
use crate::app::components::button::Variant;
use crate::app::components::instance_icon;
use crate::app::components::loader_icon::loader_icon;
use crate::app::components::modal::Modal;
use crate::app::components::scene_art::scene_handle;
use crate::app::components::tile::{INSET, tile};
use crate::app::components::{ProgressBar, Select, TextButton, input};
use crate::app::lang;
use crate::app::pages::library::instance::Loader;
use crate::app::theme::{color, fonts};

/// The card's width.
const WIDTH: f32 = 640.0;
/// The icon tile beside the name.
const ICON_TILE: f32 = 72.0;
/// The banner preview's height. Its width fills the card.
const BANNER_HEIGHT: f32 = 120.0;

/// The loaders in the order the cards show them, with their captions. The names are
/// product names, so only the captions come from the catalogue.
fn loaders() -> [(Loader, &'static str, &'static str); 5] {
    [
        (Loader::Vanilla, "Vanilla", lang::t("new_instance.caption.vanilla")),
        (Loader::Fabric, "Fabric", lang::t("new_instance.caption.fabric")),
        (Loader::Forge, "Forge", lang::t("new_instance.caption.forge")),
        (Loader::NeoForge, "NeoForge", lang::t("new_instance.caption.neoforge")),
        (Loader::Quilt, "Quilt", lang::t("new_instance.caption.quilt")),
    ]
}

/// The dialog card.
pub fn modal(dialog: &NewInstance) -> Element<'_, Message> {
    let mut body = column![banner_section(dialog), name_row(dialog), version_row(dialog), loader_section(dialog)].spacing(20);
    if dialog.loader != Loader::Vanilla {
        body = body.push(build_section(dialog));
    }
    if let Some(error) = &dialog.error {
        body = body.push(error_line(error));
    }

    // The modal scrolls its own body once the form outgrows the window, so the form adds none.
    Modal::new(lang::t("new_instance.title"), body)
        .subtitle(lang::t("new_instance.subtitle"))
        .icon(Icon::lucide().plus(), color::Primary::DEFAULT, color::Primary::TINT)
        .footer(footer(dialog))
        .on_close(Message::Close)
        .close_on_scrim(!dialog.is_installing())
        .width(WIDTH)
        .into()
}

/// The icon tile, which opens the editor, and the name field beside it.
fn name_row(dialog: &NewInstance) -> Element<'_, Message> {
    let icon_tile = tile(
        icon_of(&dialog.icon, ICON_TILE - 2.0 * INSET),
        ICON_TILE,
        ICON_TILE,
        17.0,
        false,
        Message::OpenIconEditor,
    );

    let hint = if dialog.name.trim().is_empty() {
        lang::t("new_instance.folder_hint").to_owned()
    } else {
        format!("instances/{}", naming::clean_directory_name(&dialog.name))
    };
    let name = column![
        label(lang::t("new_instance.name_label")),
        text_input(lang::t("new_instance.name_placeholder"), &dialog.name)
            .on_input(Message::NameChanged)
            .size(14)
            .padding(Padding::new(0.0).left(14).right(14).top(11).bottom(11))
            .style(input::style)
            .width(Fill),
        Text::new(hint).size(11).font(fonts::JetBrainsMono::REGULAR).color(color::Text::MUTED),
    ]
    .spacing(6)
    .width(Fill);

    row![icon_tile, name].spacing(16).align_y(alignment::Vertical::Center).into()
}

/// The Minecraft version and the Java runtime, side by side.
fn version_row(dialog: &NewInstance) -> Element<'_, Message> {
    let versions: Vec<String> = dialog.releases.as_ref().map(|releases| releases.ids.clone()).unwrap_or_default();
    let selected = dialog.mc_version.as_ref().and_then(|mc| versions.iter().position(|id| id == mc));
    let latest = dialog
        .releases
        .as_ref()
        .and_then(|releases| releases.latest.as_ref())
        .is_some_and(|latest| dialog.mc_version.as_ref() == Some(latest));
    let version_placeholder = if dialog.releases_error.is_some() {
        lang::t("new_instance.versions_failed")
    } else {
        lang::t("new_instance.versions_loading")
    };
    let version = Select::new(versions, selected)
        .on_select(Message::MinecraftPicked)
        .placeholder(version_placeholder)
        .searchable()
        .width(Fill);
    let version_caption = latest.then(|| lang::t("new_instance.latest_release").to_owned());

    let mut java_options = vec![auto_java_label(dialog)];
    java_options.extend(dialog.bundled.iter().map(|java| java.label.clone()));
    let java_selected = match &dialog.java_override {
        None => 0,
        Some(component) => dialog.bundled.iter().position(|java| &java.component == component).map_or(0, |at| at + 1),
    };
    let java = Select::new(java_options, Some(java_selected))
        .on_select(Message::JavaPicked)
        .searchable()
        .width(Fill);
    let java_caption = dialog
        .java_override
        .is_none()
        .then(|| lang::t("new_instance.java_auto_matched").to_owned());

    row![
        field(lang::t("new_instance.minecraft_version"), version.into(), version_caption),
        field(lang::t("new_instance.java"), java.into(), java_caption),
    ]
    .spacing(16)
    .into()
}

/// The label of the automatic Java choice, naming the runtime the version asks for.
fn auto_java_label(dialog: &NewInstance) -> String {
    match dialog
        .version_info
        .as_ref()
        .and_then(|info| info.major.map(|major| (major, &info.component)))
    {
        Some((major, _)) => {
            let major = major.to_string();
            lang::t_with("new_instance.java_auto", &[("major", major.as_str())])
        }
        None => lang::t("new_instance.java_auto_unknown").to_owned(),
    }
}

/// The five loader cards in one row.
fn loader_section(dialog: &NewInstance) -> Element<'_, Message> {
    let cards: Vec<Element<'_, Message>> = loaders()
        .into_iter()
        .map(|(loader, name, caption)| container(loader_card(loader, name, caption, dialog.loader == loader)).width(Fill).into())
        .collect();
    column![label(lang::t("new_instance.mod_loader")), row(cards).spacing(10)]
        .spacing(8)
        .into()
}

/// One loader card: its mark, its name and its caption. The chosen card is outlined.
fn loader_card(loader: Loader, name: &'static str, caption: &'static str, chosen: bool) -> Element<'static, Message> {
    let content = column![
        loader_icon(loader, 28.0),
        Text::new(name).size(13).font(fonts::Inter::SEMI_BOLD).color(color::Text::HEADLINE),
        Text::new(caption).size(11).font(fonts::Inter::REGULAR).color(color::Text::MUTED),
    ]
    .spacing(8)
    .padding(Padding::new(12.0))
    .width(Fill);
    button(content)
        .width(Fill)
        .padding(0)
        .style(move |_: &Theme, _| button::Style {
            background: Some(Background::Color(if chosen { color::Primary::TINT } else { rgb!(255, 255, 255, 4%) })),
            text_color: color::Text::DEFAULT,
            border: Border {
                radius: 12.0.into(),
                width: 1.0,
                color: if chosen { color::Primary::DEFAULT } else { color::Surface::LINE },
            },
            ..button::Style::default()
        })
        .on_press(Message::LoaderPicked(loader))
        .into()
}

/// The loader's version picker, shown for a modded loader.
fn build_section(dialog: &NewInstance) -> Element<'_, Message> {
    let options: Vec<String> = dialog
        .builds
        .as_ref()
        .map(|builds| {
            builds
                .iter()
                .map(|build| {
                    if build.stable {
                        build.version.clone()
                    } else {
                        lang::t_with("new_instance.beta", &[("version", build.version.as_str())])
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    let selected = match (&dialog.loader_version, &dialog.builds) {
        (Some(version), Some(builds)) => builds.iter().position(|build| &build.version == version),
        _ => None,
    };
    let placeholder = if dialog.builds.is_none() {
        lang::t("new_instance.builds_loading")
    } else {
        lang::t("new_instance.no_builds")
    };
    let select = Select::new(options, selected)
        .on_select(Message::BuildPicked)
        .placeholder(placeholder)
        .searchable()
        .width(Fill);
    field(lang::t("new_instance.loader_version"), select.into(), None)
}

/// The banner, as a small strip across the top of the form. Clicking it opens the banner
/// editor.
fn banner_section(dialog: &NewInstance) -> Element<'_, Message> {
    let art: Element<'_, Message> = match &dialog.banner {
        BannerChoice::Scene(index) => {
            let (biome, seed) = banner::scene(*index);
            svg(scene_handle(biome, seed))
                .content_fit(ContentFit::Cover)
                .width(Fill)
                .height(Fill)
                .into()
        }
        BannerChoice::Uploaded(upload) => banner::uploaded(upload.handle.clone()),
    };
    let pill = container(
        Text::new(lang::t("new_instance.change_banner"))
            .size(11)
            .font(fonts::Inter::SEMI_BOLD)
            .color(color::Text::HEADLINE),
    )
    .padding(banner::pill_padding())
    .style(|_| banner::pill_style());
    let hint = container(pill)
        .width(Fill)
        .height(Fill)
        .align_x(alignment::Horizontal::Right)
        .align_y(alignment::Vertical::Bottom)
        .padding(Padding::new(12.0));
    let strip = container(stack![art, hint])
        .width(Fill)
        .height(BANNER_HEIGHT)
        .clip(true)
        .style(|_| container::Style {
            border: Border {
                radius: 12.0.into(),
                ..Border::default()
            },
            ..container::Style::default()
        });
    column![
        label(lang::t("new_instance.banner")),
        tile(strip, Fill, BANNER_HEIGHT, 12.0, false, Message::OpenBannerEditor)
    ]
    .spacing(8)
    .into()
}

/// The icon as the form draws it.
fn icon_of(choice: &IconChoice, size: f32) -> Element<'_, Message> {
    match choice {
        IconChoice::Composed { background, symbol } => instance_icon::composed(background, symbol, size),
        IconChoice::Uploaded(upload) => instance_icon::bytes(upload.handle.clone(), size),
    }
}

/// The footer: the install's progress while one runs, otherwise Cancel and Create.
fn footer(dialog: &NewInstance) -> Element<'_, Message> {
    let Some(installing) = &dialog.installing else {
        let mut create = TextButton::content(
            row![
                icon(Icon::lucide().plus(), 14.0, color::Primary::ON),
                Text::new(lang::t("new_instance.create"))
                    .size(13)
                    .font(fonts::Inter::BOLD)
                    .color(color::Primary::ON),
            ]
            .spacing(8)
            .align_y(alignment::Vertical::Center),
        )
        .variant(Variant::Primary)
        .padding(Padding::new(0.0).left(18).right(18).top(12).bottom(12));
        if dialog.ready() {
            create = create.on_press(Message::Submit);
        }
        return row![
            Space::new().width(Fill),
            TextButton::content(
                Text::new(lang::t("common.cancel"))
                    .size(13)
                    .font(fonts::Inter::SEMI_BOLD)
                    .color(color::Text::DEFAULT),
            )
            .variant(Variant::Ghost)
            .padding(Padding::new(0.0).left(18).right(18).top(12).bottom(12))
            .on_press(Message::Close),
            create,
        ]
        .spacing(10)
        .align_y(alignment::Vertical::Center)
        .into();
    };

    let (stage, fraction, detail) = match &installing.progress {
        Some(progress) => (progress.stage.label(), progress.fraction, progress.detail.clone()),
        None => (lang::t("new_instance.starting"), None, String::new()),
    };
    let progress = column![
        Text::new(stage).size(12).font(fonts::Inter::SEMI_BOLD).color(color::Text::DEFAULT),
        ProgressBar::new(fraction.unwrap_or(0.0)).indeterminate(fraction.is_none()).width(Fill),
        Text::new(detail).size(11).font(fonts::JetBrainsMono::REGULAR).color(color::Text::MUTED),
    ]
    .spacing(6)
    .width(Fill);
    row![
        progress,
        TextButton::content(
            Text::new(lang::t("new_instance.cancel_install"))
                .size(13)
                .font(fonts::Inter::SEMI_BOLD)
                .color(color::Text::DEFAULT),
        )
        .variant(Variant::Ghost)
        .padding(Padding::new(0.0).left(18).right(18).top(12).bottom(12))
        .on_press(Message::CancelInstall),
    ]
    .spacing(12)
    .align_y(alignment::Vertical::Center)
    .into()
}

/// A labelled control, with an optional caption under it.
fn field<'a>(name: &'static str, control: Element<'a, Message>, caption: Option<String>) -> Element<'a, Message> {
    let mut column = column![label(name), control].spacing(6).width(Fill);
    if let Some(caption) = caption {
        column = column.push(Text::new(caption).size(11).font(fonts::JetBrainsMono::REGULAR).color(color::Text::MUTED));
    }
    column.into()
}

/// A small uppercase section label.
fn label(text: &'static str) -> Element<'static, Message> {
    Text::new(text)
        .size(11)
        .font(fonts::JetBrainsMono::REGULAR)
        .color(color::Text::MUTED)
        .into()
}

/// A line of error text.
fn error_line(error: &str) -> Element<'_, Message> {
    Text::new(error).size(12).font(fonts::Inter::MEDIUM).color(color::Accents::PINK).into()
}

/// A lucide glyph in `ink`.
fn icon<'a, M: 'a>(markup: &'static str, size: f32, ink: iced::Color) -> Element<'a, M> {
    svg(svg::Handle::from_memory(markup.as_bytes()))
        .width(size)
        .height(size)
        .style(move |_, _| svg::Style { color: Some(ink) })
        .into()
}
