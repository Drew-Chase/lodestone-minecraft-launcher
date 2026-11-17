//! A text editor for the files of an instance: syntax colours, line numbers, undo, and the typing helpers a config
//! file wants (closing brackets and quotes, keeping the indent, indenting by the file's own unit).
//!
//! iced's `text_editor` does the editing. What it lacks is built around it here: the history in [`history`], the key
//! rules in [`keys`], the colours in [`highlight`], the gutter in [`gutter`], and the scrolling, since the editor is
//! laid out at the full size of its text inside a scroll that shows a bar and moves sideways. The owner reads the
//! file, hands the text in as a [`Loaded`], and writes [`Editor::bytes`] back when [`Message::Save`] arrives.

pub mod encoding;
mod gutter;
pub mod highlight;
mod history;
pub mod keys;
pub mod language;
mod view;

use std::sync::Arc;
use std::time::{Instant, SystemTime};

use iced::widget::scrollable::AbsoluteOffset;
use iced::widget::text_editor::{Action, Binding, Content, Cursor, Edit, KeyPress, Motion, Position, Status};
use iced::widget::{Id, operation};
use iced::{Size, Task, keyboard};

use encoding::Encoding;
use history::{History, Kind, Snapshot};
use keys::{Context, Step, Typed};
use language::Language;

/// The id of the scroll the text sits in, so the caret can be kept on screen.
const SCROLL_ID: &str = "editor-scroll";
/// The size of the text, and the height of each of its lines.
const TEXT_SIZE: f32 = 13.0;
const LINE_HEIGHT: f32 = 20.0;
/// The width of one character of the mono face at [`TEXT_SIZE`]: six tenths of its size.
const ADVANCE: f32 = TEXT_SIZE * 0.6;
/// The room around the text inside the editor.
const PADDING: f32 = 12.0;
/// How many columns a tab takes. cosmic-text draws a tab eight columns wide, and iced has no setting for it.
const TAB_COLUMNS: usize = 8;
/// The size the scroll is taken to have before it has reported one.
const ASSUMED_VIEW: Size = Size::new(640.0, 400.0);

/// The largest file the editor opens: 512 KB. The text is shaped whole three times as it opens (as it is read, when it
/// takes the editor's font, and when it is coloured), on the UI thread, which measured about 1.6 s for a 900 KB log. Half
/// that size keeps the wait under a second, and holds every config and crash report a pack is likely to carry.
pub const MAX_BYTES: u64 = 512 << 10;

/// When a file was last written and how long it was, to tell whether it changed on disk since it was read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Stamp {
    pub modified: Option<SystemTime>,
    pub length: u64,
}

/// A file as it was read: its path inside the instance, its text, the encoding the text was in, and its stamp.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Loaded {
    pub path: String,
    pub text: String,
    pub encoding: Encoding,
    pub stamp: Stamp,
}

/// What a key or a button asks of the editor beyond typing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Undo,
    Redo,
    /// Takes one level of indent off the caret's line, or off every selected line.
    Unindent,
    /// Comments the caret's line or the selected lines out, or back in when they all are.
    ToggleComment,
    /// Moves the caret a screenful up (`-1`) or down (`1`).
    Page(isize),
}

/// Messages the editor handles, or hands to its owner.
#[derive(Debug, Clone)]
pub enum Message {
    /// An action of the text editor: typing, a move of the caret, a click.
    Action(Action),
    /// The edits a typing helper makes in place of a key.
    Steps(Vec<Step>),
    Command(Command),
    /// The text scrolled to this offset.
    Scrolled {
        x: f32,
        y: f32,
    },
    /// The scroll the text sits in was laid out at this size.
    Resized(Size),
    /// Save was asked for. The owner writes [`Editor::bytes`] to the file.
    Save,
    /// Revert was asked for. The owner reads the file again.
    Revert,
    /// Close was asked for. The owner closes the editor, after asking about unsaved changes.
    Close,
}

/// An open file.
#[derive(Debug)]
pub struct Editor {
    /// The file's path inside the instance, with `/` separators.
    path: String,
    content: Content,
    language: Language,
    encoding: Encoding,
    /// Whether the file's lines ended in CR LF when it was read. The text is kept with plain line feeds, and the
    /// endings are put back on save.
    crlf: bool,
    /// One level of indent in this file.
    unit: String,
    read_only: bool,
    history: History,
    /// The version of the text on screen, the version last saved, and the last version number handed out.
    revision: u64,
    saved_revision: u64,
    last_revision: u64,
    /// Counts the times the whole text was replaced (an undo, a reload), which makes the highlighter start over.
    replaced: u64,
    /// The width of the longest line seen, in columns. It sizes the editor, so the scroll reaches the line's end.
    longest: usize,
    scroll: (f32, f32),
    /// The size of the scroll the text sits in. Zero until it has been laid out.
    viewport: Size,
    stamp: Stamp,
    /// Whether the file changed on disk while the text here has unsaved changes.
    changed_on_disk: bool,
    /// Whether the last save wrote UTF-8 because the file's own encoding could not hold the text.
    reencoded: bool,
}

impl Editor {
    /// An editor on `loaded`. A `read_only` editor shows the text and lets it be selected and copied, and takes no edit.
    pub fn new(loaded: Loaded, read_only: bool) -> Self {
        let language = Language::of(&loaded.path);
        let crlf = loaded.text.contains("\r\n");
        let text = if crlf { loaded.text.replace("\r\n", "\n") } else { loaded.text };
        let unit = keys::indent_unit(text.lines(), language);
        let longest = text.lines().map(columns).max().unwrap_or(0);
        Self {
            path: loaded.path,
            content: Content::with_text(&text),
            language,
            encoding: loaded.encoding,
            crlf,
            unit,
            read_only,
            history: History::default(),
            revision: 0,
            saved_revision: 0,
            last_revision: 0,
            replaced: 0,
            longest,
            scroll: (0.0, 0.0),
            viewport: Size::ZERO,
            stamp: loaded.stamp,
            changed_on_disk: false,
            reencoded: false,
        }
    }

    /// The path of the open file inside the instance.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Points the editor at the file's new path, after the file was renamed or moved.
    pub fn set_path(&mut self, path: String) {
        self.language = Language::of(&path);
        self.path = path;
        self.replaced += 1;
    }

    /// Whether the text has changes that have not been saved.
    pub fn is_dirty(&self) -> bool {
        self.revision != self.saved_revision
    }

    /// Whether the editor takes no edit.
    pub fn read_only(&self) -> bool {
        self.read_only
    }

    /// Makes the editor read-only, or lets it edit again.
    pub fn set_read_only(&mut self, read_only: bool) {
        self.read_only = read_only;
    }

    /// The stamp of the file as the editor last read or wrote it.
    pub fn stamp(&self) -> Stamp {
        self.stamp
    }

    /// Records that the file on disk is no longer the one the editor read, while the text here has unsaved changes.
    pub fn mark_changed_on_disk(&mut self) {
        self.changed_on_disk = true;
    }

    /// The text as the bytes to write, in the file's own encoding and line endings, and the encoding the bytes are in.
    /// Text the file's encoding cannot hold is written as UTF-8.
    pub fn bytes(&self) -> (Vec<u8>, Encoding) {
        // The editor holds the file's text with plain line feeds, a last one included when the file ended in one.
        let text = self.content.text();
        let text = if self.crlf { text.replace('\n', "\r\n") } else { text };
        encoding::encode(&text, self.encoding)
    }

    /// Records that the text on screen is what the file now holds, written in `encoding` and stamped `stamp`.
    pub fn saved(&mut self, encoding: Encoding, stamp: Stamp) {
        self.reencoded = encoding != self.encoding;
        self.encoding = encoding;
        self.stamp = stamp;
        self.saved_revision = self.revision;
        self.changed_on_disk = false;
    }

    /// Replaces the text with the file as it was read again, keeping the scroll. The history starts over, since its
    /// steps were made on the text that is gone.
    pub fn reload(&mut self, loaded: Loaded) {
        let (scroll, viewport, read_only) = (self.scroll, self.viewport, self.read_only);
        let replaced = self.replaced + 1;
        *self = Self::new(loaded, read_only);
        self.scroll = scroll;
        self.viewport = viewport;
        self.replaced = replaced;
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Action(action) => self.act(action),
            Message::Steps(steps) => {
                if self.read_only {
                    return Task::none();
                }
                self.begin_edit(Kind::Other);
                let multiline = steps.iter().any(|step| matches!(step, Step::Text(text) if text.contains('\n')));
                for step in steps {
                    self.step(step);
                }
                self.after_edit(multiline);
                self.follow_caret()
            }
            Message::Command(command) => self.command(command),
            Message::Scrolled { x, y } => {
                self.scroll = (x, y);
                Task::none()
            }
            Message::Resized(size) => {
                self.viewport = size;
                Task::none()
            }
            // The owner answers these.
            Message::Save | Message::Revert | Message::Close => Task::none(),
        }
    }

    /// Applies an action of the text editor. An edit is recorded for undo first, and a read-only editor drops it.
    fn act(&mut self, action: Action) -> Task<Message> {
        if action.is_edit() {
            if self.read_only {
                return Task::none();
            }
            let (kind, whole) = match &action {
                Action::Edit(Edit::Insert(_)) => (Kind::Typing, false),
                Action::Edit(Edit::Backspace | Edit::Delete) => (Kind::Deleting, true),
                _ => (Kind::Other, true),
            };
            self.begin_edit(kind);
            self.content.perform(action);
            self.after_edit(whole && self.content.line_count() < 4000);
            return self.follow_caret();
        }
        // A move of the caret ends the step being typed, so an undo stops where the typing stopped.
        let scrolls = !matches!(action, Action::Scroll { .. });
        self.history.break_step();
        self.content.perform(action);
        if scrolls { self.follow_caret() } else { Task::none() }
    }

    /// Makes one of a typing helper's edits.
    fn step(&mut self, step: Step) {
        match step {
            Step::Text(text) => self.content.perform(Action::Edit(Edit::Paste(Arc::new(text)))),
            Step::Backspace => self.content.perform(Action::Edit(Edit::Backspace)),
            Step::Delete => self.content.perform(Action::Edit(Edit::Delete)),
            Step::Left => self.content.perform(Action::Move(Motion::Left)),
            Step::Right => self.content.perform(Action::Move(Motion::Right)),
            Step::Wrap { open, close } => {
                if let Some(selected) = self.content.selection() {
                    self.content
                        .perform(Action::Edit(Edit::Paste(Arc::new(format!("{open}{selected}{close}")))));
                }
            }
            Step::IndentLines => self.shift_lines(Shift::Indent),
        }
    }

    fn command(&mut self, command: Command) -> Task<Message> {
        match command {
            Command::Undo | Command::Redo => {
                let current = self.snapshot();
                let restored = if command == Command::Undo {
                    self.history.undo(current)
                } else {
                    self.history.redo(current)
                };
                if let Some(snapshot) = restored {
                    self.restore(snapshot);
                }
            }
            Command::Unindent | Command::ToggleComment if self.read_only => {}
            Command::Unindent => {
                self.begin_edit(Kind::Other);
                self.shift_lines(Shift::Unindent);
                self.after_edit(false);
            }
            Command::ToggleComment => {
                if let Some(mark) = self.language.line_comment() {
                    self.begin_edit(Kind::Other);
                    self.shift_lines(Shift::Comment(mark));
                    self.after_edit(true);
                }
            }
            Command::Page(direction) => {
                let lines = ((self.view_size().height / LINE_HEIGHT).floor() as usize).saturating_sub(1).max(1);
                let motion = if direction < 0 { Motion::Up } else { Motion::Down };
                self.history.break_step();
                for _ in 0..lines {
                    self.content.perform(Action::Move(motion));
                }
            }
        }
        self.follow_caret()
    }

    /// The text and the caret as they are now, for the history.
    fn snapshot(&self) -> Snapshot {
        Snapshot {
            text: self.content.text(),
            cursor: self.content.cursor(),
            revision: self.revision,
        }
    }

    /// Puts a snapshot from the history on screen.
    fn restore(&mut self, snapshot: Snapshot) {
        self.content = Content::with_text(&snapshot.text);
        self.content.move_to(snapshot.cursor);
        self.revision = snapshot.revision;
        self.replaced += 1;
        self.longest = snapshot.text.lines().map(columns).max().unwrap_or(0);
    }

    /// Records the text for undo when this edit starts a new step, and gives the text a new version.
    fn begin_edit(&mut self, kind: Kind) {
        if self.history.begins_step(kind, Instant::now()) {
            let before = self.snapshot();
            self.history.push(before);
        }
        self.last_revision += 1;
        self.revision = self.last_revision;
    }

    /// Brings the longest line up to date after an edit. `whole` measures every line again, which a change of several
    /// lines needs. Otherwise only the caret's line can have grown.
    fn after_edit(&mut self, whole: bool) {
        if whole {
            self.longest = self.content.lines().map(|line| columns(&line.text)).max().unwrap_or(0);
        } else if let Some(line) = self.content.line(self.content.cursor().position.line) {
            self.longest = self.longest.max(columns(&line.text));
        }
    }

    /// The lines an edit of whole lines covers: the caret's line, or every line the selection touches. A selection
    /// that ends at the very start of a line does not take that line in.
    fn selected_lines(&self) -> std::ops::RangeInclusive<usize> {
        let cursor = self.content.cursor();
        let (caret, anchor) = (cursor.position, cursor.selection.unwrap_or(cursor.position));
        let (first, last) = if (caret.line, caret.column) <= (anchor.line, anchor.column) {
            (caret, anchor)
        } else {
            (anchor, caret)
        };
        let end = if last.column == 0 && last.line > first.line {
            last.line - 1
        } else {
            last.line
        };
        first.line..=end
    }

    /// Indents, unindents or comments the selected lines, and puts the caret and the selection back where they were
    /// in the text, moved by what their own lines gained or lost.
    fn shift_lines(&mut self, shift: Shift) {
        let cursor = self.content.cursor();
        let lines = self.selected_lines();
        let texts: Vec<String> = lines
            .clone()
            .map(|line| self.content.line(line).map(|line| line.text.into_owned()).unwrap_or_default())
            .collect();
        // Commenting puts the mark at the shallowest indent of the lines, so the marks line up. The lines are
        // uncommented when every one that holds text already starts with the mark.
        let indents = texts
            .iter()
            .filter(|text| !text.trim().is_empty())
            .map(|text| text.len() - text.trim_start().len());
        let shallowest = indents.min().unwrap_or(0);
        let uncomment = match shift {
            Shift::Comment(mark) => texts
                .iter()
                .filter(|text| !text.trim().is_empty())
                .all(|text| text.trim_start().starts_with(mark)),
            _ => false,
        };
        // A selection would be replaced by the first insert, so it is dropped while the lines are edited.
        if cursor.selection.is_some() {
            self.content.perform(Action::Move(Motion::Left));
        }
        let mut moved: Vec<(usize, usize, isize)> = Vec::with_capacity(texts.len());
        for (line, text) in lines.zip(&texts) {
            let blank = text.trim().is_empty();
            let (column, change) = match shift {
                Shift::Indent if blank => continue,
                Shift::Indent => (0, Change::Insert(self.unit.clone())),
                Shift::Unindent => (0, Change::Remove(keys::unindent_width(text, &self.unit))),
                Shift::Comment(_) if blank => continue,
                Shift::Comment(mark) if uncomment => {
                    let at = text.len() - text.trim_start().len();
                    let spaced = text[at + mark.len()..].starts_with(' ');
                    (at, Change::Remove(mark.len() + usize::from(spaced)))
                }
                Shift::Comment(mark) => (shallowest, Change::Insert(format!("{mark} "))),
            };
            self.content.move_to(Cursor {
                position: Position { line, column },
                selection: None,
            });
            match change {
                Change::Insert(text) => {
                    moved.push((line, column, text.len() as isize));
                    self.content.perform(Action::Edit(Edit::Paste(Arc::new(text))));
                }
                Change::Remove(count) => {
                    moved.push((line, column, -(count as isize)));
                    for _ in 0..count {
                        self.content.perform(Action::Edit(Edit::Delete));
                    }
                }
            }
        }
        let shifted = |position: Position| {
            let column = moved
                .iter()
                .find(|(line, _, _)| *line == position.line)
                .map_or(position.column, |(_, at, by)| shift_column(position.column, *at, *by));
            Position { column, ..position }
        };
        self.content.move_to(Cursor {
            position: shifted(cursor.position),
            selection: cursor.selection.map(shifted),
        });
    }

    /// The size of the scroll the text sits in, or a size to assume before it has been laid out.
    fn view_size(&self) -> Size {
        if self.viewport.width > 0.0 && self.viewport.height > 0.0 {
            self.viewport
        } else {
            ASSUMED_VIEW
        }
    }

    /// Scrolls so the caret is on screen, when it is not. The editor is as tall and as wide as its text, so the scroll
    /// around it has to be moved to follow the caret.
    fn follow_caret(&mut self) -> Task<Message> {
        let caret = self.content.cursor().position;
        let column = self
            .content
            .line(caret.line)
            .map_or(0, |line| columns(&line.text[..caret.column.min(line.text.len())]));
        let view = self.view_size();
        let x = reveal(
            self.scroll.0,
            view.width,
            PADDING + column as f32 * ADVANCE,
            ADVANCE,
            PADDING + ADVANCE * 4.0,
        );
        let y = reveal(
            self.scroll.1,
            view.height,
            PADDING + caret.line as f32 * LINE_HEIGHT,
            LINE_HEIGHT,
            PADDING,
        );
        if x.is_none() && y.is_none() {
            return Task::none();
        }
        self.scroll = (x.unwrap_or(self.scroll.0), y.unwrap_or(self.scroll.1));
        operation::scroll_to(Id::new(SCROLL_ID), AbsoluteOffset { x, y })
    }

    /// The binding for a key press: the editor's own shortcuts, then the typing helpers, then what the text editor
    /// does by default. A read-only editor keeps the keys that move, select and copy.
    fn binding(&self, press: KeyPress) -> Option<Binding<Message>> {
        if !matches!(press.status, Status::Focused { .. }) {
            return None;
        }
        let modifiers = press.modifiers;
        if modifiers.command() && !modifiers.alt() {
            let shortcut = match press.key.to_latin(press.physical_key) {
                Some('s') => Some(Message::Save),
                Some('z') if modifiers.shift() => Some(Message::Command(Command::Redo)),
                Some('z') => Some(Message::Command(Command::Undo)),
                Some('y') => Some(Message::Command(Command::Redo)),
                Some('/') => Some(Message::Command(Command::ToggleComment)),
                _ => None,
            };
            if let Some(message) = shortcut {
                return Some(Binding::Custom(message));
            }
        }
        if self.read_only {
            return Binding::from_key_press(press).filter(|binding| {
                !matches!(
                    binding,
                    Binding::Insert(_) | Binding::Enter | Binding::Backspace | Binding::Delete | Binding::Paste | Binding::Cut
                )
            });
        }
        let named = match press.modified_key.as_ref() {
            keyboard::Key::Named(named) => Some(named),
            _ => None,
        };
        let typed = match named {
            Some(keyboard::key::Named::Tab) if modifiers.shift() => return Some(Binding::Custom(Message::Command(Command::Unindent))),
            Some(keyboard::key::Named::PageUp) if !modifiers.shift() => return Some(Binding::Custom(Message::Command(Command::Page(-1)))),
            Some(keyboard::key::Named::PageDown) if !modifiers.shift() => return Some(Binding::Custom(Message::Command(Command::Page(1)))),
            Some(keyboard::key::Named::Tab) => Some(Typed::Tab),
            Some(keyboard::key::Named::Enter) => Some(Typed::Enter),
            Some(keyboard::key::Named::Backspace) => Some(Typed::Backspace),
            _ if modifiers.command() || modifiers.alt() => None,
            _ => press
                .text
                .as_ref()
                .and_then(|text| text.chars().find(|character| !character.is_control()))
                .map(Typed::Char),
        };
        if let Some(typed) = typed {
            let cursor = self.content.cursor();
            let line = self
                .content
                .line(cursor.position.line)
                .map(|line| line.text.into_owned())
                .unwrap_or_default();
            let context = Context {
                line: &line,
                column: cursor.position.column,
                selection: cursor.selection.is_some_and(|anchor| anchor != cursor.position),
                language: self.language,
                unit: &self.unit,
            };
            if let Some(steps) = keys::plan(typed, &context) {
                return Some(Binding::Custom(Message::Steps(steps)));
            }
        }
        Binding::from_key_press(press)
    }
}

/// What [`Editor::shift_lines`] does to each line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shift {
    Indent,
    Unindent,
    /// Adds this comment mark, or removes it when every line already has it.
    Comment(&'static str),
}

/// The change to one line of a shift.
enum Change {
    Insert(String),
    Remove(usize),
}

/// Where a caret at `column` ends up when `by` bytes are inserted (or, when negative, removed) at `at` in its line. A
/// caret before the change stays, and one inside removed text lands where the text was.
fn shift_column(column: usize, at: usize, by: isize) -> usize {
    if column < at {
        column
    } else if by >= 0 {
        column + by.unsigned_abs()
    } else {
        column.saturating_sub(by.unsigned_abs()).max(at)
    }
}

/// How many columns `text` takes on screen: a tab runs to the next multiple of [`TAB_COLUMNS`], and a wide character,
/// such as a CJK one, takes two.
fn columns(text: &str) -> usize {
    text.chars().fold(0, |column, character| match character {
        '\t' => (column / TAB_COLUMNS + 1) * TAB_COLUMNS,
        character if (character as u32) >= 0x1100 => column + 2,
        _ => column + 1,
    })
}

/// The scroll offset that brings a span onto the screen, or `None` when it is already there. The span starts at
/// `start` and is `length` long, the screen shows `view` from `offset`, and `margin` is kept clear around the span.
fn reveal(offset: f32, view: f32, start: f32, length: f32, margin: f32) -> Option<f32> {
    if start - margin < offset {
        Some((start - margin).max(0.0))
    } else if start + length + margin > offset + view {
        Some((start + length + margin - view).max(0.0))
    } else {
        None
    }
}

#[cfg(test)]
#[path = "../../../../tests/app/components/editor/mod.rs"]
mod tests;
