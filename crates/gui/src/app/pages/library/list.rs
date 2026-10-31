//! The "Your Instances" section and its virtualized list, which windows rows
//! against the page scroll.

use std::ops::Range;

use crate::app::components::art_cache::{ArtCache, ArtKey, Kind};
use crate::app::components::instance_card::{InstanceMessages, THUMB_SIZE, compact_row, instance_card, table_header, table_row};
use crate::app::lang;
use crate::app::pages::library::instance::Instance;
use crate::app::perf;
use crate::app::theme::{color, fonts};
use iced::widget::image::Handle;
use iced::widget::{Space, Text, column, container, row};
use iced::{Element, Fill, Length, Size, alignment};

use super::{LIST_TOP, Library, LibraryView, Message, card_art_size, content_width};

/// The rows the list builds at a window's size.
pub(super) struct RowWindow {
    /// How many instances sit in a row: the grid's columns, or one.
    pub columns: usize,
    /// How many rows the whole list has.
    pub total_rows: usize,
    /// The first row built, and the row after the last one built.
    pub first: usize,
    pub end: usize,
}

impl Library {
    /// The "Your Instances" section — title, filter tabs, view switcher,
    /// and the virtualized list (which continues the page scroll).
    pub(super) fn instances_section(&self, viewport: Size, art: &ArtCache) -> Element<'_, Message> {
        column![
            row![
                Text::new(lang::t("library.your_instances"))
                    .size(18)
                    .font(fonts::Inter::EXTRA_BOLD)
                    .color(color::Text::HEADLINE),
                Space::new().width(Fill),
                self.filter_tabs(),
                Space::new().width(12),
                self.view_switcher(),
            ]
            .align_y(alignment::Vertical::Center)
            .width(Fill),
            self.instances_list(viewport, art),
        ]
        .spacing(16)
        .width(Fill)
        .into()
    }

    /// The virtualized instance list — plain rows inside the page scroll.
    /// Only the rows around the scroll offset are built; top/bottom
    /// spacers hold the page height stable so the scrollbar reflects the
    /// full list.
    fn instances_list(&self, viewport: Size, art: &ArtCache) -> Element<'static, Message> {
        let _scope = perf::scope("view.library.list");
        let visible = self.visible();
        if visible.is_empty() {
            return container(
                Text::new(if self.instances.is_empty() {
                    lang::t("library.empty")
                } else {
                    lang::t("library.no_matches")
                })
                .size(13)
                .font(fonts::Inter::REGULAR)
                .color(color::Text::MUTED),
            )
            .width(Fill)
            .height(Length::Fixed((viewport.height - LIST_TOP).max(160.0)))
            .align_x(alignment::Horizontal::Center)
            .align_y(alignment::Vertical::Center)
            .into();
        }

        let view = self.view;
        let pitch = view.row_pitch();
        let gap = view.gap();
        let item_height = view.item_height();
        let window = self.row_window(viewport, visible.len());
        let columns = window.columns;
        let (card_width, card_height) = card_art_size(content_width(viewport.width), columns);

        let mut content = column![Space::new().height(window.first as f32 * pitch)].spacing(gap);
        for row_index in window.first..window.end {
            let start = row_index * columns;
            let slice = &visible[start..(start + columns).min(visible.len())];
            let mut items: Vec<Element<'static, Message>> = Vec::with_capacity(columns);
            match view {
                LibraryView::Grid => {
                    for instance in slice {
                        let hovered = self.hovered.as_deref() == Some(instance.id.as_str());
                        let install = self.pending_install(&instance.id);
                        let banner = picture(art, instance, Kind::Card, card_width, card_height);
                        items.push(instance_card(instance, hovered, instance_messages(instance), install, banner));
                    }
                }
                LibraryView::Compact => {
                    for instance in slice {
                        let install = self.pending_install(&instance.id);
                        let thumb = picture(art, instance, Kind::Thumb, THUMB_SIZE, THUMB_SIZE);
                        items.push(compact_row(instance, instance_messages(instance), install, thumb));
                    }
                }
                LibraryView::Table => {
                    for (offset, instance) in slice.iter().enumerate() {
                        let install = self.pending_install(&instance.id);
                        items.push(table_row(start + offset, instance, instance_messages(instance), install));
                    }
                }
            }
            // Pad short rows so Fill-width items keep a uniform size.
            for _ in slice.len()..columns {
                items.push(Space::new().width(Fill).height(item_height).into());
            }
            content = content.push(row(items).spacing(24).width(Fill));
        }
        let bottom = (window.total_rows - window.end) as f32 * pitch;
        content = content.push(Space::new().height(bottom));

        // The table keeps its header pinned above its rows.
        if view == LibraryView::Table {
            column![table_header(), content].width(Fill).into()
        } else {
            content.into()
        }
    }

    /// The rows the list builds at `viewport` for `visible` instances: the grid's columns, the row count,
    /// and the rows around the scroll offset.
    pub(super) fn row_window(&self, viewport: Size, visible: usize) -> RowWindow {
        let view = self.view;
        let pitch = view.row_pitch();
        let columns = view.columns(content_width(viewport.width));
        let total_rows = visible.div_ceil(columns);
        // One row above the estimate, the viewport plus two below — the
        // LIST_TOP estimate drifts within a pitch, the overscan hides it.
        let first = ((((self.scroll_y - LIST_TOP) / pitch).floor() as isize - 1).clamp(0, total_rows as isize - 1)) as usize;
        let viewport_rows = (viewport.height / pitch).ceil() as usize;
        let end = (first + viewport_rows + 3).min(total_rows);
        RowWindow {
            columns,
            total_rows,
            first,
            end,
        }
    }

    /// The pictures the list and the hero draw at `viewport`: the art of the rows the list builds, and the
    /// hero's banner. Instances with an uploaded banner draw that instead, so they need none.
    pub(crate) fn art_keys(&self, viewport: Size) -> Vec<ArtKey> {
        let visible = self.visible();
        let content = content_width(viewport.width);
        let mut keys = Vec::new();
        if !visible.is_empty() {
            let window = self.row_window(viewport, visible.len());
            let built = built_instances(&visible, window.first..window.end, window.columns);
            match self.view {
                LibraryView::Grid => {
                    let (width, height) = card_art_size(content, window.columns);
                    keys.extend(
                        built
                            .into_iter()
                            .filter(|instance| instance.art_path.is_none())
                            .map(|instance| ArtKey::new(Kind::Card, instance.biome, instance.art_seed, width, height)),
                    );
                }
                LibraryView::Compact => keys.extend(
                    built
                        .into_iter()
                        .filter(|instance| instance.art_path.is_none())
                        .map(|instance| ArtKey::new(Kind::Thumb, instance.biome, instance.art_seed, THUMB_SIZE, THUMB_SIZE)),
                ),
                LibraryView::Table => {}
            }
        }
        if let Some(hero) = self.hero_instance().filter(|hero| hero.art_path.is_none()) {
            keys.push(ArtKey::new(Kind::Hero, hero.biome, hero.art_seed, content, super::hero::HERO_HEIGHT));
        }
        keys
    }
}

/// The instances in rows `rows` of a list that puts `columns` instances to a row.
fn built_instances<'a>(visible: &[&'a Instance], rows: Range<usize>, columns: usize) -> Vec<&'a Instance> {
    let start = (rows.start * columns).min(visible.len());
    let end = (rows.end * columns).min(visible.len());
    visible[start..end].to_vec()
}

/// The rendered picture of `instance` at `width`×`height`, or `None` while it renders, or when the instance
/// has its own uploaded banner.
fn picture(art: &ArtCache, instance: &Instance, kind: Kind, width: f32, height: f32) -> Option<Handle> {
    if instance.art_path.is_some() {
        return None;
    }
    art.get(ArtKey::new(kind, instance.biome, instance.art_seed, width, height))
}

/// The messages one instance's card or row sends: its Play, open, hover and menu messages.
fn instance_messages(instance: &Instance) -> InstanceMessages<Message> {
    let id = instance.id.clone();
    InstanceMessages {
        play: Message::Play(id.clone()),
        open: Message::OpenInstance(id.clone()),
        enter: Message::Hover {
            instance: id.clone(),
            hovered: true,
        },
        exit: Message::Hover {
            instance: id.clone(),
            hovered: false,
        },
        menu: Box::new(move |action| Message::InstanceMenu {
            instance: id.clone(),
            action,
        }),
    }
}
