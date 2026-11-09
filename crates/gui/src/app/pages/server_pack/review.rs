//! The Review step: the mods the scan read, sorted by side, each with the choice it takes, and the instance's folders to
//! tick. The footer counts what the pack takes, and continues to the Export step.

use aurora_leaf::Icon;
use iced::widget::{Space, button, column, container, image, mouse_area, row, text};
use iced::{Alignment, Background, Border, Element, Fill, Length, Padding, Size};
use lodestone_export::server_pack::Side;
use lodestone_export::server_pack::plan::Choice;

use super::{Counts, Filter, FolderRow, Message, ModRow, Sort, State, Tab, steps};
use crate::app::components::badge::Badge;
use crate::app::components::checkbox::{self, State as Box};
use crate::app::components::input;
use crate::app::components::page::{PAGE_SIDE, content_width, is_narrow};
use crate::app::components::panel::panel_with_padding;
use crate::app::components::scrollbar::scroll;
use crate::app::components::stepper::stepper;
use crate::app::components::table::{self, Column, NAME_MIN};
use crate::app::components::typography;
use crate::app::components::{Select, Tabs, TextButton};
use crate::app::lang;
use crate::app::theme::{color, fonts, utility};

/// The width of the side column in the mod list.
const SIDE_WIDTH: f32 = 130.0;
/// The width of the evidence column in the mod list.
const EVIDENCE_WIDTH: f32 = 200.0;
/// The width of the choice column in the mod list.
const CHOICE_WIDTH: f32 = 150.0;
/// The width of the filter rail.
const RAIL_WIDTH: f32 = 210.0;
/// The width of the filter rail on a narrow window, where the list needs the room.
const NARROW_RAIL_WIDTH: f32 = 150.0;
/// The gap between the list's cells, the gap between the rail and the list, and the padding of each row either side.
const CELL_GAP: f32 = 12.0;
const RAIL_GAP: f32 = 20.0;
const ROW_PAD: f32 = 8.0;
/// The page's padding either side, inside the step's margins.
const PAGE_PAD: f32 = 32.0;

/// Which of the list's optional columns show, in the order [side, evidence]. The evidence hides first, then the side. The
/// choice always shows.
type Shown = [bool; 2];

/// The columns that show in a list `list_width` pixels wide.
fn shown_columns(list_width: f32) -> Shown {
    const COLUMNS: [Column; 3] = [
        Column {
            width: SIDE_WIDTH,
            hide: Some(1),
        },
        Column {
            width: EVIDENCE_WIDTH,
            hide: Some(0),
        },
        Column {
            width: CHOICE_WIDTH,
            hide: None,
        },
    ];
    let shown = table::fit(&COLUMNS, list_width - 2.0 * ROW_PAD, NAME_MIN, CELL_GAP);
    [shown[0], shown[1]]
}
/// The side of a row's icon.
const ICON: f32 = 28.0;

/// The Review step, with its tab and footer. `window` is the window's inner size, which sets the rail and the list's columns.
pub fn view(state: &State, window: Size) -> Element<'_, Message> {
    let body = match state.tab {
        Tab::Mods => mods_tab(state, window),
        Tab::Folders => folders_tab(state),
    };
    let caption = state.folder.display().to_string();
    column![
        stepper(&steps(), 0, Some(caption)),
        container(tabs(state)).padding(Padding::new(0.0).left(32.0).right(32.0).top(12.0)),
        container(body)
            .width(Fill)
            .height(Fill)
            .padding(Padding::new(0.0).left(32.0).right(32.0).top(12.0)),
        footer(state),
    ]
    .width(Fill)
    .height(Fill)
    .into()
}

/// The Mods and Folders tabs, with the one in use lit.
fn tabs(state: &State) -> Element<'_, Message> {
    let active = usize::from(state.tab == Tab::Folders);
    Tabs::new([lang::t("server_pack.review.tab.mods"), lang::t("server_pack.review.tab.folders")])
        .active(active)
        .on_select(|index| Message::Tab(if index == 0 { Tab::Mods } else { Tab::Folders }))
        .underline()
        .into()
}

/// The Mods tab: the filter rail beside the list, with the batch bar when rows are selected. A narrow window narrows the rail,
/// and the list drops its least useful columns to fit what is left.
fn mods_tab(state: &State, window: Size) -> Element<'_, Message> {
    let counts = state.counts();
    let rail_width = if is_narrow(window.width) { NARROW_RAIL_WIDTH } else { RAIL_WIDTH };
    let rail = container(rail(state, &counts)).width(Length::Fixed(rail_width)).height(Fill);
    let list_width = content_width(window.width) - 2.0 * (PAGE_PAD - PAGE_SIDE) - rail_width - RAIL_GAP;
    let shown = shown_columns(list_width);
    let mut list = column![toolbar(state), header_row(state, shown), list_rows(state, shown)]
        .spacing(10)
        .width(Fill)
        .height(Fill);
    if !state.selected.is_empty() {
        list = list.push(batch_bar(state));
    }
    row![rail, list].spacing(RAIL_GAP).width(Fill).height(Fill).into()
}

/// The filter rail: every side with its count, and a note for the unsure mods.
fn rail<'a>(state: &'a State, counts: &Counts) -> Element<'a, Message> {
    let total = state.mods.len();
    let mut items = column![
        rail_item(lang::t("server_pack.review.rail.all"), total, None, state.filter, Filter::All),
        rail_item(
            lang::t("server_pack.review.rail.client"),
            counts.client,
            Some(color::cyan()),
            state.filter,
            Filter::Client
        ),
        rail_item(
            lang::t("server_pack.review.rail.server"),
            counts.server,
            Some(color::primary()),
            state.filter,
            Filter::Server
        ),
        rail_item(
            lang::t("server_pack.review.rail.both"),
            counts.both,
            Some(color::text_muted()),
            state.filter,
            Filter::Both
        ),
        rail_item(
            lang::t("server_pack.review.rail.unsure"),
            counts.unsure,
            Some(color::amber()),
            state.filter,
            Filter::Unsure
        ),
    ]
    .spacing(4);
    if counts.unsure > 0 {
        let count = counts.unsure.to_string();
        let note = lang::t_with("server_pack.review.unsure_note", &[("COUNT", count.as_str())]);
        items = items.push(Space::new().height(Fill));
        items = items.push(
            container(typography::small(note).color(color::text_muted()))
                .padding(Padding::new(12.0))
                .width(Fill)
                .style(|_| container::Style {
                    background: Some(Background::Color(color::amber_tint())),
                    border: Border {
                        color: color::amber(),
                        width: 1.0,
                        radius: 8.0.into(),
                    },
                    ..Default::default()
                }),
        );
    }
    items.into()
}

/// One row of the rail: its dot, its label and its count. The active one is lit.
fn rail_item<'a>(label: &'static str, count: usize, dot: Option<iced::Color>, active: Filter, this: Filter) -> Element<'a, Message> {
    let lit = active == this;
    let ink = if lit { color::primary() } else { color::text_muted() };
    let mut content = row![].spacing(8).align_y(Alignment::Center);
    if let Some(dot) = dot {
        content = content.push(container(Space::new()).width(8).height(8).style(move |_| container::Style {
            background: Some(Background::Color(dot)),
            border: Border {
                radius: 4.0.into(),
                ..Default::default()
            },
            ..Default::default()
        }));
    }
    content = content.push(text(label).size(12.5).font(fonts::Inter::MEDIUM).color(ink));
    content = content.push(Space::new().width(Fill));
    content = content.push(text(count.to_string()).size(11.0).font(fonts::JetBrainsMono::REGULAR).color(ink));
    button(content)
        .on_press(Message::Filter(this))
        .padding(Padding::new(0.0).left(12.0).right(12.0).top(9.0).bottom(9.0))
        .width(Fill)
        .style(move |_, _| button::Style {
            background: lit.then(|| Background::Color(color::primary_tint())),
            text_color: ink,
            border: Border {
                radius: 8.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}

/// The search field, and the sort menu beside it.
fn toolbar(state: &State) -> Element<'_, Message> {
    let search = iced::widget::text_input(lang::t("server_pack.review.search"), &state.search)
        .on_input(Message::Search)
        .style(input::style)
        .size(13)
        .padding(Padding::new(0.0).left(12.0).right(12.0).top(9.0).bottom(9.0))
        .width(Fill);
    let sort_index = match state.sort {
        Sort::Side => 0,
        Sort::Name => 1,
    };
    let sort: Element<'_, Message> = Select::new(
        [lang::t("server_pack.review.sort.side"), lang::t("server_pack.review.sort.name")],
        Some(sort_index),
    )
    .on_select(|index| Message::Sort(if index == 0 { Sort::Side } else { Sort::Name }))
    .width(Length::Fixed(190.0))
    .into();
    row![search, sort].spacing(12).align_y(Alignment::Center).width(Fill).into()
}

/// The column headings above the list. The first one is the select-all checkbox.
fn header_row(state: &State, shown: Shown) -> Element<'_, Message> {
    let all = !state.visible.is_empty() && state.visible.iter().all(|index| state.selected.contains(index));
    let some = state.visible.iter().any(|index| state.selected.contains(index));
    let box_state = if all {
        Box::Checked
    } else if some {
        Box::Partial
    } else {
        Box::Unchecked
    };
    let select_all = mouse_area(checkbox::mark(box_state)).on_press(Message::SelectVisible);
    let heading = |label: &'static str| typography::eyebrow(label).color(color::text_muted());
    let mut cells: Vec<Element<'_, Message>> = vec![
        row![select_all, heading(lang::t("server_pack.review.column.mod"))]
            .spacing(12)
            .align_y(Alignment::Center)
            .width(Fill)
            .into(),
    ];
    if shown[0] {
        cells.push(
            container(heading(lang::t("server_pack.review.column.side")))
                .width(Length::Fixed(SIDE_WIDTH))
                .into(),
        );
    }
    if shown[1] {
        cells.push(
            container(heading(lang::t("server_pack.review.column.evidence")))
                .width(Length::Fixed(EVIDENCE_WIDTH))
                .into(),
        );
    }
    cells.push(
        container(heading(lang::t("server_pack.review.column.pack")))
            .width(Length::Fixed(CHOICE_WIDTH))
            .into(),
    );
    row(cells)
        .spacing(CELL_GAP)
        .align_y(Alignment::Center)
        .padding(Padding::new(0.0).left(ROW_PAD).right(ROW_PAD))
        .width(Fill)
        .into()
}

/// The rows of the visible mods, in a scrollable list, or a note when none show.
fn list_rows(state: &State, shown: Shown) -> Element<'_, Message> {
    if state.mods.is_empty() {
        return panel_with_padding(
            typography::body(lang::t("server_pack.review.no_mods")).color(color::text_muted()),
            Padding::new(18.0),
        );
    }
    if state.visible.is_empty() {
        return container(typography::body(lang::t("server_pack.review.empty")).color(color::text_muted()))
            .padding(Padding::new(18.0))
            .width(Fill)
            .into();
    }
    let rows: Vec<Element<'_, Message>> = state.visible.iter().map(|&index| mod_row(state, index, shown)).collect();
    scroll(column(rows).spacing(2).width(Fill)).width(Fill).height(Fill).into()
}

/// One mod's row: its checkbox and icon, its name and file, its side, its evidence and its choice.
fn mod_row(state: &State, index: usize, shown: Shown) -> Element<'_, Message> {
    let Some(row_data) = state.mods.get(index) else {
        return Space::new().into();
    };
    let detection = &row_data.detection;
    let selected = state.selected.contains(&index);
    let box_state = if selected { Box::Checked } else { Box::Unchecked };
    let check = checkbox::mark(box_state);
    let mut name = column![
        typography::body(pretty_name(&detection.filename)).color(color::text_default()),
        text(detection.filename.clone())
            .size(11.0)
            .font(fonts::JetBrainsMono::REGULAR)
            .color(color::text_muted()),
    ]
    .spacing(2);
    // The columns a narrow list drops move under the name, so the row keeps what they say.
    if !shown[0] {
        name = name.push(side_badge(detection.side.clone()));
    }
    if !shown[1] {
        name = name.push(
            text(detection.evidence.clone())
                .size(11.0)
                .font(fonts::JetBrainsMono::REGULAR)
                .color(color::text_muted()),
        );
    }
    let left = row![check, icon(row_data), name].spacing(12).align_y(Alignment::Center).width(Fill);
    let mut cells: Vec<Element<'_, Message>> = vec![left.into()];
    if shown[0] {
        cells.push(container(side_badge(detection.side.clone())).width(Length::Fixed(SIDE_WIDTH)).into());
    }
    if shown[1] {
        cells.push(
            container(
                text(detection.evidence.clone())
                    .size(11.0)
                    .font(fonts::JetBrainsMono::REGULAR)
                    .color(color::text_muted()),
            )
            .width(Length::Fixed(EVIDENCE_WIDTH))
            .into(),
        );
    }
    let choice = choice_select(index, state.choice_of(index));
    cells.push(container(choice).width(Length::Fixed(CHOICE_WIDTH)).into());
    let content = row(cells)
        .spacing(CELL_GAP)
        .align_y(Alignment::Center)
        .padding(Padding::new(0.0).left(ROW_PAD).right(ROW_PAD).top(8.0).bottom(8.0))
        .width(Fill);
    let background = if selected {
        Some(Background::Color(color::primary_tint()))
    } else {
        None
    };
    mouse_area(container(content).width(Fill).style(move |_| container::Style {
        background,
        border: Border {
            radius: 8.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }))
    .on_press(Message::Select(index))
    .into()
}

/// The row's icon: the mod's own, or a package glyph when it declares none.
fn icon(row_data: &ModRow) -> Element<'static, Message> {
    let picture: Element<'static, Message> = match &row_data.icon {
        Some(handle) => image(handle.clone()).width(ICON).height(ICON).into(),
        None => utility::icon(Icon::lucide().package(), 16.0, Some((color::text_muted(), color::text_muted()))),
    };
    container(picture)
        .width(ICON)
        .height(ICON)
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .style(|_| container::Style {
            background: Some(Background::Color(color::surface_bg3())),
            border: Border {
                radius: 6.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}

/// The side a mod was detected on, as a badge.
fn side_badge<'a>(side: Option<Side>) -> Element<'a, Message> {
    let (label, ink, fill) = match side {
        Some(Side::ClientOnly) => (lang::t("server_pack.side.client"), color::cyan(), color::cyan_tint()),
        Some(Side::ServerOnly) => (lang::t("server_pack.side.server"), color::primary(), color::primary_tint()),
        Some(Side::ClientAndServer) => (lang::t("server_pack.side.both"), color::text_muted(), color::surface_bg3()),
        None => (lang::t("server_pack.side.unsure"), color::amber(), color::amber_tint()),
    };
    Badge::new(label, ink, fill).into()
}

/// The choice a mod takes, as a menu of Auto, Include and Exclude.
fn choice_select(index: usize, choice: Option<Choice>) -> Element<'static, Message> {
    let selected = match choice {
        None => 0,
        Some(Choice::Include) => 1,
        Some(Choice::Exclude) => 2,
    };
    Select::new(choice_labels(), Some(selected))
        .on_select(move |picked| Message::Choose {
            mods: vec![index],
            choice: choice_at(picked),
        })
        .width(Length::Fixed(CHOICE_WIDTH - 8.0))
        .into()
}

/// The labels of the choice menu, in the order [`choice_at`] reads them.
fn choice_labels() -> [&'static str; 3] {
    [
        lang::t("server_pack.review.choice.auto"),
        lang::t("server_pack.review.choice.include"),
        lang::t("server_pack.review.choice.exclude"),
    ]
}

/// The choice an index of the choice menu names: `None` is Auto.
fn choice_at(index: usize) -> Option<Choice> {
    match index {
        1 => Some(Choice::Include),
        2 => Some(Choice::Exclude),
        _ => None,
    }
}

/// The bar shown while rows are selected: how many, the side to set on them, and clearing the selection.
fn batch_bar(state: &State) -> Element<'_, Message> {
    let mut mods: Vec<usize> = state.selected.iter().copied().collect();
    mods.sort_unstable();
    let selected_count = mods.len().to_string();
    let count = lang::t_with("server_pack.review.selected", &[("COUNT", selected_count.as_str())]);
    let set: Element<'_, Message> = Select::new(choice_labels(), None::<usize>)
        .placeholder(lang::t("server_pack.review.set_pack"))
        .on_select(move |picked| Message::Choose {
            mods: mods.clone(),
            choice: choice_at(picked),
        })
        .width(Length::Fixed(190.0))
        .into();
    let clear: Element<'_, Message> = TextButton::ghost(lang::t("server_pack.review.clear"))
        .on_press(Message::ClearSelection)
        .into();
    container(
        row![
            text(count).size(13.0).font(fonts::Inter::SEMI_BOLD).color(color::text_default()),
            set,
            Space::new().width(Fill),
            clear
        ]
        .spacing(12)
        .align_y(Alignment::Center),
    )
    .padding(Padding::new(0.0).left(14.0).right(14.0).top(10.0).bottom(10.0))
    .width(Fill)
    .style(|_| container::Style {
        background: Some(Background::Color(color::surface_bg2())),
        border: Border {
            color: color::surface_line(),
            width: 1.0,
            radius: 10.0.into(),
        },
        ..Default::default()
    })
    .into()
}

/// The Folders tab: each top-level folder of the instance, ticked when the pack takes it.
fn folders_tab(state: &State) -> Element<'_, Message> {
    if state.folders.is_empty() {
        return typography::body(lang::t("server_pack.review.folders.empty"))
            .color(color::text_muted())
            .into();
    }
    let note = typography::small(lang::t("server_pack.review.folders.note")).color(color::text_muted());
    let rows: Vec<Element<'_, Message>> = state
        .folders
        .iter()
        .enumerate()
        .map(|(index, folder)| folder_row(state, index, folder))
        .collect();
    column![note, scroll(column(rows).spacing(2).width(Fill)).width(Fill).height(Fill)]
        .spacing(12)
        .width(Fill)
        .height(Fill)
        .into()
}

/// One folder's row: its checkbox, its name, and whether it is a folder or a file.
fn folder_row<'a>(state: &'a State, index: usize, folder: &'a FolderRow) -> Element<'a, Message> {
    let box_state = if state.folder_ticked(index) { Box::Checked } else { Box::Unchecked };
    let kind = if folder.is_folder {
        lang::t("server_pack.review.folders.folder")
    } else {
        lang::t("server_pack.review.folders.file")
    };
    let glyph = if folder.is_folder {
        Icon::lucide().folder()
    } else {
        Icon::lucide().file()
    };
    row![
        checkbox::checkbox(box_state, Message::Folder(index)),
        utility::icon(glyph, 16.0, Some((color::text_muted(), color::text_muted()))),
        typography::body(folder.name.clone()).color(color::text_default()),
        Space::new().width(Fill),
        typography::small(kind).color(color::text_muted()),
    ]
    .spacing(12)
    .align_y(Alignment::Center)
    .padding(Padding::new(0.0).left(12.0).right(12.0).top(10.0).bottom(10.0))
    .width(Fill)
    .into()
}

/// The footer: what the pack takes, and the buttons that leave the step or continue to Export.
fn footer(state: &State) -> Element<'_, Message> {
    let counts = state.counts();
    let (taken, left_out) = (counts.taken.to_string(), counts.left_out.to_string());
    let summary = lang::t_with("server_pack.review.footer", &[("IN", taken.as_str()), ("OUT", left_out.as_str())]);
    let back: Element<'_, Message> = TextButton::ghost(lang::t("server_pack.back")).on_press(Message::Back).into();
    let cont: Element<'_, Message> = TextButton::primary(lang::t("server_pack.review.continue"))
        .on_press(Message::Continue)
        .into();
    container(
        row![
            text(summary).size(12.5).font(fonts::Inter::MEDIUM).color(color::text_muted()),
            Space::new().width(Fill),
            back,
            cont,
        ]
        .spacing(12)
        .align_y(Alignment::Center),
    )
    .padding(Padding::new(0.0).left(32.0).right(32.0).top(14.0).bottom(14.0))
    .width(Fill)
    .style(|_| container::Style {
        background: Some(Background::Color(color::surface_bg2())),
        border: Border {
            color: color::surface_line(),
            width: 1.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    })
    .into()
}

/// A mod's display name, from its file name: `sodium-fabric-0.6.13.jar` reads as "Sodium Fabric".
pub fn pretty_name(filename: &str) -> String {
    let base = filename
        .strip_suffix(".jar")
        .or_else(|| filename.strip_suffix(".JAR"))
        .unwrap_or(filename);
    let end = base
        .char_indices()
        .find(|&(index, character)| matches!(character, '-' | '_' | ' ') && base[index + 1..].starts_with(|next: char| next.is_ascii_digit()))
        .map_or(base.len(), |(index, _)| index);
    let words: Vec<String> = base[..end]
        .split(['-', '_', ' '])
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect(),
                None => String::new(),
            }
        })
        .collect();
    if words.is_empty() { base.to_string() } else { words.join(" ") }
}

#[cfg(test)]
#[path = "../../../../tests/app/pages/server_pack/review.rs"]
mod tests;
