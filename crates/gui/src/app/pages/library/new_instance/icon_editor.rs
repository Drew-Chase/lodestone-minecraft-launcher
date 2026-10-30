//! The icon editor: a second modal over the New Instance dialog. It mixes a background
//! preset with a symbol, or takes an uploaded image in their place. The backgrounds page
//! through a carousel, and the symbols wrap down a grid with a scrollbar of its own.

use aurora_leaf::Icon;
use iced::widget::scrollable::AbsoluteOffset;
use iced::widget::{Id, Space, Text, column, container, operation, row, svg};
use iced::{Element, Fill, Length, Padding, Task, alignment};

use super::{IconChoice, Upload};
use crate::app::components::carousel::{self, Step};
use crate::app::components::instance_icon::{self, BACKGROUNDS, SYMBOLS};
use crate::app::components::modal::Modal;
use crate::app::components::scrollbar;
use crate::app::components::tile::{INSET, tile};
use crate::app::components::{TextButton, button::Variant};
use crate::app::lang;
use crate::app::pages::library::instance::InstanceIcon;
use crate::app::theme::{color, fonts};

/// The width of the editor card.
const WIDTH: f32 = 760.0;
/// The large preview's edge.
const PREVIEW: f32 = 152.0;
/// The edge of one cell: a swatch, or a symbol in the grid.
const CELL: f32 = 56.0;
/// The gap between cells, across and down.
const GAP: f32 = 8.0;
/// How many swatches the carousel shows at once.
const VISIBLE: usize = 6;
/// The symbol grid's columns, and the height it shows before its own scrollbar appears.
const COLUMNS: usize = 6;
const GRID_HEIGHT: f32 = 176.0;
/// The space the carousel's back button takes, so the symbol columns line up with the swatches.
const NAV_GUTTER: f32 = 36.0;
/// How many recents the editor lists.
pub const RECENTS_SHOWN: usize = 4;

/// The editor's working copy of the icon, applied on Save.
#[derive(Debug, Clone)]
pub struct IconEditor {
    background: &'static str,
    symbol: &'static str,
    upload: Option<Upload>,
    recents: Vec<InstanceIcon>,
    /// The first swatch the background carousel shows.
    background_offset: usize,
}

/// The editor's messages.
#[derive(Debug, Clone)]
pub enum Message {
    Background(usize),
    Symbol(usize),
    BackgroundsStep(Step),
    Randomize,
    RecentPicked(usize),
    Upload,
    Save,
    Cancel,
}

/// What an editor message leaves behind.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    /// Keep the editor open.
    Stay,
    /// Ask for an image to replace the composition.
    Upload,
    /// Close with this icon.
    Save(IconChoice),
    /// Close without changing the icon.
    Cancel,
}

/// The symbol grid's scrollable, so the editor can bring a chosen symbol into view.
fn grid_id() -> Id {
    Id::new("icon-editor-symbols")
}

impl IconEditor {
    /// An editor that starts from `icon`, with the recents the database holds.
    pub fn new(icon: &IconChoice, recents: Vec<InstanceIcon>) -> Self {
        let (background, symbol, upload) = match icon {
            IconChoice::Composed { background, symbol } => (*background, *symbol, None),
            IconChoice::Uploaded(upload) => (BACKGROUNDS[0].id, SYMBOLS[0].id, Some(upload.clone())),
        };
        let mut editor = Self {
            background,
            symbol,
            upload,
            recents,
            background_offset: 0,
        };
        editor.reveal(background, symbol);
        editor
    }

    /// Chooses a background and a symbol, and pages the carousel to show the background.
    fn reveal(&mut self, background: &'static str, symbol: &'static str) {
        self.background = background;
        self.symbol = symbol;
        let background_index = BACKGROUNDS.iter().position(|preset| preset.id == background).unwrap_or(0);
        self.background_offset = carousel::revealing(background_index, BACKGROUNDS.len(), VISIBLE);
    }

    /// The row of the symbol grid the chosen symbol sits on.
    pub fn symbol_row(&self) -> usize {
        let index = SYMBOLS.iter().position(|entry| entry.id == self.symbol).unwrap_or(0);
        index / COLUMNS
    }

    /// Applies a message from the editor.
    pub fn update(&mut self, message: Message) -> Outcome {
        match message {
            Message::Background(index) => {
                self.background = BACKGROUNDS[index].id;
                self.upload = None;
                Outcome::Stay
            }
            Message::Symbol(index) => {
                self.symbol = SYMBOLS[index].id;
                self.upload = None;
                Outcome::Stay
            }
            Message::BackgroundsStep(step) => {
                self.background_offset = carousel::stepped(self.background_offset, BACKGROUNDS.len(), VISIBLE, step);
                Outcome::Stay
            }
            Message::Randomize => {
                let (background, symbol) = instance_icon::random();
                self.reveal(background, symbol);
                self.upload = None;
                Outcome::Stay
            }
            Message::RecentPicked(index) => {
                if let Some(InstanceIcon::Composed { background, symbol }) = self.recents.get(index) {
                    let background = instance_icon::preset(background).id;
                    let symbol = instance_icon::symbol(symbol).id;
                    self.reveal(background, symbol);
                    self.upload = None;
                }
                Outcome::Stay
            }
            Message::Upload => Outcome::Upload,
            Message::Save => Outcome::Save(self.choice()),
            Message::Cancel => Outcome::Cancel,
        }
    }

    /// Scrolls the symbol grid so the chosen symbol's row is at its top. The editor calls
    /// this after a choice moved the symbol, such as Randomize or a recent.
    pub fn reveal_symbol<T: 'static>(&self) -> Task<T> {
        let top = self.symbol_row() as f32 * (CELL + GAP);
        operation::scroll_to(grid_id(), AbsoluteOffset { x: None, y: Some(top) })
    }

    /// Replaces the composition with an uploaded image.
    pub fn set_upload(&mut self, upload: Upload) {
        self.upload = Some(upload);
    }

    /// The icon as the editor holds it now.
    pub fn choice(&self) -> IconChoice {
        match &self.upload {
            Some(upload) => IconChoice::Uploaded(upload.clone()),
            None => IconChoice::Composed {
                background: self.background,
                symbol: self.symbol,
            },
        }
    }

    /// The editor card: previews on the left, the pickers on the right.
    pub fn view(&self) -> Element<'_, Message> {
        let small = row![self.preview(48.0), self.preview(32.0), self.preview(20.0)]
            .spacing(12)
            .align_y(alignment::Vertical::Center);
        let left = column![
            container(self.preview(PREVIEW)).width(Fill).align_x(alignment::Horizontal::Center),
            container(small).width(Fill).align_x(alignment::Horizontal::Center),
            TextButton::content(
                row![
                    glyph(Icon::lucide().refresh_cw(), 14.0, color::Text::DEFAULT),
                    Text::new("Randomize").size(12).font(fonts::Inter::SEMI_BOLD).color(color::Text::DEFAULT),
                ]
                .spacing(8)
                .align_y(alignment::Vertical::Center),
            )
            .padding(Padding::new(0.0).left(14).right(14).top(9).bottom(9))
            .width(Fill)
            .on_press(Message::Randomize),
            TextButton::content(
                row![
                    glyph(Icon::lucide().upload(), 14.0, color::Text::DEFAULT),
                    Text::new(lang::t("icon.upload"))
                        .size(12)
                        .font(fonts::Inter::SEMI_BOLD)
                        .color(color::Text::DEFAULT),
                ]
                .spacing(8)
                .align_y(alignment::Vertical::Center),
            )
            .padding(Padding::new(0.0).left(14).right(14).top(9).bottom(9))
            .width(Fill)
            .on_press(Message::Upload),
            label(lang::t("icon.recents")),
            self.recents_row(),
        ]
        .spacing(14)
        .width(220);

        let right = column![
            label(lang::t("icon.background")),
            self.swatches(),
            label(lang::t("icon.symbol")),
            self.symbols()
        ]
        .spacing(12)
        .width(Fill);

        row![left, right].spacing(24).width(Fill).into()
    }

    /// The composed icon, or the upload, at `size` pixels square.
    fn preview(&self, size: f32) -> Element<'_, Message> {
        match &self.upload {
            Some(upload) => instance_icon::bytes(upload.handle.clone(), size),
            None => instance_icon::composed(self.background, self.symbol, size),
        }
    }

    /// The background swatches as a carousel, the chosen one ringed.
    fn swatches(&self) -> Element<'_, Message> {
        let items: Vec<Element<'_, Message>> = BACKGROUNDS
            .iter()
            .enumerate()
            .map(|(index, preset)| {
                let chosen = self.upload.is_none() && preset.id == self.background;
                tile(
                    instance_icon::swatch(preset.id, CELL - 2.0 * INSET),
                    CELL,
                    CELL,
                    14.0,
                    chosen,
                    Message::Background(index),
                )
            })
            .collect();
        carousel::carousel(items, self.background_offset, VISIBLE, CELL, Message::BackgroundsStep)
    }

    /// The symbols as a grid that wraps downward, with a scrollbar of its own. Its left edge
    /// lines up with the swatches' first cell.
    fn symbols(&self) -> Element<'_, Message> {
        let rows: Vec<Element<'_, Message>> = SYMBOLS
            .chunks(COLUMNS)
            .enumerate()
            .map(|(row_index, chunk)| {
                let cells: Vec<Element<'_, Message>> = chunk
                    .iter()
                    .enumerate()
                    .map(|(offset, entry)| {
                        let index = row_index * COLUMNS + offset;
                        let chosen = self.upload.is_none() && entry.id == self.symbol;
                        tile(
                            container(instance_icon::composed(self.background, entry.id, CELL - 12.0)).padding(4.0),
                            CELL,
                            CELL,
                            14.0,
                            chosen,
                            Message::Symbol(index),
                        )
                    })
                    .collect();
                row(cells).spacing(GAP).into()
            })
            .collect();

        let grid_width = COLUMNS as f32 * CELL + (COLUMNS - 1) as f32 * GAP + scrollbar::TRACK_WIDTH;
        let grid = scrollbar::scroll(column(rows).spacing(GAP))
            .id(grid_id())
            .width(Length::Fixed(grid_width))
            .height(GRID_HEIGHT);
        row![Space::new().width(NAV_GUTTER), grid].into()
    }

    /// The recents, newest first, drawn as composed icons.
    fn recents_row(&self) -> Element<'_, Message> {
        row(self.recents.iter().take(RECENTS_SHOWN).enumerate().map(|(index, recent)| {
            let body: Element<'_, Message> = match recent {
                InstanceIcon::Composed { background, symbol } => instance_icon::composed(background, symbol, 36.0 - 2.0 * INSET),
                InstanceIcon::Custom(_) => Space::new().width(36).height(36).into(),
            };
            tile(body, 36.0, 36.0, 9.0, false, Message::RecentPicked(index))
        }))
        .spacing(8)
        .into()
    }
}

/// The editor's modal card, over the dialog beneath it.
pub fn modal(editor: &IconEditor) -> Element<'_, Message> {
    Modal::new(lang::t("icon.title"), editor.view())
        .subtitle(lang::t("icon.subtitle"))
        .icon(Icon::lucide().pencil(), color::Primary::DEFAULT, color::Primary::TINT)
        .footer(
            row![
                Space::new().width(Fill),
                TextButton::content(
                    row![
                        glyph(Icon::lucide().x(), 13.0, color::Text::DEFAULT),
                        Text::new("Cancel").size(13).font(fonts::Inter::SEMI_BOLD).color(color::Text::DEFAULT),
                    ]
                    .spacing(8)
                    .align_y(alignment::Vertical::Center),
                )
                .variant(Variant::Ghost)
                .padding(Padding::new(0.0).left(16).right(16).top(11).bottom(11))
                .on_press(Message::Cancel),
                TextButton::content(
                    row![
                        glyph(Icon::lucide().save(), 14.0, color::Primary::ON),
                        Text::new("Save icon").size(13).font(fonts::Inter::BOLD).color(color::Primary::ON),
                    ]
                    .spacing(8)
                    .align_y(alignment::Vertical::Center),
                )
                .variant(Variant::Primary)
                .padding(Padding::new(0.0).left(18).right(18).top(11).bottom(11))
                .on_press(Message::Save),
            ]
            .spacing(10)
            .align_y(alignment::Vertical::Center),
        )
        .on_close(Message::Cancel)
        .width(WIDTH)
        .into()
}

/// A small uppercase section label.
fn label(text: &'static str) -> Element<'static, Message> {
    Text::new(text)
        .size(11)
        .font(fonts::JetBrainsMono::REGULAR)
        .color(color::Text::MUTED)
        .into()
}

/// A lucide glyph in `ink`.
fn glyph<'a, M: 'a>(markup: &'static str, size: f32, ink: iced::Color) -> Element<'a, M> {
    svg(svg::Handle::from_memory(markup.as_bytes()))
        .width(size)
        .height(size)
        .style(move |_, _| svg::Style { color: Some(ink) })
        .into()
}

#[cfg(test)]
#[path = "../../../../../tests/app/pages/library/new_instance/icon_editor.rs"]
mod tests;
