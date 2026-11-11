//! The export dialog: its state, the rules its selection follows, and the modal that shows it. The dialog holds the
//! package's name, version, description and format, and the folder tree the user picks the files from. Its
//! messages are handled in `mod.rs`, which also turns a confirmed dialog into a job.

use std::path::PathBuf;

use aurora_leaf::Icon;
use iced::widget::{Column, Space, Text, button, column, container, row, svg, text_editor, text_input};
use iced::{Background, Border, Element, Fill, Length, Padding, Theme, alignment};
use iced_color_extension::rgb;
use lodestone_export::format::{Format, Metadata};
use lodestone_export::plan::{CheckState, Selection};

use super::job::Request;
use super::tree::{self, Column as SortColumn, Row, Tree};
use super::{FORMATS, Message, format_label};
use crate::app::components::button::{TextButton, Variant};
use crate::app::components::checkbox::{self, checkbox};
use crate::app::components::icon_button::{self, IconButton};
use crate::app::components::modal::Modal;
use crate::app::components::{Select, input, scrollbar};
use crate::app::lang;
use crate::app::pages::instance::format::format_bytes;
use crate::app::pages::library::instance::Instance;
use crate::app::theme::{color, fonts};
use iced::Task;

/// The card's width.
const WIDTH: f32 = 740.0;
/// The height of the description box.
const DESCRIPTION_HEIGHT: f32 = 88.0;
/// The height of the folder listing, which scrolls inside the card.
const TREE_HEIGHT: f32 = 300.0;
/// The most rows the listing draws. A folder with more still exports in full; the rest are only not drawn.
const MAX_ROWS: usize = 400;
/// The widths of the listing's size and modified columns, and of its chevron column.
const SIZE_WIDTH: f32 = 96.0;
const MODIFIED_WIDTH: f32 = 150.0;
const CHEVRON_WIDTH: f32 = 28.0;
/// The width of the version field, beside the name field.
const VERSION_WIDTH: f32 = 180.0;

/// Whether the package's mods link to the platforms (the default) or are bundled into a portable launcher.
pub const MODS_ONLINE: usize = 0;
pub const MODS_OFFLINE: usize = 1;

/// The export dialog's state.
pub struct Dialog {
    /// The instance's id, which its mod records are kept under.
    pub(super) instance: String,
    /// The instance's name, for the alerts.
    pub(super) instance_name: String,
    /// The instance's folder.
    pub(super) folder: PathBuf,
    /// The Minecraft version and loader the instance runs, which the package's manifests name.
    pub(super) minecraft: String,
    pub(super) loader: String,
    pub(super) loader_version: Option<String>,
    /// Whether the instance is locked to its modpack, which a Lodestone package keeps.
    pub(super) locked: bool,
    pub(super) format: Format,
    pub(super) name: String,
    pub(super) version: String,
    pub(super) description: text_editor::Content,
    /// Whether a portable launcher bundles every mod, rather than linking the ones it can.
    pub(super) offline: bool,
    pub(super) selection: Selection,
    pub(super) tree: Tree,
}

impl Dialog {
    /// The dialog for `instance`, whose folder is `folder`. Everything but the default top-level folders is taken.
    pub fn new(instance: &Instance, folder: PathBuf) -> Self {
        Self {
            instance: instance.id.clone(),
            instance_name: instance.name.clone(),
            folder,
            minecraft: instance.mc_version.clone(),
            loader: instance.loader.to_db().to_string(),
            loader_version: instance.loader_version.clone(),
            locked: instance.locked,
            format: Format::Mrpack,
            name: instance.name.clone(),
            version: "1.0.0".to_string(),
            description: text_editor::Content::new(),
            offline: false,
            selection: Selection::new_export(),
            tree: Tree::default(),
        }
    }

    /// Whether Export can be pressed: a name, a version, and at least one taken entry at the top of the folder.
    pub fn ready(&self) -> bool {
        !self.name.trim().is_empty() && !self.version.trim().is_empty() && self.has_selection()
    }

    /// The metadata the package says about the instance, as the fields now read.
    pub fn metadata(&self) -> Metadata {
        Metadata {
            name: self.name.trim().to_string(),
            version: self.version.trim().to_string(),
            description: self.description.text().trim().to_string(),
            minecraft: self.minecraft.clone(),
            loader: self.loader.clone(),
            loader_version: self.loader_version.clone(),
            locked: self.locked,
        }
    }

    /// The export request for the dialog as it stands, writing to `target`. A portable launcher is copied from the
    /// launcher's own file, which is read here.
    pub fn request(&self, target: PathBuf) -> Result<Request, String> {
        let executable = if self.format == Format::Portable {
            std::env::current_exe().map_err(|error| error.to_string())?
        } else {
            PathBuf::new()
        };
        Ok(Request {
            instance: self.instance.clone(),
            folder: self.folder.clone(),
            selection: self.selection.clone(),
            format: self.format,
            metadata: self.metadata(),
            offline: self.offline,
            target,
            executable,
        })
    }

    /// The file name the save dialog offers: the name and version, with the format's extension.
    pub fn file_name(&self) -> String {
        let stem = super::job::file_safe(&format!("{}-{}", self.name.trim(), self.version.trim()));
        match super::extension(self.format) {
            "" => stem,
            extension => format!("{stem}.{extension}"),
        }
    }

    /// Applies a field's message. Export and close are handled by the root.
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::NameChanged(name) => self.name = name,
            Message::VersionChanged(version) => self.version = version,
            Message::DescriptionEdited(action) => self.description.perform(action),
            Message::FormatSelected(index) => {
                if let Some(format) = FORMATS.get(index) {
                    self.format = *format;
                }
            }
            Message::ModsSelected(index) => self.offline = index == MODS_OFFLINE,
            Message::Toggle(path) => self.toggle(&path),
            Message::ToggleAll => self.toggle_all(),
            Message::Open(folder) => return self.open(folder),
            Message::Sort(column) => self.tree.sort_by(column),
            Message::Listed { folder, result } => self.tree.store(folder, result),
            Message::Close | Message::Submit | Message::PathChosen(_) => {}
        }
        Task::none()
    }

    /// Makes `folder` the folder on screen, reading its listing when it has not been read.
    pub fn open(&mut self, folder: String) -> Task<Message> {
        if !self.tree.open(folder.clone()) {
            return Task::none();
        }
        let root = self.folder.clone();
        let reading = folder.clone();
        Task::perform(tree::read_folder(root, reading), move |result| Message::Listed { folder, result })
    }

    /// The folder listing's read, when it is needed for the dialog to open.
    pub fn listing(&mut self) -> Task<Message> {
        self.open(String::new())
    }

    /// Turns the row at `path` on or off, the way a click on its checkbox does.
    fn toggle(&mut self, path: &str) {
        let included = self.selection.state(path) != CheckState::Checked;
        let tree = &self.tree;
        self.selection.set(path, included, |folder| tree.children(folder));
    }

    /// Turns every row on screen on or off, the way the header's checkbox does.
    fn toggle_all(&mut self) {
        let included = self.header_state() != CheckState::Checked;
        for path in self.tree.screen_paths() {
            let tree = &self.tree;
            self.selection.set(&path, included, |folder| tree.children(folder));
        }
    }

    /// The header's state: checked when every row on screen is, unchecked when none is, and partly otherwise.
    fn header_state(&self) -> CheckState {
        let states: Vec<CheckState> = self.tree.screen_paths().iter().map(|path| self.selection.state(path)).collect();
        if states.is_empty() || states.iter().all(|state| *state == CheckState::Unchecked) {
            CheckState::Unchecked
        } else if states.iter().all(|state| *state == CheckState::Checked) {
            CheckState::Checked
        } else {
            CheckState::Partial
        }
    }

    /// Whether any entry at the top of the instance folder is taken.
    fn has_selection(&self) -> bool {
        self.tree
            .rows_of("")
            .is_some_and(|rows| rows.iter().any(|row| self.selection.state(&row.path) != CheckState::Unchecked))
    }

    /// The modal the dialog shows.
    pub fn view(&self) -> Element<'_, Message> {
        let mut body = column![self.format_field(), self.name_row(), self.description_field()].spacing(20);
        if self.format == Format::Portable {
            body = body.push(self.mods_field());
        }
        body = body.push(self.tree_field());
        Modal::new(lang::t("export.title"), body)
            .subtitle(lang::t("export.subtitle"))
            .icon(Icon::lucide().package(), color::primary(), color::primary_tint())
            .footer(self.footer())
            .on_close(Message::Close)
            .close_on_scrim(true)
            .width(WIDTH)
            .into()
    }

    /// The format picker, which the rest of the dialog follows.
    fn format_field(&self) -> Element<'_, Message> {
        let labels: Vec<String> = FORMATS.iter().map(|format| format_label(*format).to_owned()).collect();
        let selected = FORMATS.iter().position(|format| *format == self.format);
        let select = Select::new(labels, selected).on_select(Message::FormatSelected).width(Fill);
        field(lang::t("export.format.label"), select.into())
    }

    /// The name, and the version beside it.
    fn name_row(&self) -> Element<'_, Message> {
        let name = field(
            lang::t("export.name.label"),
            text_input(lang::t("export.name.placeholder"), &self.name)
                .on_input(Message::NameChanged)
                .size(14)
                .padding(FIELD_PADDING)
                .style(input::style)
                .width(Fill)
                .into(),
        );
        let version = container(field(
            lang::t("export.version.label"),
            text_input(lang::t("export.version.placeholder"), &self.version)
                .on_input(Message::VersionChanged)
                .size(14)
                .padding(FIELD_PADDING)
                .style(input::style)
                .width(Fill)
                .into(),
        ))
        .width(VERSION_WIDTH);
        row![name, version].spacing(16).into()
    }

    /// The description, a multi-line box.
    fn description_field(&self) -> Element<'_, Message> {
        let editor = text_editor(&self.description)
            .placeholder(lang::t("export.description.placeholder"))
            .on_action(Message::DescriptionEdited)
            .padding(FIELD_PADDING)
            .height(Length::Fixed(DESCRIPTION_HEIGHT))
            .style(description_style);
        field(lang::t("export.description.label"), editor.into())
    }

    /// Whether a portable launcher downloads its mods on first run, or carries them.
    fn mods_field(&self) -> Element<'_, Message> {
        let labels = vec![lang::t("export.mods.online").to_owned(), lang::t("export.mods.offline").to_owned()];
        let selected = if self.offline { MODS_OFFLINE } else { MODS_ONLINE };
        let select = Select::new(labels, Some(selected)).on_select(Message::ModsSelected).width(Fill);
        column![
            field(lang::t("export.mods.label"), select.into()),
            Text::new(lang::t("export.mods.hint"))
                .size(11)
                .font(fonts::JetBrainsMono::REGULAR)
                .color(color::text_muted()),
        ]
        .spacing(6)
        .width(Fill)
        .into()
    }

    /// The folder listing: where the user is, the header, and the rows.
    fn tree_field(&self) -> Element<'_, Message> {
        let list = container(scrollbar::scroll_panel(self.rows_column()).width(Fill))
            .height(Length::Fixed(TREE_HEIGHT))
            .width(Fill);
        column![label(lang::t("export.files.label")), self.breadcrumb(), self.header_row(), list]
            .spacing(8)
            .width(Fill)
            .into()
    }

    /// The instance folder and the folders down to the one on screen, each one a way back.
    fn breadcrumb(&self) -> Element<'_, Message> {
        let mut crumbs = row![
            TextButton::ghost(lang::t("export.files.root"))
                .on_press(Message::Open(String::new()))
                .padding(Padding::new(0.0).left(6).right(6).top(3).bottom(3))
        ]
        .spacing(2)
        .align_y(alignment::Vertical::Center);
        let mut folder = String::new();
        for segment in self.tree.folder.split('/').filter(|segment| !segment.is_empty()) {
            folder = if folder.is_empty() {
                segment.to_string()
            } else {
                format!("{folder}/{segment}")
            };
            crumbs = crumbs.push(Text::new("/").size(12).font(fonts::JetBrainsMono::REGULAR).color(color::text_muted()));
            crumbs = crumbs.push(
                TextButton::ghost(segment.to_string())
                    .on_press(Message::Open(folder.clone()))
                    .padding(Padding::new(0.0).left(6).right(6).top(3).bottom(3)),
            );
        }
        crumbs.into()
    }

    /// The column headings: the header's checkbox, then the sortable columns.
    fn header_row(&self) -> Element<'_, Message> {
        row![
            checkbox(checkbox_state(self.header_state()), Message::ToggleAll),
            sort_heading(lang::t("export.files.name"), SortColumn::Name, &self.tree).width(Fill),
            sort_heading(lang::t("export.files.size"), SortColumn::Size, &self.tree).width(SIZE_WIDTH),
            sort_heading(lang::t("export.files.modified"), SortColumn::Modified, &self.tree).width(MODIFIED_WIDTH),
            Space::new().width(CHEVRON_WIDTH),
        ]
        .spacing(10)
        .align_y(alignment::Vertical::Center)
        .into()
    }

    /// The rows of the folder on screen, or a line saying why there are none.
    fn rows_column(&self) -> Column<'_, Message> {
        let folder = self.tree.folder.as_str();
        if let Some(error) = self.tree.error_of(folder) {
            return Column::new().push(status_line(&lang::t_with("export.files.failed", &[("error", error)])));
        }
        let Some(rows) = self.tree.sorted_rows() else {
            return Column::new().push(status_line(lang::t("export.files.reading")));
        };
        if rows.is_empty() {
            return Column::new().push(status_line(lang::t("export.files.empty")));
        }
        let hidden = rows.len().saturating_sub(MAX_ROWS);
        let mut list = Column::new().spacing(2).width(Fill);
        for row in rows.into_iter().take(MAX_ROWS) {
            list = list.push(self.row(row));
        }
        if hidden > 0 {
            list = list.push(status_line(&lang::t_with("export.files.more", &[("count", hidden.to_string().as_str())])));
        }
        list
    }

    /// One row: its checkbox, its icon and name, its size and modified time, and a chevron that opens a folder.
    fn row(&self, row: &Row) -> Element<'_, Message> {
        let state = checkbox_state(self.selection.state(&row.path));
        let glyph = if row.is_folder { Icon::lucide().folder() } else { Icon::lucide().file() };
        let icon = svg(svg::Handle::from_memory(glyph.as_bytes()))
            .width(14.0)
            .height(14.0)
            .style(|_, _| svg::Style {
                color: Some(color::text_muted()),
            });
        let name: Element<'_, Message> = if row.is_folder {
            TextButton::ghost(row.name.clone())
                .on_press(Message::Open(row.path.clone()))
                .padding(Padding::new(0.0).left(4).right(4).top(2).bottom(2))
                .into()
        } else {
            Text::new(row.name.clone())
                .size(13)
                .font(fonts::Inter::REGULAR)
                .color(color::text_default())
                .into()
        };
        let size = if row.is_folder { String::new() } else { format_bytes(row.size) };
        let modified = row
            .modified
            .map(|time| time.format("%-m/%-d/%y, %-I:%M %p").to_string())
            .unwrap_or_default();
        let chevron: Element<'_, Message> = if row.is_folder {
            IconButton::new(Icon::lucide().chevron_right())
                .size(24)
                .icon_size(12.0)
                .style(icon_button::ghost_button_style())
                .on_press(Message::Open(row.path.clone()))
                .into()
        } else {
            Space::new().width(24.0).into()
        };
        row![
            checkbox(state, Message::Toggle(row.path.clone())),
            icon,
            container(name).width(Fill),
            cell(size).width(SIZE_WIDTH),
            cell(modified).width(MODIFIED_WIDTH),
            container(chevron).width(CHEVRON_WIDTH),
        ]
        .spacing(10)
        .align_y(alignment::Vertical::Center)
        .into()
    }

    /// The footer: Cancel, and Export, which opens the save dialog once the dialog is ready.
    fn footer(&self) -> Element<'_, Message> {
        let mut submit = TextButton::content(
            Text::new(lang::t("export.submit"))
                .size(13)
                .font(fonts::Inter::BOLD)
                .color(color::primary_on()),
        )
        .variant(Variant::Primary)
        .padding(Padding::new(0.0).left(18).right(18).top(9).bottom(9));
        if self.ready() {
            submit = submit.on_press(Message::Submit);
        }
        row![
            Space::new().width(Fill),
            TextButton::content(
                Text::new(lang::t("common.cancel"))
                    .size(13)
                    .font(fonts::Inter::SEMI_BOLD)
                    .color(color::text_default()),
            )
            .variant(Variant::Ghost)
            .padding(Padding::new(0.0).left(18).right(18).top(9).bottom(9))
            .on_press(Message::Close),
            submit,
        ]
        .spacing(10)
        .align_y(alignment::Vertical::Center)
        .into()
    }
}

/// The checkbox state a selection state shows.
fn checkbox_state(state: CheckState) -> checkbox::State {
    match state {
        CheckState::Checked => checkbox::State::Checked,
        CheckState::Partial => checkbox::State::Partial,
        CheckState::Unchecked => checkbox::State::Unchecked,
    }
}

/// A column heading. It shows the arrow of the sort it is, and sorts by its column when pressed.
fn sort_heading(text: &'static str, column: SortColumn, tree: &Tree) -> button::Button<'static, Message> {
    let active = tree.sort.column == column;
    let arrow = match (active, tree.sort.descending) {
        (false, _) => None,
        (true, false) => Some(Icon::lucide().arrow_up()),
        (true, true) => Some(Icon::lucide().chevron_down()),
    };
    let content = row![
        Text::new(text).size(11).font(fonts::JetBrainsMono::REGULAR).color(color::text_muted()),
        arrow.map(|markup| {
            svg(svg::Handle::from_memory(markup.as_bytes()))
                .width(10.0)
                .height(10.0)
                .style(|_, _| svg::Style {
                    color: Some(color::text_muted()),
                })
        })
    ]
    .spacing(4)
    .align_y(alignment::Vertical::Center);
    button(content)
        .padding(Padding::new(0.0))
        .style(|_, _| button::Style {
            background: Some(Background::Color(rgb!(255, 255, 255, 0%))),
            ..button::Style::default()
        })
        .on_press(Message::Sort(column))
}

/// A plain cell of the listing, in the mono type the columns use.
fn cell(text: String) -> container::Container<'static, Message> {
    container(Text::new(text).size(11).font(fonts::JetBrainsMono::REGULAR).color(color::text_muted()))
}

/// A line of muted text where the listing has no rows to show.
fn status_line(text: &str) -> Element<'static, Message> {
    Text::new(text.to_owned())
        .size(12)
        .font(fonts::Inter::REGULAR)
        .color(color::text_muted())
        .into()
}

/// A labelled control.
fn field<'a>(name: &'static str, control: Element<'a, Message>) -> Element<'a, Message> {
    column![label(name), control].spacing(6).width(Fill).into()
}

/// A small uppercase section label.
fn label(text: &'static str) -> Element<'static, Message> {
    Text::new(text)
        .size(11)
        .font(fonts::JetBrainsMono::REGULAR)
        .color(color::text_muted())
        .into()
}

/// The description box's style: the input's fill and hairline, with the green ring while it has focus.
fn description_style(_: &Theme, status: text_editor::Status) -> text_editor::Style {
    let focused = matches!(status, text_editor::Status::Focused { .. });
    text_editor::Style {
        background: Background::Color(rgb!(255, 255, 255, 4%)),
        border: Border {
            radius: 12.0.into(),
            width: 1.0,
            color: if focused {
                rgb!(34, 255, 132, 50%)
            } else {
                color::surface_line_strong()
            },
        },
        placeholder: color::text_placeholder(),
        value: color::text_default(),
        selection: input::selection_color(),
    }
}

const FIELD_PADDING: Padding = Padding {
    top: 11.0,
    right: 14.0,
    bottom: 11.0,
    left: 14.0,
};

#[cfg(test)]
#[path = "../../../tests/app/export/dialog.rs"]
mod tests;
