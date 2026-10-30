//! The "Your Instances" section and its virtualized list, which windows rows
//! against the page scroll.

use crate::app::components::instance_card::{compact_row, instance_card, table_header, table_row};
use crate::app::lang;
use crate::app::theme::{color, fonts};
use iced::widget::{Space, Text, column, container, row};
use iced::{Element, Fill, Length, Size, alignment};

use super::{LIST_TOP, Library, LibraryView, Message};

impl Library {
    /// The "Your Instances" section — title, filter tabs, view switcher,
    /// and the virtualized list (which continues the page scroll).
    pub(super) fn instances_section(&self, viewport: Size) -> Element<'_, Message> {
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
            self.instances_list(viewport),
        ]
        .spacing(16)
        .width(Fill)
        .into()
    }

    /// The virtualized instance list — plain rows inside the page scroll.
    /// Only the rows around the scroll offset are built; top/bottom
    /// spacers hold the page height stable so the scrollbar reflects the
    /// full list.
    fn instances_list(&self, viewport: Size) -> Element<'static, Message> {
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
        // The grid tiles across the page body — the window width minus the
        // 68px rail and the page's 28px side paddings.
        let columns = view.columns(viewport.width - 68.0 - 56.0);
        let total_rows = visible.len().div_ceil(columns);
        // One row above the estimate, the viewport plus two below — the
        // LIST_TOP estimate drifts within a pitch, the overscan hides it.
        let first = ((((self.scroll_y - LIST_TOP) / pitch).floor() as isize - 1).clamp(0, total_rows as isize - 1)) as usize;
        let viewport_rows = (viewport.height / pitch).ceil() as usize;
        let end = (first + viewport_rows + 3).min(total_rows);

        let mut content = column![Space::new().height(first as f32 * pitch)].spacing(gap);
        for row_index in first..end {
            let start = row_index * columns;
            let slice = &visible[start..(start + columns).min(visible.len())];
            let mut items: Vec<Element<'static, Message>> = Vec::with_capacity(columns);
            match view {
                LibraryView::Grid => {
                    for instance in slice {
                        items.push(instance_card(
                            instance,
                            Message::Play(instance.name.clone()),
                            Message::OpenInstance(instance.id.clone()),
                        ));
                    }
                }
                LibraryView::Compact => {
                    for instance in slice {
                        items.push(compact_row(
                            instance,
                            Message::Play(instance.name.clone()),
                            Message::OpenInstance(instance.id.clone()),
                        ));
                    }
                }
                LibraryView::Table => {
                    for (offset, instance) in slice.iter().enumerate() {
                        items.push(table_row(
                            start + offset,
                            instance,
                            Message::Play(instance.name.clone()),
                            Message::OpenInstance(instance.id.clone()),
                        ));
                    }
                }
            }
            // Pad short rows so Fill-width items keep a uniform size.
            for _ in slice.len()..columns {
                items.push(Space::new().width(Fill).height(item_height).into());
            }
            content = content.push(row(items).spacing(24).width(Fill));
        }
        let bottom = (total_rows - end) as f32 * pitch;
        content = content.push(Space::new().height(bottom));

        // The table keeps its header pinned above its rows.
        if view == LibraryView::Table {
            column![table_header(), content].width(Fill).into()
        } else {
            content.into()
        }
    }
}
