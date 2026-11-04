//! The Skins page's layout. The 3D viewer fills the page's height on the left, with its controls laid over it. On the right the
//! account's capes stand above the list of skins, and the list scrolls while the viewer stays where it is.

use aurora_leaf::Icon;
use iced::widget::image::{FilterMethod, Handle};
use iced::widget::{Row, Space, button, column, container, image, row, stack, text};
use iced::{Background, Border, Color, Element, Fill, Padding, alignment};

use super::catalog::{BUNDLED, Pack};
use super::viewer::{self, Layers, Model, Pose};
use super::{Account, CapePick, Choice, Layer, Message, Section, Skins};
use crate::app::components::TextButton;
use crate::app::components::grid::grid;
use crate::app::components::hint::hint;
use crate::app::components::page::PAGE_SIDE;
use crate::app::components::scrollbar;
use crate::app::components::segmented::segmented;
use crate::app::components::tile::tile;
use crate::app::lang;
use crate::app::pages::Page;
use crate::app::theme::{color, fonts, utility};

/// The share of the page the viewer takes, and the least width it keeps. The rest of the page is the list of skins.
const VIEWER_SHARE: f32 = 0.42;
const VIEWER_MIN: f32 = 300.0;

/// The gap between the viewer and the list, and between tiles.
const COLUMN_GAP: f32 = 20.0;
const GAP: f32 = 12.0;

/// The room the list keeps clear on its right for the scrollbar: its track, and a gap before the track.
const SCROLLBAR_INSET: f32 = scrollbar::TRACK_WIDTH + GAP;

/// A skin's tile, and its 3D preview inside it. The preview is shown at 96 by 128.
const TILE_WIDTH: f32 = 112.0;
const TILE_HEIGHT: f32 = 170.0;
const PREVIEW_WIDTH: f32 = 96.0;
const PREVIEW_HEIGHT: f32 = 128.0;

/// A cape's tile, and its picture: the cape's front, four times over.
const CAPE_TILE_WIDTH: f32 = 72.0;
const CAPE_TILE_HEIGHT: f32 = 100.0;
const CAPE_PICTURE_WIDTH: f32 = 40.0;
const CAPE_PICTURE_HEIGHT: f32 = 64.0;

/// The page: the viewer fills the height on the left, and the account's capes stand above the list of skins on the right.
/// `width` is the width inside the page's margins.
pub(super) fn view<'a>(page: &'a Skins, account: Account<'a>, width: f32) -> Element<'a, Message> {
    let viewer_width = (width * VIEWER_SHARE).max(VIEWER_MIN);
    let list_width = (width - viewer_width - COLUMN_GAP).max(TILE_WIDTH);
    // The scrollbar's track lies over the list's right edge, so the capes and the sections keep clear of it.
    let inner_width = (list_width - SCROLLBAR_INSET).max(TILE_WIDTH);
    let columns = ((inner_width + GAP) / (TILE_WIDTH + GAP)).floor().max(1.0) as usize;
    let list = column![
        container(capes(page, account, inner_width)).padding(Padding::new(0.0).top(24.0).right(SCROLLBAR_INSET)),
        scrollbar::scroll(container(sections(page, columns)).padding(Padding::new(0.0).top(12.0).bottom(24.0).right(SCROLLBAR_INSET)))
            .width(Fill)
            .height(Fill),
    ]
    .spacing(20)
    .width(Fill)
    .height(Fill);
    row![
        container(viewer_panel(page, account)).width(viewer_width).height(Fill),
        container(list).width(Fill).height(Fill),
    ]
    .spacing(COLUMN_GAP)
    .width(Fill)
    .height(Fill)
    .padding(Padding::new(0.0).left(PAGE_SIDE).right(PAGE_SIDE))
    .into()
}

/// The viewer: the model fills the panel, and the title, the layer and pose switches and the chosen skin's card lie over it.
fn viewer_panel<'a>(page: &'a Skins, account: Account<'a>) -> Element<'a, Message> {
    let model = Model {
        skin: page.shown_skin(),
        pose: page.pose(),
        layers: page.layers(),
        slim: page.shown_slim(),
        time: 0.0,
    };
    stack![
        viewer::viewer(model),
        overlay(crate::h2!(lang::t("skins.title")), alignment::Horizontal::Left, alignment::Vertical::Top),
        overlay(layer_column(page.layers()), alignment::Horizontal::Left, alignment::Vertical::Center),
        overlay(pose_column(page.pose()), alignment::Horizontal::Right, alignment::Vertical::Center),
        overlay(info_card(page, account), alignment::Horizontal::Center, alignment::Vertical::Bottom),
    ]
    .width(Fill)
    .height(Fill)
    .into()
}

/// Content laid over the viewer at one side of it, with room around it.
fn overlay<'a>(content: impl Into<Element<'a, Message>>, horizontal: alignment::Horizontal, vertical: alignment::Vertical) -> Element<'a, Message> {
    container(content)
        .width(Fill)
        .height(Fill)
        .padding(16)
        .align_x(horizontal)
        .align_y(vertical)
        .into()
}

/// The chosen skin's card, laid over the foot of the viewer: what it is, its model, and what can be done with it.
fn info_card<'a>(page: &'a Skins, account: Account<'a>) -> Element<'a, Message> {
    let eyebrow = row![
        text(lang::t("skins.selected"))
            .size(10)
            .font(fonts::JetBrainsMono::SEMI_BOLD)
            .color(color::text_muted()),
        Space::new().width(Fill),
        text(lang::t("skins.drag_hint"))
            .size(11)
            .font(fonts::Inter::REGULAR)
            .color(color::text_muted()),
    ]
    .align_y(alignment::Vertical::Center);
    let name = row![
        text(shown_name(page))
            .size(18)
            .font(fonts::Inter::SEMI_BOLD)
            .color(color::text_headline()),
        Space::new().width(Fill),
        model_control(page),
    ]
    .spacing(10)
    .align_y(alignment::Vertical::Center);
    let card = column![eyebrow, name, actions(page, account)].spacing(10).width(Fill);
    container(card)
        .padding(Padding::new(16.0))
        .width(Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(Color {
                a: 0.9,
                ..color::surface_bg1()
            })),
            border: Border {
                radius: 16.0.into(),
                width: 1.0,
                color: color::surface_line(),
            },
            ..container::Style::default()
        })
        .into()
}

/// The model of the chosen skin: the classic and slim switch for a saved skin, and the model name for any other.
fn model_control<'a>(page: &'a Skins) -> Element<'a, Message> {
    match page.choice() {
        Choice::Saved(_) => segmented(
            &[(false, lang::t("skins.classic")), (true, lang::t("skins.slim"))],
            page.slim(),
            Message::Slim,
        ),
        _ => text(model_name(page.shown_slim()))
            .size(12)
            .font(fonts::JetBrainsMono::SEMI_BOLD)
            .color(color::text_muted())
            .into(),
    }
}

/// The name of the chosen skin: the account's own, a saved or archived skin's, or a bundled skin's.
fn shown_name(page: &Skins) -> String {
    match page.choice() {
        Choice::Current => lang::t("skins.current_name").to_owned(),
        Choice::Bundled(index) => BUNDLED.get(index).map_or_else(String::new, |skin| skin.name.to_owned()),
        Choice::Saved(_) | Choice::Archived(_) => page.shown_saved().map_or_else(String::new, |skin| skin.name.clone()),
    }
}

/// The name of a model.
fn model_name(slim: bool) -> &'static str {
    if slim { lang::t("skins.slim") } else { lang::t("skins.classic") }
}

/// The buttons on the chosen skin's card: use it, show the current skin again, or take an archived skin out of the archive.
/// Changing the account's skin needs a Microsoft account, so an offline account is told so, and sent to Settings.
fn actions<'a>(page: &'a Skins, account: Account<'a>) -> Element<'a, Message> {
    let mut controls: Row<'a, Message> = Row::new().spacing(10).align_y(alignment::Vertical::Center);
    if page.choice() != Choice::Current {
        let can_change = account.microsoft && !page.busy();
        controls = controls
            .push(TextButton::primary(lang::t("skins.use")).disabled(!can_change).on_press(Message::Use))
            .push(TextButton::ghost(lang::t("skins.show_current")).on_press(Message::Choose(Choice::Current)));
        if let Choice::Archived(index) = page.choice() {
            controls = controls.push(TextButton::ghost(lang::t("skins.remove")).on_press(Message::RemoveArchived(index)));
        }
    }
    let mut block = column![controls].spacing(10);
    if !account.microsoft {
        block = block
            .push(
                text(lang::t("skins.needs_microsoft"))
                    .size(12)
                    .font(fonts::Inter::REGULAR)
                    .color(color::text_muted()),
            )
            .push(TextButton::ghost(lang::t("skins.open_settings")).on_press(Message::Navigate(Page::Settings)));
    }
    block.into()
}

/// The layer switches beside the viewer: an icon for each, named on hover.
fn layer_column<'a>(layers: Layers) -> Element<'a, Message> {
    let switches = [
        (Layer::Hat, layers.hat, Icon::lucide().hat_glasses(), lang::t("skins.layer_hat")),
        (Layer::Jacket, layers.jacket, Icon::lucide().shirt(), lang::t("skins.layer_jacket")),
        (Layer::Sleeves, layers.sleeves, Icon::lucide().hand(), lang::t("skins.layer_sleeves")),
        (Layer::Pants, layers.pants, Icon::lucide().columns_2(), lang::t("skins.layer_pants")),
    ];
    column(switches.map(|(layer, on, icon, name)| icon_choice(icon, name, on, Message::Layer(layer))))
        .spacing(8)
        .into()
}

/// The pose choices beside the viewer: an icon for each, named on hover.
fn pose_column<'a>(pose: Pose) -> Element<'a, Message> {
    let choices = [
        (Pose::Stand, Icon::lucide().person_standing(), lang::t("skins.pose_stand")),
        (Pose::Arms, Icon::lucide().accessibility(), lang::t("skins.pose_arms")),
        (Pose::Walk, Icon::lucide().footprints(), lang::t("skins.pose_walk")),
        (Pose::Run, Icon::lucide().wind(), lang::t("skins.pose_run")),
        (Pose::Crouch, Icon::lucide().chevrons_down(), lang::t("skins.pose_crouch")),
    ];
    column(choices.map(|(choice, icon, name)| icon_choice(icon, name, choice == pose, Message::Pose(choice))))
        .spacing(8)
        .into()
}

/// A square icon switch that shows whether it is on, and sends `on_press` when pressed. Its name shows on hover, because the
/// icon alone does not say what the switch does.
fn icon_choice<'a>(icon: &'static str, name: &'static str, on: bool, on_press: Message) -> Element<'a, Message> {
    let ink = if on { color::primary() } else { color::text_muted() };
    let glyph: Element<'a, Message> = centred(utility::icon(icon, 16.0, Some((ink, ink))));
    hint(
        button(glyph)
            .width(36)
            .height(36)
            .padding(0)
            .on_press(on_press)
            .style(move |_, _| button::Style {
                background: Some(Background::Color(if on { color::primary_tint() } else { color::surface_bg3() })),
                border: Border {
                    radius: 10.0.into(),
                    width: 1.0,
                    color: if on { color::primary_glow() } else { color::surface_line() },
                },
                ..button::Style::default()
            }),
        name,
    )
}

/// The account's capes: "No cape" first, then each cape the account owns. The one the viewer shows is ringed, and a press shows
/// it in the viewer. Save applies it.
fn capes<'a>(page: &'a Skins, account: Account<'a>, width: f32) -> Element<'a, Message> {
    let worn = account.capes.iter().find(|cape| cape.active).map(|cape| cape.id.as_str());
    let pick = page.cape_pick();
    let none_selected = match pick {
        CapePick::Off => true,
        CapePick::Worn => worn.is_none(),
        CapePick::Owned(_) => false,
    };
    let mut cells: Vec<Element<'a, Message>> = vec![cape_tile(
        glyph(Icon::lucide().circle_x()),
        lang::t("skins.capes.none").to_owned(),
        none_selected,
        Message::PickCape(CapePick::Off),
    )];
    for cape in account.capes {
        let selected = match pick {
            CapePick::Off => false,
            CapePick::Worn => cape.active,
            CapePick::Owned(id) => id == &cape.id,
        };
        let picture: Element<'a, Message> = match page.cape_face(&cape.id) {
            Some(handle) => image(handle.clone())
                .width(CAPE_PICTURE_WIDTH)
                .height(CAPE_PICTURE_HEIGHT)
                .filter_method(FilterMethod::Nearest)
                .into(),
            None => Space::new().width(CAPE_PICTURE_WIDTH).height(CAPE_PICTURE_HEIGHT).into(),
        };
        cells.push(cape_tile(
            picture,
            cape.name.clone(),
            selected,
            Message::PickCape(CapePick::Owned(cape.id.clone())),
        ));
    }
    let columns = ((width + GAP) / (CAPE_TILE_WIDTH + GAP)).floor().max(1.0) as usize;
    let title = text(lang::t("skins.capes.title"))
        .size(13)
        .font(fonts::Inter::SEMI_BOLD)
        .color(color::text_headline());
    let mut header = row![title, Space::new().width(Fill)].align_y(alignment::Vertical::Center);
    if !account.capes.is_empty() {
        let can_save = account.microsoft && !page.busy() && page.cape_pending(worn);
        header = header.push(
            TextButton::primary(lang::t("skins.cape.save"))
                .disabled(!can_save)
                .on_press(Message::SaveCape),
        );
    }
    let mut block = column![header, grid(cells, columns, GAP)].spacing(12).width(Fill);
    if account.capes.is_empty() {
        block = block.push(
            text(lang::t("skins.capes.empty"))
                .size(12)
                .font(fonts::Inter::REGULAR)
                .color(color::text_muted()),
        );
    }
    block.into()
}

/// One cape's tile: its picture and name, pressed to wear the cape.
fn cape_tile<'a>(picture: Element<'a, Message>, name: String, selected: bool, on_press: Message) -> Element<'a, Message> {
    let content = column![picture, text(name).size(11).font(fonts::Inter::REGULAR).color(color::text_muted())]
        .spacing(6)
        .align_x(alignment::Horizontal::Center);
    tile(centred(content), CAPE_TILE_WIDTH, CAPE_TILE_HEIGHT, 12.0, selected, on_press)
}

/// An icon, drawn as the picture of a tile.
fn glyph<'a>(icon: &'static str) -> Element<'a, Message> {
    container(utility::icon(icon, 28.0, Some((color::text_muted(), color::text_muted()))))
        .height(40)
        .into()
}

/// The skins the page lists: the saved skins, the archive, then each pack of bundled skins. The packs start folded away.
fn sections<'a>(page: &'a Skins, columns: usize) -> Element<'a, Message> {
    let folder = TextButton::ghost(lang::t("skins.open_folder")).on_press(Message::OpenFolder);
    let mut blocks: Vec<Element<'a, Message>> = vec![
        section(
            page,
            Section::Saved,
            lang::t("skins.section.saved").to_owned(),
            lang::t_count("skins.count", page.saved().len()),
            Some(folder.into()),
            || {
                if page.saved().is_empty() {
                    return empty(lang::t("skins.empty.saved"));
                }
                let cells = page
                    .saved()
                    .iter()
                    .enumerate()
                    .map(|(index, skin)| skin_tile(page, page.saved_preview(index), skin.name.clone(), Choice::Saved(index)))
                    .collect();
                grid(cells, columns, GAP)
            },
        ),
        section(
            page,
            Section::Archived,
            lang::t("skins.section.archived").to_owned(),
            lang::t_count("skins.count", page.archived().len()),
            None,
            || {
                if page.archived().is_empty() {
                    return empty(lang::t("skins.empty.archived"));
                }
                let cells = page
                    .archived()
                    .iter()
                    .enumerate()
                    .map(|(index, skin)| skin_tile(page, page.archived_preview(index), skin.name.clone(), Choice::Archived(index)))
                    .collect();
                grid(cells, columns, GAP)
            },
        ),
    ];
    for pack in Pack::ALL {
        blocks.push(pack_section(page, pack, columns));
    }
    column(blocks).spacing(28).width(Fill).into()
}

/// One pack of bundled skins as a section. The vanilla defaults are named in the language, and the packs by Mojang's titles.
fn pack_section<'a>(page: &'a Skins, pack: Pack, columns: usize) -> Element<'a, Message> {
    let title = pack.title().map_or_else(|| lang::t("skins.section.defaults").to_owned(), str::to_owned);
    let members: Vec<usize> = BUNDLED
        .iter()
        .enumerate()
        .filter(|(_, skin)| skin.pack == pack)
        .map(|(index, _)| index)
        .collect();
    section(
        page,
        Section::Pack(pack),
        title,
        lang::t_count("skins.count", members.len()),
        None,
        || {
            let cells = members
                .iter()
                .map(|&index| skin_tile(page, page.bundled_preview(index), BUNDLED[index].name.to_owned(), Choice::Bundled(index)))
                .collect();
            grid(cells, columns, GAP)
        },
    )
}

/// A section: its heading, which folds the section away when pressed, the actions beside the heading, and the section's tiles
/// while it is open. The tiles are built only while the section is open.
fn section<'a>(
    page: &Skins,
    which: Section,
    title: String,
    count: String,
    actions: Option<Element<'a, Message>>,
    tiles: impl FnOnce() -> Element<'a, Message>,
) -> Element<'a, Message> {
    let collapsed = page.collapsed(which);
    let chevron = if collapsed {
        Icon::lucide().chevron_right()
    } else {
        Icon::lucide().chevron_down()
    };
    let heading = button(
        row![
            container(utility::icon(chevron, 14.0, Some((color::text_muted(), color::text_muted())))),
            text(title).size(15).font(fonts::Inter::SEMI_BOLD).color(color::text_headline()),
            text(count).size(12).font(fonts::Inter::REGULAR).color(color::text_muted()),
        ]
        .spacing(8)
        .align_y(alignment::Vertical::Center),
    )
    .padding(0)
    .on_press(Message::ToggleSection(which))
    .style(|_, _| button::Style {
        background: None,
        text_color: color::text_headline(),
        border: Border::default(),
        ..button::Style::default()
    });
    let mut header = row![heading, Space::new().width(Fill)].align_y(alignment::Vertical::Center);
    if let Some(actions) = actions {
        header = header.push(actions);
    }
    if collapsed {
        return header.into();
    }
    column![header, tiles()].spacing(14).width(Fill).into()
}

/// A skin's tile: its 3D preview and its name. The tile rings when the skin is the one chosen.
fn skin_tile<'a>(page: &Skins, preview: Option<&Handle>, name: String, choice: Choice) -> Element<'a, Message> {
    let picture: Element<'a, Message> = match preview {
        Some(handle) => image(handle.clone()).width(PREVIEW_WIDTH).height(PREVIEW_HEIGHT).into(),
        None => Space::new().width(PREVIEW_WIDTH).height(PREVIEW_HEIGHT).into(),
    };
    let content = column![picture, text(name).size(11).font(fonts::Inter::REGULAR).color(color::text_muted())]
        .spacing(6)
        .align_x(alignment::Horizontal::Center);
    tile(
        centred(content),
        TILE_WIDTH,
        TILE_HEIGHT,
        12.0,
        page.choice() == choice,
        Message::Choose(choice),
    )
}

/// A line that says a section has nothing in it.
fn empty<'a>(message: &'static str) -> Element<'a, Message> {
    text(message).size(12).font(fonts::Inter::REGULAR).color(color::text_muted()).into()
}

/// Its content, centred in both directions and filling its box.
fn centred<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(content)
        .width(Fill)
        .height(Fill)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .into()
}
