//! The gallery: the project's pictures in a grid, a page at a time, and the viewer a picture opens in.

use aurora_leaf::Icon;
use iced::widget::{Space, Text, column, container, image, opaque, row, stack};
use iced::{Background, Color, ContentFit, Element, Fill, Length, Size, alignment};
use lodestone_platforms::GalleryImage;

use super::parts::pager;
use super::{GALLERY_PER_PAGE, Message, ResourcePage};
use crate::app::components::IconButton;
use crate::app::components::hint::hint;
use crate::app::components::icon_button::SECONDARY_BUTTON_STYLE;
use crate::app::components::page::content_width;
use crate::app::components::tile::tile;
use crate::app::lang;
use crate::app::theme::{color, fonts};

/// The columns of the gallery's grid.
const COLUMNS: usize = 3;
/// The gap between the grid's pictures.
const GAP: f32 = 16.0;
/// The corner radius of a picture.
const RADIUS: f32 = 12.0;

/// The gallery: its page of pictures in a grid, and the pager under it.
pub(super) fn view<'a>(page: &'a ResourcePage, window: Size) -> Element<'a, Message> {
    let pictures = page.gallery();
    let pages = pictures.len().div_ceil(GALLERY_PER_PAGE).max(1);
    let start = page.gallery_page.min(pages - 1) * GALLERY_PER_PAGE;
    let width = ((content_width(window.width) - GAP * (COLUMNS as f32 - 1.0)) / COLUMNS as f32).max(160.0);
    let height = width * 9.0 / 16.0;
    let visible: Vec<(usize, &GalleryImage)> = pictures.iter().enumerate().skip(start).take(GALLERY_PER_PAGE).collect();
    let rows: Vec<Element<'a, Message>> = visible
        .chunks(COLUMNS)
        .map(|line| {
            let cells: Vec<Element<'a, Message>> = line.iter().map(|&(index, picture)| cell(page, index, picture, width, height)).collect();
            row(cells).spacing(GAP).into()
        })
        .collect();
    column![
        column(rows).spacing(GAP).width(Fill),
        container(pager(page.gallery_page.min(pages - 1), pages, Message::GalleryPage))
            .width(Fill)
            .align_x(alignment::Horizontal::Center),
    ]
    .spacing(24)
    .width(Fill)
    .into()
}

/// One picture of the grid, with its title under it. A press opens the viewer on the picture.
fn cell<'a>(page: &'a ResourcePage, index: usize, picture: &'a GalleryImage, width: f32, height: f32) -> Element<'a, Message> {
    let thumbnail = thumbnail(page.picture(&picture.thumbnail_url), width, height);
    let caption = Text::new(picture.title.clone().unwrap_or_default())
        .size(12)
        .font(fonts::Inter::MEDIUM)
        .color(color::Text::DEFAULT)
        .width(Length::Fixed(width));
    tile(
        column![thumbnail, caption].spacing(8).width(Length::Fixed(width)),
        Length::Fixed(width),
        Length::Shrink,
        RADIUS,
        false,
        Message::OpenPicture(index),
    )
}

/// A picture cut to fill its box, or a plain panel while it loads.
fn thumbnail<'a>(handle: Option<&image::Handle>, width: f32, height: f32) -> Element<'a, Message> {
    match handle {
        Some(handle) => image(handle.clone()).width(width).height(height).content_fit(ContentFit::Cover).into(),
        None => container(Space::new())
            .width(width)
            .height(height)
            .style(|_| container::Style {
                background: Some(Background::Color(color::Surface::BG3)),
                ..container::Style::default()
            })
            .into(),
    }
}

/// The viewer a gallery picture opens in: the picture at full size over a dim backdrop, with buttons to step through
/// the gallery and to close it. It covers the window, and nothing beneath it takes a press.
pub(super) fn viewer<'a>(page: &'a ResourcePage, index: usize, picture: &'a GalleryImage) -> Element<'a, Message> {
    let count = page.gallery().len();
    let shown: Element<'a, Message> = match page.picture(&picture.full_url).or_else(|| page.picture(&picture.thumbnail_url)) {
        Some(handle) => image(handle.clone()).width(Fill).height(Fill).content_fit(ContentFit::Contain).into(),
        None => Space::new().width(Fill).height(Fill).into(),
    };
    let backdrop = container(Space::new().width(Fill).height(Fill)).style(|_| container::Style {
        background: Some(Background::Color(Color::from_rgba(0.03, 0.04, 0.05, 0.92))),
        ..container::Style::default()
    });
    let caption = Text::new(lang::t_with(
        "resource.viewer.count",
        &[
            ("index", (index + 1).to_string().as_str()),
            ("count", count.to_string().as_str()),
            ("title", picture.title.as_deref().unwrap_or_default()),
        ],
    ))
    .size(12)
    .font(fonts::Inter::MEDIUM)
    .color(color::Text::MUTED);
    let close = IconButton::new(Icon::lucide().x())
        .size(36.0)
        .radius(10.0)
        .icon_size(16.0)
        .style(SECONDARY_BUTTON_STYLE)
        .on_press(Message::ClosePicture);
    let previous = step_button(
        Icon::lucide().chevron_left(),
        index > 0,
        Message::StepPicture(-1),
        lang::t("resource.viewer.previous"),
    );
    let next = step_button(
        Icon::lucide().chevron_right(),
        index + 1 < count,
        Message::StepPicture(1),
        lang::t("resource.viewer.next"),
    );
    let controls = row![previous, shown, next,]
        .spacing(16)
        .align_y(alignment::Vertical::Center)
        .width(Fill)
        .height(Fill);
    let layer = column![
        row![Space::new().width(Fill), hint(close, lang::t("resource.viewer.close"))].padding(16),
        container(controls).padding(16).width(Fill).height(Fill),
        container(caption).padding(16).width(Fill).align_x(alignment::Horizontal::Center),
    ]
    .width(Fill)
    .height(Fill);
    opaque(stack![backdrop, layer].width(Fill).height(Fill))
}

/// A button that steps the viewer, dimmed at either end of the gallery.
fn step_button<'a>(markup: &'static str, enabled: bool, message: Message, label: &'static str) -> Element<'a, Message> {
    let button = IconButton::new(markup)
        .size(40.0)
        .radius(10.0)
        .icon_size(18.0)
        .style(SECONDARY_BUTTON_STYLE);
    let button = if enabled { button.on_press(message) } else { button.disabled(true) };
    hint(button, label)
}
