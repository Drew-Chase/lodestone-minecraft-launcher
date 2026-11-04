//! Mods tab — the mod jars in the instance's `mods/` folder as one divided list, with a text filter, a switch and a menu
//! per mod. A mod that came from a platform opens its resource page when pressed, and one that did not opens the search
//! that links it to a project. The list builds only the rows on screen, so a folder of thousands of mods scrolls as
//! smoothly as a short one.
//!
//! The folder is read by [`super::files`]. The root reads each mod's recorded source, checks the mods against their
//! platforms, and carries out the actions the menu and the dialogs ask for. The tab applies what comes back.

mod dialog;
mod list;
mod menu;

use std::collections::{HashMap, HashSet};

use iced::widget::scrollable::AbsoluteOffset;
use iced::widget::{Id, Text, column, container, operation, responsive, text_input};
use iced::{Background, Border, Element, Fill, Padding, Task, alignment};
use iced_color_extension::rgb;
use lodestone_platforms::ContentVersion;

pub use dialog::{Associate, Dependencies, Dialog, Line, Message as DialogMessage, Picked, Switch};
pub use menu::Action;

use super::GUTTER;
use super::files::ModFile;
use crate::app::components::button::Variant;
use crate::app::components::hint::hint;
use crate::app::components::input;
use crate::app::components::panel::panel;
use crate::app::components::{TextButton, scrollbar};
use crate::app::lang;
use crate::app::shared_library::ModSource;
use crate::app::theme::{color, fonts};
use aurora_leaf::Icon;

/// The active filter: a case-insensitive match on name or author, and the updates chip, which narrows the list to the
/// mods the platforms have a newer release for.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModFilter {
    pub query: String,
    pub updates_only: bool,
}

impl ModFilter {
    /// Whether `row` passes the filter.
    pub fn matches(&self, row: &Row) -> bool {
        let entry = &row.file;
        (!self.updates_only || row.update.is_some())
            && (self.query.is_empty()
                || contains_ignore_case(&entry.name, &self.query)
                || entry.author.as_deref().is_some_and(|author| contains_ignore_case(author, &self.query)))
    }
}

fn contains_ignore_case(haystack: &str, needle: &str) -> bool {
    haystack.to_lowercase().contains(&needle.to_lowercase())
}

/// One mod in the list: its jar, the platform source it was installed from when that is known, and the newer release
/// the platform has when there is one.
#[derive(Debug)]
pub struct Row {
    pub file: ModFile,
    pub source: Option<ModSource>,
    pub update: Option<ContentVersion>,
}

/// The Mods tab's state.
#[derive(Debug, Default)]
pub struct State {
    rows: Vec<Row>,
    filter: ModFilter,
    /// Indices into `rows` that pass `filter`, kept current by `refilter`.
    visible: Vec<usize>,
    /// Where the list is scrolled to, from the scroll it reports.
    scroll_y: f32,
    /// The recorded source of each mod, by file name. Kept apart from the rows, so a rescan joins them again.
    sources: HashMap<String, ModSource>,
    /// The newer release of each mod that has one, by file name.
    updates: HashMap<String, ContentVersion>,
    /// The files whose action is in flight. Their menus show nothing until it lands.
    busy: HashSet<String>,
    /// The dialog on top of the tab, when one is open.
    dialog: Option<Dialog>,
    /// Whether the platforms have been asked about these mods since the folder was read.
    checks_started: bool,
}

/// Messages the Mods tab handles, or passes to the root.
#[derive(Debug, Clone)]
pub enum Message {
    /// The filter text changed.
    Query(String),
    /// The updates chip was pressed. `true` narrows the list to the mods with a newer release.
    Updates(bool),
    /// A mod's switch was flipped.
    Toggle { index: usize, enabled: bool },
    /// Add Mod was pressed. The root opens Discover on the instance's mods.
    Add,
    /// The list scrolled to this offset.
    Scrolled(f32),
    /// A mod's row was pressed, by file name. The root opens its resource page, or the search that links it.
    Open(String),
    /// A row of a mod's menu was picked, by file name. A separator sends `None`. The root carries the action out.
    Menu { file_name: String, action: Option<Action> },
    /// A message of the dialog on top of the tab.
    Dialog(DialogMessage),
}

impl State {
    /// Replaces the mods with the ones the folder holds, unfiltered. Each is joined to the source and the update
    /// known for it, and what is known of a jar that is gone is forgotten.
    pub fn set_mods(&mut self, mods: Vec<ModFile>) {
        let present: HashSet<String> = mods.iter().map(|file| file.file_name.clone()).collect();
        self.sources.retain(|file_name, _| present.contains(file_name));
        self.updates.retain(|file_name, _| present.contains(file_name));
        self.rows = mods.into_iter().map(|file| self.joined(file)).collect();
        self.refilter();
    }

    /// Replaces the recorded sources with the ones the database holds for the instance.
    pub fn apply_sources(&mut self, records: Vec<ModSource>) {
        self.sources = records.into_iter().map(|record| (record.file_name.clone(), record)).collect();
        for row in &mut self.rows {
            row.source = self.sources.get(&row.file.file_name).cloned();
        }
    }

    /// Records the source of one mod, as its record was just written.
    pub fn apply_source(&mut self, record: ModSource) {
        if let Some(row) = self.rows.iter_mut().find(|row| row.file.file_name == record.file_name) {
            row.source = Some(record.clone());
        }
        self.sources.insert(record.file_name.clone(), record);
    }

    /// Forgets the source and the update of a mod whose jar is gone or was replaced.
    pub fn forget_source(&mut self, file_name: &str) {
        self.sources.remove(file_name);
        self.updates.remove(file_name);
        if let Some(row) = self.rows.iter_mut().find(|row| row.file.file_name == file_name) {
            row.source = None;
            row.update = None;
        }
    }

    /// Applies the outcome of a check: the sources recorded for jars the platforms named, and the update each checked
    /// mod has. A checked mod with no entry in `updates` has no update.
    pub fn apply_checks(&mut self, identified: Vec<ModSource>, mut updates: HashMap<String, ContentVersion>, checked: &[String]) {
        for record in identified {
            self.sources.insert(record.file_name.clone(), record);
        }
        for file_name in checked {
            match updates.remove(file_name) {
                Some(release) => self.updates.insert(file_name.clone(), release),
                None => self.updates.remove(file_name),
            };
        }
        for row in &mut self.rows {
            row.source = self.sources.get(&row.file.file_name).cloned();
            row.update = self.updates.get(&row.file.file_name).cloned();
        }
    }

    /// Marks a mod's action as in flight, or done.
    pub fn set_busy(&mut self, file_name: &str, busy: bool) {
        if busy {
            self.busy.insert(file_name.to_string());
        } else {
            self.busy.remove(file_name);
        }
    }

    /// The row of the mod `file_name`, if the folder holds it.
    pub fn row(&self, file_name: &str) -> Option<&Row> {
        self.rows.iter().find(|row| row.file.file_name == file_name)
    }

    /// The dialog on top of the tab, when one is open.
    pub fn dialog(&self) -> Option<&Dialog> {
        self.dialog.as_ref()
    }

    /// The dialog on top of the tab, for the root to fill in.
    pub fn dialog_mut(&mut self) -> Option<&mut Dialog> {
        self.dialog.as_mut()
    }

    /// Opens `dialog` on top of the tab, in place of any open one.
    pub fn open_dialog(&mut self, dialog: Dialog) {
        self.dialog = Some(dialog);
    }

    /// Closes the dialog on top of the tab, if one is open.
    pub fn close_dialog(&mut self) {
        self.dialog = None;
    }

    /// Whether the folder holds any mod.
    pub fn has_mods(&self) -> bool {
        !self.rows.is_empty()
    }

    /// The file name of every mod the folder holds.
    pub fn file_names(&self) -> Vec<String> {
        self.rows.iter().map(|row| row.file.file_name.clone()).collect()
    }

    /// Whether the platforms have been asked about these mods since the folder was read.
    pub fn checks_started(&self) -> bool {
        self.checks_started
    }

    /// Records that the platforms are being asked about these mods.
    pub fn start_checks(&mut self) {
        self.checks_started = true;
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Query(query) => {
                self.filter.query = query;
                self.filter_changed()
            }
            Message::Updates(updates_only) => {
                self.filter.updates_only = updates_only;
                self.filter_changed()
            }
            Message::Toggle { index, enabled } => {
                if let Some(row) = self.rows.get_mut(index) {
                    row.file.enabled = enabled;
                }
                Task::none()
            }
            Message::Scrolled(offset) => {
                self.scroll_y = offset.max(0.0);
                Task::none()
            }
            // The root answers these: a press opens a mod's page or its search, a menu row runs an action, and Add
            // opens Discover.
            Message::Add | Message::Open(_) | Message::Menu { .. } => Task::none(),
            Message::Dialog(message) => {
                match message {
                    DialogMessage::Close => self.dialog = None,
                    other => {
                        if let Some(dialog) = self.dialog.as_mut() {
                            dialog.local(&other);
                        }
                    }
                }
                Task::none()
            }
        }
    }

    /// The row for `file`, joined to what is known of it.
    fn joined(&self, file: ModFile) -> Row {
        Row {
            source: self.sources.get(&file.file_name).cloned(),
            update: self.updates.get(&file.file_name).cloned(),
            file,
        }
    }

    /// Applies a change of the filter: the list is filtered again, and scrolled back to its top.
    fn filter_changed(&mut self) -> Task<Message> {
        self.refilter();
        self.scroll_y = 0.0;
        operation::scroll_to(Id::new(list::SCROLL_ID), AbsoluteOffset { x: None, y: Some(0.0) })
    }

    fn refilter(&mut self) {
        let visible = self
            .rows
            .iter()
            .enumerate()
            .filter(|(_, row)| self.filter.matches(row))
            .map(|(index, _)| index)
            .collect();
        self.visible = visible;
    }

    /// How many mods have a newer release the platforms know of.
    fn update_count(&self) -> usize {
        self.rows.iter().filter(|row| row.update.is_some()).count()
    }

    /// The tab: the filter bar, which stays at the top while the list scrolls beneath it, then the list. The list's
    /// height is the room the tab leaves for it, which the list needs to know to build the rows on screen.
    pub fn view(&self, takes_mods: bool) -> Element<'_, Message> {
        let toolbar = container(self.toolbar(takes_mods))
            .width(Fill)
            .padding(Padding::new(0.0).left(GUTTER).right(GUTTER));
        let list = responsive(move |size| self.list_area(size.height));
        column![toolbar, list].spacing(16).width(Fill).height(Fill).into()
    }

    /// The dialog on top of the tab, when one is open. The root stacks it above the window, as it stacks the viewer.
    pub fn dialog_view(&self) -> Option<Element<'_, Message>> {
        self.dialog.as_ref().map(|dialog| dialog.view().map(Message::Dialog))
    }

    /// The list in its scroll. The list builds the rows its height and offset reveal, and the scroll reports its offset.
    fn list_area(&self, height: f32) -> Element<'_, Message> {
        let content: Element<'_, Message> = if self.visible.is_empty() {
            let message = if self.rows.is_empty() {
                lang::t("instance.mods.empty")
            } else {
                lang::t("instance.mods.no_match")
            };
            panel(Text::new(message).size(12).font(fonts::Inter::REGULAR).color(color::text_muted()))
        } else {
            list::view(self, height)
        };
        // The list keeps 40px clear below it, as the page does, so its last row scrolls clear of the bottom edge.
        let padded = container(content)
            .width(Fill)
            .padding(Padding::new(0.0).left(GUTTER).right(GUTTER).bottom(40.0));
        scrollbar::scroll(padded)
            .id(list::SCROLL_ID)
            .on_scroll(|viewport| Message::Scrolled(viewport.absolute_offset().y))
            .width(Fill)
            .height(Fill)
            .into()
    }

    fn toolbar(&self, takes_mods: bool) -> Element<'_, Message> {
        let add = TextButton::labeled_icon(Icon::lucide().plus(), lang::t("instance.mods.add"), Variant::Primary);
        let add = if takes_mods { add.on_press(Message::Add) } else { add.disabled(true) };
        let add_hint = if takes_mods {
            lang::t("instance.mods.add_hint")
        } else {
            lang::t("instance.mods.add_vanilla")
        };
        iced::widget::row![
            text_input(lang::t("instance.mods.filter_placeholder"), &self.filter.query)
                .on_input(Message::Query)
                .style(input::style)
                .padding(Padding::new(0.0).left(14).right(14).top(10).bottom(10))
                .size(13)
                .width(Fill),
            updates_chip(self.filter.updates_only, self.update_count()),
            hint(add, add_hint),
        ]
        .spacing(10)
        .align_y(alignment::Vertical::Center)
        .into()
    }
}

/// The chip that narrows the list to the mods with a newer release, with how many there are. It lights while it is on, as
/// the Worlds page's filter chips do.
fn updates_chip(active: bool, count: usize) -> Element<'static, Message> {
    let (ink, weight) = if active {
        (color::text_headline(), fonts::Inter::SEMI_BOLD)
    } else {
        (color::text_muted(), fonts::Inter::MEDIUM)
    };
    let content = iced::widget::row![
        Text::new(lang::t("instance.mods.updates_filter")).size(12).font(weight).color(ink),
        Text::new(count.to_string())
            .size(11)
            .font(fonts::JetBrainsMono::REGULAR)
            .color(color::text_placeholder()),
    ]
    .spacing(8)
    .align_y(alignment::Vertical::Center);
    container(
        TextButton::content(content)
            .padding(Padding::new(0.0).left(12.0).right(12.0).top(7.0).bottom(7.0))
            .on_press(Message::Updates(!active)),
    )
    .style(move |_| container::Style {
        background: active.then_some(Background::Color(rgb!(255, 255, 255, 10%))),
        border: Border {
            radius: 999.0.into(),
            width: if active { 1.0 } else { 0.0 },
            color: color::surface_line_strong(),
        },
        ..container::Style::default()
    })
    .into()
}

#[cfg(test)]
#[path = "../../../../../tests/app/pages/instance/mods/mod.rs"]
mod tests;
