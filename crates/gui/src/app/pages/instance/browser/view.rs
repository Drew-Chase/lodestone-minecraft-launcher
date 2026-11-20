//! The Files tab's layout: the path of the folder on screen with a way up, the column headings, and the rows. Only the
//! rows the scroll reveals are built, so a folder of thousands of files scrolls as a short one does.

use aurora_leaf::Icon;
use iced::widget::{Space, Text, button, column, container, mouse_area, responsive, row, stack, svg, text_input};
use iced::{Background, Border, Color, Element, Fill, Length, Padding, Point, Size, alignment, mouse};
use iced_color_extension::rgb;

use super::menu::{self, Menu, Target};
use super::search::{self, Message as SearchMessage};
use super::{Action, Message, State, drag};
use crate::app::components::context_menu::ContextMenu;
use crate::app::components::dropdown::Dropdown;
use crate::app::components::hint::hint;
use crate::app::components::hover_track::HoverTrack;
use crate::app::components::icon_button::{self, IconButton};
use crate::app::components::keyed_list::{self, KeyedList};
use crate::app::components::panel::{hairline, panel_with_padding};
use crate::app::components::pointer_track::PointerTrack;
use crate::app::components::table::{self, Column as TableColumn};
use crate::app::components::{TextButton, input, scrollbar};
use crate::app::folder_listing::{Column, Row};
use crate::app::lang;
use crate::app::pages::instance::format::format_bytes;
use crate::app::theme::{color, fonts, utility};

/// The id of the scrollable the rows sit in, so a new folder starts at its top.
pub const SCROLL_ID: &str = "instance-files-list";
/// The id of the rename field, so it takes the focus when it appears.
pub const RENAME_ID: &str = "instance-files-rename";
/// The id of the find field, so Ctrl+F takes the focus.
pub const SEARCH_ID: &str = "instance-files-search";
/// A row's height.
pub const ROW_HEIGHT: f32 = 36.0;
/// The width of the list while a file is open in the editor beside it.
const SPLIT_WIDTH: f32 = 300.0;
/// The list height assumed before the list has reported one.
pub const ASSUMED_HEIGHT: f32 = 400.0;

/// The rows built past each edge of the window.
const OVERSCAN: usize = 8;
/// The width of the size column and the modified column.
const SIZE_WIDTH: f32 = 84.0;
const MODIFIED_WIDTH: f32 = 148.0;
/// The ⋯ button is a square this wide, so it keeps room above and below in the row's height.
const MORE_WIDTH: f32 = 28.0;
/// The gap between a row's cells.
const GAP: f32 = 12.0;
/// The room a row keeps either side of its cells.
const ROW_INSET: f32 = 10.0;
/// How far below and right of the pointer the dragged rows' label sits, so it is never under the pointer.
const LABEL_OFFSET: f32 = 16.0;
/// The keys of the spacers that stand in for the rows above and below the window.
const TOP_SPACER: u64 = u64::MAX;
const BOTTOM_SPACER: u64 = u64::MAX - 1;

/// Which of the columns beside the name a list of a given width shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Shown {
    size: bool,
    modified: bool,
}

impl Shown {
    /// The names alone, with no column beside them.
    const NAMES: Self = Self {
        size: false,
        modified: false,
    };
}

/// The columns that fit a list `width` wide. The modified time goes first as the list narrows, then the size.
fn shown(width: f32) -> Shown {
    let columns = [
        TableColumn {
            width: SIZE_WIDTH,
            hide: Some(1),
        },
        TableColumn {
            width: MODIFIED_WIDTH,
            hide: Some(0),
        },
        TableColumn {
            width: MORE_WIDTH,
            hide: None,
        },
    ];
    let fit = table::fit(&columns, width - ROW_INSET * 2.0, table::NAME_MIN, GAP);
    Shown {
        size: fit[0],
        modified: fit[1],
    }
}

/// The glyph of an entry: a folder, a link, or a file by what its name says it holds.
fn glyph(row: &Row) -> &'static str {
    if row.is_folder {
        return if row.is_link {
            Icon::lucide().folder_symlink()
        } else {
            Icon::lucide().folder()
        };
    }
    if row.is_link {
        return Icon::lucide().file_symlink();
    }
    let extension = row.name.rsplit_once('.').map(|(_, extension)| extension.to_ascii_lowercase());
    match extension.as_deref() {
        Some("jar" | "zip" | "gz" | "mrpack" | "lodestone" | "tar" | "xz" | "7z" | "disabled") => Icon::lucide().file_archive(),
        Some("png" | "jpg" | "jpeg" | "gif" | "webp" | "ico" | "bmp") => Icon::lucide().file_image(),
        Some("json" | "json5" | "jsonc" | "mcmeta" | "snbt" | "toml" | "yaml" | "yml" | "xml" | "js" | "zs" | "mcfunction") => {
            Icon::lucide().file_code()
        }
        Some("cfg" | "conf" | "config" | "ini" | "properties" | "lang") => Icon::lucide().file_cog(),
        Some("txt" | "log" | "md") => Icon::lucide().file_text(),
        _ => Icon::lucide().file(),
    }
}

impl State {
    /// The tab: the path bar, then the list, which takes the room that is left and scrolls itself.
    pub fn view(&self) -> Element<'_, Message> {
        let mut list = column![self.path_bar(), self.search_bar()].spacing(12).width(Fill).height(Fill);
        if self.read_only {
            list = list.push(notice(Icon::lucide().lock(), lang::t("instance.files.read_only")));
        }
        let list = list.push(responsive(move |size| self.list(size)));
        // With a file open the list narrows to its names, and the editor takes the rest of the tab.
        let body: Element<'_, Message> = match &self.editor {
            Some(editor) => row![container(list).width(SPLIT_WIDTH).height(Fill), editor.view().map(Message::Editor)]
                .spacing(16)
                .width(Fill)
                .height(Fill)
                .into(),
            None => list.into(),
        };
        // The pointer is followed across the whole tab while a row is held, so a drag reaches the breadcrumbs and ends
        // wherever the button comes up.
        let tracked = PointerTrack::new(body, self.drag.is_armed(), Message::Moved, Message::Released);
        let mut layers = stack![tracked].width(Fill).height(Fill);
        if let (Some(paths), Some(at)) = (self.drag.dragged(), self.drag.pointer()) {
            layers = layers.push(self.drag_label(paths, at));
        }
        if self.file_hover {
            layers = layers.push(drop_overlay());
        }
        layers.into()
    }

    /// The path of the folder on screen: Up, then the instance folder and each folder down to this one, each a way back
    /// and a place to drop dragged rows. New folder and New file sit at the far end.
    fn path_bar(&self) -> Element<'_, Message> {
        let at_root = self.folder.is_empty();
        let up = IconButton::new(Icon::lucide().corner_left_up())
            .size(30)
            .icon_size(14.0)
            .style(icon_button::ghost_button_style());
        let up = if at_root { up.disabled(true) } else { up.on_press(Message::Up) };
        let mut bar = row![hint(up, lang::t("instance.files.up"))]
            .spacing(4)
            .align_y(alignment::Vertical::Center);
        if !at_root {
            // Up is a drop target too: it stands for the folder above.
            let above = drag::parent(&self.folder).to_string();
            let up = bar;
            bar = row![self.drop_zone(up, above)].spacing(4).align_y(alignment::Vertical::Center);
        }

        if self.editor.is_some() {
            // Beside the editor there is room for one name: the folder on screen.
            let name = self.folder.rsplit('/').next().filter(|name| !name.is_empty());
            let label = name.map_or_else(|| lang::t("instance.files.root").to_string(), str::to_string);
            bar = bar.push(container(self.crumb(label, self.folder.clone())).width(Fill).clip(true));
        } else {
            bar = bar.push(self.crumb(lang::t("instance.files.root").to_string(), String::new()));
            let mut folder = String::new();
            for segment in self.folder.split('/').filter(|segment| !segment.is_empty()) {
                folder = if folder.is_empty() {
                    segment.to_string()
                } else {
                    format!("{folder}/{segment}")
                };
                bar = bar.push(Text::new("/").size(12).font(fonts::JetBrainsMono::REGULAR).color(color::text_muted()));
                bar = bar.push(self.crumb(segment.to_string(), folder.clone()));
            }
            bar = bar.push(Space::new().width(Fill));
        }
        if !self.changes_blocked() {
            let new_folder = IconButton::new(Icon::lucide().folder_plus())
                .size(30)
                .icon_size(14.0)
                .style(icon_button::ghost_button_style())
                .on_press(Message::Menu {
                    path: None,
                    action: Some(Action::NewFolder),
                });
            let new_file = IconButton::new(Icon::lucide().file_plus())
                .size(30)
                .icon_size(14.0)
                .style(icon_button::ghost_button_style())
                .on_press(Message::Menu {
                    path: None,
                    action: Some(Action::NewFile),
                });
            bar = bar
                .push(hint(new_folder, lang::t("instance.files.menu.new_folder")))
                .push(hint(new_file, lang::t("instance.files.menu.new_file")));
        }
        bar.into()
    }

    /// The find bar under the path: the field, the two toggles, and a clear button while there is a query. A search that stopped
    /// at its limit says so under the field.
    fn search_bar(&self) -> Element<'_, Message> {
        let field = text_input(lang::t("instance.files.search.placeholder"), &self.search.query)
            .id(SEARCH_ID)
            .on_input(|text| Message::Search(SearchMessage::Query(text)))
            .style(input::style)
            .padding(Padding::new(0.0).left(12).right(12).top(7).bottom(7))
            .size(13)
            .width(Fill);
        let mut bar = row![field].spacing(8).align_y(alignment::Vertical::Center);
        if self.search.active() {
            let clear = IconButton::new(Icon::lucide().x())
                .size(30)
                .icon_size(14.0)
                .style(icon_button::ghost_button_style())
                .on_press(Message::Search(SearchMessage::Clear));
            bar = bar.push(hint(clear, lang::t("instance.files.search.clear")));
        }
        bar = bar.push(search_toggle(
            Icon::lucide().folder_tree(),
            lang::t("instance.files.search.subfolders"),
            self.search.subfolders,
            Message::Search(SearchMessage::Subfolders(!self.search.subfolders)),
        ));
        bar = bar.push(search_toggle(
            Icon::lucide().text_search(),
            lang::t("instance.files.search.contents"),
            self.search.contents,
            Message::Search(SearchMessage::Contents(!self.search.contents)),
        ));
        let mut bar = column![bar].spacing(6).width(Fill);
        if self.search.truncated {
            bar = bar.push(
                Text::new(lang::t_with(
                    "instance.files.search.truncated",
                    &[("count", search::LIMIT.to_string().as_str())],
                ))
                .size(12)
                .font(fonts::Inter::REGULAR)
                .color(color::text_muted()),
            );
        }
        bar.into()
    }

    /// One folder of the path: its name, which opens it, in a zone that takes a drop.
    fn crumb(&self, label: String, folder: String) -> Element<'_, Message> {
        let button = TextButton::ghost(label)
            .on_press(Message::Open(folder.clone()))
            .padding(Padding::new(0.0).left(8).right(8).top(5).bottom(5));
        self.drop_zone(button, folder)
    }

    /// Wraps `content` as a place above the list that stands for `folder`. It lights while dragged rows are over it
    /// and the folder can take them.
    fn drop_zone<'a>(&'a self, content: impl Into<Element<'a, Message>>, folder: String) -> Element<'a, Message> {
        let hovered = self.crumb_hover.as_deref() == Some(folder.as_str());
        let takes = hovered && self.drag.dragged().is_some_and(|dragged| drag::accepts(&folder, dragged));
        let tracked = HoverTrack::new(
            content,
            hovered,
            Message::CrumbHover {
                folder: folder.clone(),
                hovered: true,
            },
            Message::CrumbHover { folder, hovered: false },
        );
        container(tracked).style(move |_| target_ring(takes, 12.0)).into()
    }

    /// The list in its panel: the headings, then the rows in their scroll, or a line saying why there are none.
    fn list(&self, size: Size) -> Element<'_, Message> {
        // Beside the editor the list is a column of names.
        let shown = if self.editor.is_some() { Shown::NAMES } else { shown(size.width) };
        let rows: Element<'_, Message> = if self.search.shown {
            if !self.search.ready {
                status(lang::t("instance.files.search.searching").to_string())
            } else if self.rows.is_empty() {
                status(lang::t_with("instance.files.search.none", &[("query", self.search.query.as_str())]))
            } else {
                self.rows(shown, size.height)
            }
        } else if let Some(error) = &self.error {
            status(lang::t_with("export.files.failed", &[("error", error.as_str())]))
        } else if !self.listed {
            status(lang::t("export.files.reading").to_string())
        } else if self.rows.is_empty() {
            status(lang::t("export.files.empty").to_string())
        } else {
            self.rows(shown, size.height)
        };
        // A press on the empty part of the list selects nothing, and a right-press there opens the folder's own menu.
        let Menu { items, actions } = menu::folder(self.changes_blocked(), true);
        let scroll = scrollbar::scroll(rows)
            .id(SCROLL_ID)
            .on_scroll(|viewport| Message::Scrolled {
                offset: viewport.absolute_offset().y,
                height: viewport.bounds().height,
            })
            .width(Fill)
            .height(Fill);
        let area = mouse_area(scroll).on_press(Message::ClearSelection);
        let area = ContextMenu::new(area, format!("folder:{}", self.folder))
            .items(items)
            .on_select(move |selected| Message::Menu {
                path: None,
                action: actions.get(selected).copied().flatten(),
            });
        panel_with_padding(
            column![self.headings(shown), hairline(), area].width(Fill).height(Fill),
            Padding::new(6.0).top(4.0),
        )
    }

    /// The column headings. Each sorts by its column when pressed, and shows the arrow of the sort it is.
    fn headings(&self, shown: Shown) -> Element<'_, Message> {
        let mut headings = row![
            // The room a row's glyph takes, so the Name heading sits over the names.
            Space::new().width(16.0),
            container(self.heading(lang::t("export.files.name"), Column::Name)).width(Fill),
        ]
        .spacing(GAP)
        .align_y(alignment::Vertical::Center)
        .padding(Padding::new(0.0).left(ROW_INSET).right(ROW_INSET + scrollbar::TRACK_WIDTH))
        .height(30.0);
        if shown.size {
            headings = headings.push(container(self.heading(lang::t("export.files.size"), Column::Size)).width(SIZE_WIDTH));
        }
        if shown.modified {
            headings = headings.push(container(self.heading(lang::t("export.files.modified"), Column::Modified)).width(MODIFIED_WIDTH));
        }
        headings.push(Space::new().width(MORE_WIDTH)).into()
    }

    fn heading(&self, text: &'static str, column: Column) -> Element<'_, Message> {
        let arrow = match (self.sort.column == column, self.sort.descending) {
            (false, _) => None,
            (true, false) => Some(Icon::lucide().arrow_up()),
            (true, true) => Some(Icon::lucide().arrow_down()),
        };
        let content = row![
            Text::new(text).size(11).font(fonts::JetBrainsMono::REGULAR).color(color::text_muted()),
            arrow.map(|markup| utility::icon(markup, 10.0, Some((color::text_muted(), color::text_muted())))),
        ]
        .spacing(4)
        .align_y(alignment::Vertical::Center);
        button(content)
            .padding(0)
            .style(|_, _| button::Style::default())
            .on_press(Message::Sort(column))
            .into()
    }

    /// The rows the scroll reveals, between spacers that hold the height of the rest. Each row is keyed by its path, so
    /// its menu and its rename field stay with it as the window moves.
    fn rows(&self, shown: Shown, height: f32) -> Element<'_, Message> {
        let total = self.order.len();
        let window = keyed_list::visible_rows(self.scroll_y, height, total, ROW_HEIGHT, OVERSCAN);
        let mut children: Vec<(u64, Element<'_, Message>)> = Vec::with_capacity(window.len() + 2);
        children.push((TOP_SPACER, Space::new().height(window.start as f32 * ROW_HEIGHT).into()));
        for &index in &self.order[window.clone()] {
            let row = &self.rows[index];
            children.push((keyed_list::key_of(&row.path), self.row_view(row, shown)));
        }
        children.push((BOTTOM_SPACER, Space::new().height((total - window.end) as f32 * ROW_HEIGHT).into()));
        // The rows stop short of the scrollbar's track, and the list keeps a little room after its last row.
        container(KeyedList::new(children).width(Fill))
            .width(Fill)
            .padding(Padding::new(0.0).right(scrollbar::TRACK_WIDTH).top(4.0).bottom(24.0))
            .into()
    }

    /// One entry's row: its glyph, its name or the field that renames it, its size and modified time, and its menu.
    fn row_view<'a>(&'a self, row: &'a Row, shown: Shown) -> Element<'a, Message> {
        // The file open in the editor is marked in the list, as the selection is.
        let open = self.editor.as_ref().is_some_and(|editor| editor.path() == row.path);
        let picked = self.selection.contains(&row.path);
        let selected = picked || open;
        let hovered = self.hovered.as_deref() == Some(row.path.as_str());
        let dragged = self.drag.dragged();
        // A folder under dragged rows lights when it can take them. The rows being dragged, and rows cut and waiting
        // for their paste, are drawn faint.
        let takes = hovered && row.is_folder && dragged.is_some_and(|dragged| drag::accepts(&row.path, dragged));
        let travelling = dragged.is_some_and(|dragged| dragged.contains(&row.path));
        let faint = travelling || self.is_cut(&row.path);
        let ink = |strong: Color| {
            if faint {
                Color {
                    a: strong.a * 0.45,
                    ..strong
                }
            } else {
                strong
            }
        };

        let glyph_ink = ink(if row.is_folder { color::primary() } else { color::text_muted() });
        let name: Element<'_, Message> = match self.rename.as_ref().filter(|rename| rename.path == row.path) {
            Some(rename) => text_input("", &rename.text)
                .id(RENAME_ID)
                .on_input(Message::RenameInput)
                .on_submit(Message::RenameSubmit)
                .style(input::style)
                .padding(Padding::new(0.0).left(8).right(8).top(5).bottom(5))
                .size(13)
                .width(Fill)
                .into(),
            None => Text::new(row.name.as_str())
                .size(13)
                .font(fonts::Inter::MEDIUM)
                .color(ink(color::text_default()))
                .wrapping(iced::widget::text::Wrapping::None)
                .into(),
        };
        // A match in a search names the folder it is in, in muted type after its name.
        let name: Element<'_, Message> = if self.search.shown {
            row![
                name,
                Text::new(location_of(row))
                    .size(12)
                    .font(fonts::JetBrainsMono::REGULAR)
                    .color(color::text_muted())
                    .wrapping(iced::widget::text::Wrapping::None),
            ]
            .spacing(8)
            .align_y(alignment::Vertical::Center)
            .into()
        } else {
            name
        };
        let mut body = row![
            utility::icon(glyph(row), 16.0, Some((glyph_ink, glyph_ink))),
            container(name).width(Fill).clip(true)
        ]
        .spacing(GAP)
        .align_y(alignment::Vertical::Center)
        .padding(Padding::new(0.0).left(ROW_INSET).right(ROW_INSET))
        .height(Length::Fixed(ROW_HEIGHT));
        if shown.size {
            let size = if row.is_folder { String::new() } else { size_label(row.size) };
            body = body.push(cell(size, SIZE_WIDTH, ink(color::text_muted())));
        }
        if shown.modified {
            let modified = row
                .modified
                .map(|time| time.format("%-m/%-d/%y, %-I:%M %p").to_string())
                .unwrap_or_default();
            body = body.push(cell(modified, MODIFIED_WIDTH, ink(color::text_muted())));
        }

        let count = if picked { self.selection.len() } else { 1 };
        let target = Target {
            is_folder: row.is_folder,
            count,
        };
        let Menu { items, actions } = menu::entry(target, self.changes_blocked(), true);
        let more_path = row.path.clone();
        let more_actions = actions.clone();
        let more = Dropdown::new(Icon::lucide().ellipsis_vertical())
            .items(items.clone())
            .width(MORE_WIDTH)
            .height(MORE_WIDTH)
            .on_select(move |picked| Message::Menu {
                path: Some(more_path.clone()),
                action: more_actions.get(picked).copied().flatten(),
            });
        let body = body.push(container(more).width(MORE_WIDTH));

        let press = HoverTrack::new(
            mouse_area(body)
                .interaction(mouse::Interaction::Pointer)
                .on_press(Message::Press(row.path.clone())),
            hovered,
            Message::Hover {
                path: row.path.clone(),
                hovered: true,
            },
            Message::Hover {
                path: row.path.clone(),
                hovered: false,
            },
        );
        let lit = container(press).width(Fill).style(move |_| row_style(selected, hovered, takes));
        let menu_path = row.path.clone();
        ContextMenu::new(lit, row.path.clone())
            .items(items)
            .on_select(move |picked| Message::Menu {
                path: Some(menu_path.clone()),
                action: actions.get(picked).copied().flatten(),
            })
            .into()
    }

    /// The label that follows the pointer during a drag: the row's name, or how many rows travel.
    fn drag_label(&self, paths: &[String], at: Point) -> Element<'_, Message> {
        let text = match paths {
            [path] => self.row(path).map_or_else(|| path.clone(), |row| row.name.clone()),
            _ => lang::t_count("instance.files.dragging", paths.len()),
        };
        let chip = container(Text::new(text).size(12).font(fonts::Inter::SEMI_BOLD).color(color::text_headline()))
            .padding(Padding::new(0.0).left(10).right(10).top(6).bottom(6))
            .style(|_| container::Style {
                background: Some(Background::Color(color::surface_bg3())),
                border: Border {
                    radius: 8.0.into(),
                    width: 1.0,
                    color: color::primary_glow(),
                },
                ..container::Style::default()
            });
        container(chip)
            .width(Fill)
            .height(Fill)
            .padding(Padding::new(0.0).left((at.x + LABEL_OFFSET).max(0.0)).top((at.y + LABEL_OFFSET).max(0.0)))
            .into()
    }
}

/// The folder a row sits in, as a search shows it: the instance root, or the folder's path.
fn location_of(row: &Row) -> String {
    let parent = drag::parent(&row.path);
    if parent.is_empty() {
        lang::t("instance.files.root").to_string()
    } else {
        parent.to_string()
    }
}

/// A toggle of the find bar: an icon button whose name shows in a tooltip. While it is on it is tinted and ringed in the accent,
/// and otherwise it is the ghost look of the other icon buttons in the path bar.
fn search_toggle<'a>(icon: &'static str, label: &'static str, active: bool, press: Message) -> Element<'a, Message> {
    let style = if active {
        icon_button::IconButtonStyle {
            foreground: color::primary(),
            foreground_hover: color::primary(),
            foreground_pressed: color::primary(),
            background: color::primary_tint(),
            background_hover: color::primary_tint(),
            background_pressed: color::primary_tint(),
            border: color::primary_glow(),
            border_hover: color::primary_glow(),
            foreground_disabled: color::text_disabled(),
            background_disabled: Color::TRANSPARENT,
        }
    } else {
        icon_button::ghost_button_style()
    };
    hint(IconButton::new(icon).size(30).icon_size(14.0).style(style).on_press(press), label)
}

/// A file's size for its row. A file under a kilobyte is given in bytes, since most configs are that small and
/// `0.0 KB` says nothing.
fn size_label(bytes: u64) -> String {
    if bytes < 1024 { format!("{bytes} B") } else { format_bytes(bytes) }
}

/// A line in place of the rows: why there are none.
fn status<'a>(text: String) -> Element<'a, Message> {
    container(Text::new(text).size(12).font(fonts::Inter::REGULAR).color(color::text_muted()))
        .width(Fill)
        .padding(Padding::new(18.0))
        .into()
}

/// A right-aligned cell of a row, in the mono face the sizes and times are set in.
fn cell<'a>(text: String, width: f32, ink: Color) -> Element<'a, Message> {
    container(
        Text::new(text)
            .size(12)
            .font(fonts::JetBrainsMono::REGULAR)
            .color(ink)
            .wrapping(iced::widget::text::Wrapping::None),
    )
    .width(width)
    .clip(true)
    .into()
}

/// A line above the list that says something about the whole tab, with a glyph.
fn notice<'a>(glyph: &'static str, text: &'static str) -> Element<'a, Message> {
    container(
        row![
            utility::icon(glyph, 14.0, Some((color::amber(), color::amber()))),
            Text::new(text).size(12).font(fonts::Inter::MEDIUM).color(color::text_default()),
        ]
        .spacing(10)
        .align_y(alignment::Vertical::Center),
    )
    .width(Fill)
    .padding(Padding::new(0.0).left(14).right(14).top(10).bottom(10))
    .style(|_| container::Style {
        background: Some(Background::Color(color::amber_tint())),
        border: Border {
            radius: 12.0.into(),
            ..Border::default()
        },
        ..container::Style::default()
    })
    .into()
}

/// The cover shown while a file is dragged over the window: where it will land.
fn drop_overlay<'a>() -> Element<'a, Message> {
    let message = column![
        svg(svg::Handle::from_memory(Icon::lucide().upload().as_bytes()))
            .width(28.0)
            .height(28.0)
            .style(|_, _| svg::Style {
                color: Some(color::primary()),
            }),
        Text::new(lang::t("instance.files.drop"))
            .size(14)
            .font(fonts::Inter::SEMI_BOLD)
            .color(color::text_headline()),
    ]
    .spacing(12)
    .align_x(alignment::Horizontal::Center);
    container(message)
        .width(Fill)
        .height(Fill)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .style(|_| container::Style {
            background: Some(Background::Color(rgb!(8, 12, 10, 82%))),
            border: Border {
                radius: 18.0.into(),
                width: 2.0,
                color: color::primary_glow(),
            },
            ..container::Style::default()
        })
        .into()
}

/// The fill of a row: the selection's tint, a faint wash under the pointer, and a ring while it can take a drop.
fn row_style(selected: bool, hovered: bool, takes: bool) -> container::Style {
    let background = if selected {
        Some(Background::Color(color::primary_tint()))
    } else if hovered {
        Some(Background::Color(rgb!(255, 255, 255, 4%)))
    } else {
        None
    };
    container::Style {
        background,
        ..target_ring(takes, 8.0)
    }
}

/// The ring around a place that can take the dragged rows, with corners of `radius`. A place that cannot has none.
fn target_ring(takes: bool, radius: f32) -> container::Style {
    container::Style {
        border: Border {
            radius: radius.into(),
            width: if takes { 1.0 } else { 0.0 },
            color: color::primary(),
        },
        ..container::Style::default()
    }
}

#[cfg(test)]
#[path = "../../../../../tests/app/pages/instance/browser/view.rs"]
mod tests;
