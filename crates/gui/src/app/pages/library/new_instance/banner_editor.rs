//! The banner editor: a modal over the dialog. It shows the banner that is chosen, then every
//! scene artwork as a thumbnail, with a last tile for an uploaded image.

use aurora_leaf::Icon;
use iced::widget::{Space, Text, column, container, row, svg};
use iced::{Background, Color, Element, Fill, alignment};
use iced_color_extension::rgb;

use super::banner;
use super::{BannerChoice, Upload};
use crate::app::components::TextButton;
use crate::app::components::button::Variant;
use crate::app::components::modal::Modal;
use crate::app::components::tile::tile;
use crate::app::theme::{color, fonts};

/// The card's width: five thumbnails and the gaps between them, inside the body's padding.
const WIDTH: f32 = 720.0;
/// A thumbnail's width. Its height is the 16:9 frame.
const THUMB: f32 = 112.0;
/// The grid's columns, and the gap between cells.
const COLUMNS: usize = 5;
const GAP: f32 = 8.0;
/// The height of the preview of the chosen banner.
const PREVIEW_HEIGHT: f32 = 150.0;

/// The editor's messages.
#[derive(Debug, Clone)]
pub enum Message {
    /// A scene thumbnail was pressed.
    Pick(usize),
    /// The upload tile was pressed.
    Upload,
    Save,
    Cancel,
}

/// What an editor message leaves behind.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    /// Keep the editor open.
    Stay,
    /// Ask for an image to show in place of the scenes.
    Upload,
    /// Close with this banner.
    Save(BannerChoice),
    /// Close without changing the banner.
    Cancel,
}

/// The editor's working copy of the banner, applied on Save.
#[derive(Debug, Clone)]
pub struct BannerEditor {
    choice: BannerChoice,
}

impl BannerEditor {
    /// An editor that starts from `choice`.
    pub fn new(choice: &BannerChoice) -> Self {
        Self { choice: choice.clone() }
    }

    /// Shows an uploaded image in place of the scenes.
    pub fn set_upload(&mut self, upload: Upload) {
        self.choice = BannerChoice::Uploaded(upload);
    }

    /// Applies a message from the editor.
    pub fn update(&mut self, message: Message) -> Outcome {
        match message {
            Message::Pick(index) => {
                self.choice = BannerChoice::Scene(index);
                Outcome::Stay
            }
            Message::Upload => Outcome::Upload,
            Message::Save => Outcome::Save(self.choice.clone()),
            Message::Cancel => Outcome::Cancel,
        }
    }
}

/// The editor's modal card, over the dialog beneath it.
pub fn modal(editor: &BannerEditor) -> Element<'_, Message> {
    let preview = banner::clipped(preview_art(editor), Fill, PREVIEW_HEIGHT, 12.0);
    let caption = Text::new(caption_text(editor))
        .size(12)
        .font(fonts::Inter::MEDIUM)
        .color(color::Text::MUTED);

    let rows: Vec<Element<'_, Message>> = (0..=banner::SCENE_COUNT)
        .step_by(COLUMNS)
        .map(|start| {
            let end = (start + COLUMNS).min(banner::SCENE_COUNT + 1);
            let cells: Vec<Element<'_, Message>> = (start..end).map(|index| cell(editor, index)).collect();
            row(cells).spacing(GAP).into()
        })
        .collect();
    let grid = container(column(rows).spacing(GAP)).width(Fill).align_x(alignment::Horizontal::Center);

    let body = column![preview, caption, grid].spacing(12);
    Modal::new("Banner", body)
        .subtitle("Choose the artwork for this instance, or upload your own image.")
        .icon(Icon::lucide().image(), color::Primary::DEFAULT, color::Primary::TINT)
        .footer(footer())
        .on_close(Message::Cancel)
        .width(WIDTH)
        .into()
}

/// The banner the editor currently holds, as a preview.
fn preview_art(editor: &BannerEditor) -> Element<'_, Message> {
    match &editor.choice {
        BannerChoice::Scene(index) => banner::art(*index),
        BannerChoice::Uploaded(upload) => banner::uploaded(upload.handle.clone()),
    }
}

/// The line under the preview: which scene is chosen, or that an image is.
fn caption_text(editor: &BannerEditor) -> String {
    match &editor.choice {
        BannerChoice::Scene(index) => banner::scene_name(*index),
        BannerChoice::Uploaded(_) => "Uploaded image".to_string(),
    }
}

/// One grid cell: a scene thumbnail, or the upload tile after the last scene.
fn cell(editor: &BannerEditor, index: usize) -> Element<'_, Message> {
    if index == banner::SCENE_COUNT {
        return upload_cell(editor);
    }
    let selected = match &editor.choice {
        BannerChoice::Scene(chosen) => *chosen == index,
        BannerChoice::Uploaded(_) => false,
    };
    banner::thumbnail(index, selected, THUMB, Message::Pick(index))
}

/// The upload tile. It shows the uploaded image once there is one, and asks for an image
/// until then.
fn upload_cell(editor: &BannerEditor) -> Element<'_, Message> {
    let (content, selected): (Element<'_, Message>, bool) = match &editor.choice {
        BannerChoice::Uploaded(upload) => (banner::uploaded(upload.handle.clone()), true),
        BannerChoice::Scene(_) => (placeholder(), false),
    };
    tile(
        banner::clipped(content, Fill, Fill, 9.0),
        THUMB,
        THUMB * 9.0 / 16.0,
        10.0,
        selected,
        Message::Upload,
    )
}

/// The face of the upload tile before an image is chosen.
fn placeholder<'a>() -> Element<'a, Message> {
    let face = column![
        glyph(Icon::lucide().upload(), 16.0, color::Text::MUTED),
        Text::new("Upload").size(11).font(fonts::Inter::SEMI_BOLD).color(color::Text::MUTED),
    ]
    .spacing(6)
    .align_x(alignment::Horizontal::Center);
    container(face)
        .width(Fill)
        .height(Fill)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .style(|_| container::Style {
            background: Some(Background::Color(rgb!(255, 255, 255, 4%))),
            ..container::Style::default()
        })
        .into()
}

/// The footer: Cancel and Save.
fn footer<'a>() -> Element<'a, Message> {
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
        .padding(iced::Padding::new(0.0).left(16).right(16).top(11).bottom(11))
        .on_press(Message::Cancel),
        TextButton::content(
            row![
                glyph(Icon::lucide().save(), 14.0, color::Primary::ON),
                Text::new("Save banner").size(13).font(fonts::Inter::BOLD).color(color::Primary::ON),
            ]
            .spacing(8)
            .align_y(alignment::Vertical::Center),
        )
        .variant(Variant::Primary)
        .padding(iced::Padding::new(0.0).left(18).right(18).top(11).bottom(11))
        .on_press(Message::Save),
    ]
    .spacing(10)
    .align_y(alignment::Vertical::Center)
    .into()
}

/// A lucide glyph in `ink`.
fn glyph<'a, M: 'a>(markup: &'static str, size: f32, ink: Color) -> Element<'a, M> {
    svg(iced::widget::svg::Handle::from_memory(markup.as_bytes()))
        .width(size)
        .height(size)
        .style(move |_, _| iced::widget::svg::Style { color: Some(ink) })
        .into()
}

#[cfg(test)]
#[path = "../../../../../tests/app/pages/library/new_instance/banner_editor.rs"]
mod tests;
