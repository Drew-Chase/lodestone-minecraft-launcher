//! The Library header row and the instance-list controls: search, filter
//! tabs and the view switcher.

use crate::app::components::{IconButton, TextButton};
use crate::app::theme::{color, fonts};
use aurora_leaf::Icon;
use iced::widget::{Space, Text, column, container, row, stack, text_input};
use iced::{Element, Fill, Length, Padding, alignment};

use super::{Library, LibraryFilter, LibraryView, Message, icon};

impl Library {
    /// The page header — title, stats, search and actions.
    pub(super) fn header(&self) -> Element<'_, Message> {
        let total_hours: u32 = self.instances.iter().map(|instance| instance.playtime_seconds / 3600).sum();
        row![
            column![
                Text::new("Library").size(24).font(fonts::Inter::EXTRA_BOLD).color(color::Text::HEADLINE),
                Text::new(format!(
                    "{} instance{} · {}h total playtime",
                    self.instances.len(),
                    if self.instances.len() == 1 { "" } else { "s" },
                    total_hours
                ))
                .size(12)
                .font(fonts::Inter::REGULAR)
                .color(color::Text::MUTED),
            ]
            .spacing(2),
            Space::new().width(Fill),
            self.search_box(),
            IconButton::new(Icon::lucide().refresh_cw())
                .width(44)
                .height(44)
                .icon_size(16.0)
                .on_press(Message::Reload),
            IconButton::new(Icon::lucide().bell())
                .width(44)
                .height(44)
                .icon_size(16.0)
                .on_press(Message::QuickAction("notifications")),
            TextButton::content(
                row![
                    icon(Icon::lucide().plus(), 14.0, color::Primary::ON, 1.0),
                    Text::new("New Instance").size(13).font(fonts::Inter::BOLD).color(color::Primary::ON),
                ]
                .spacing(8)
                .align_y(alignment::Vertical::Center)
            )
            .variant(crate::app::components::button::Variant::Primary)
            .padding(Padding::new(0.0).left(18).right(18).top(12).bottom(12))
            .on_press(Message::OpenNewInstance),
        ]
        .spacing(12)
        .align_y(alignment::Vertical::Center)
        .width(Fill)
        .into()
    }

    /// The search input with its leading icon.
    pub(super) fn search_box(&self) -> Element<'_, Message> {
        stack![
            text_input("Search instances...", &self.search)
                .on_input(Message::SearchChanged)
                .size(13)
                .padding(Padding::new(0.0).left(40).right(12).top(11).bottom(11))
                .width(Length::Fixed(260.0)),
            container(icon(Icon::lucide().search(), 15.0, color::Text::PLACEHOLDER, 1.0))
                .width(Length::Fixed(260.0))
                .height(Fill)
                .align_x(alignment::Horizontal::Left)
                .align_y(alignment::Vertical::Center)
                .padding(Padding::new(0.0).left(14)),
        ]
        .width(260)
        .into()
    }

    /// The filter tabs — Recent / Pinned / Modded / Vanilla.
    pub(super) fn filter_tabs(&self) -> Element<'_, Message> {
        row([
            LibraryFilter::Recent,
            LibraryFilter::Pinned,
            LibraryFilter::Modded,
            LibraryFilter::Vanilla,
        ]
        .into_iter()
        .map(|filter| {
            let label = filter.label();
            let content = if self.filter == filter {
                Text::new(label).size(12).font(fonts::Inter::SEMI_BOLD).color(color::Primary::DEFAULT)
            } else {
                Text::new(label).size(12).font(fonts::Inter::MEDIUM).color(color::Text::MUTED)
            };
            TextButton::content(content)
                .padding(Padding::new(0.0).left(14).right(14).top(8).bottom(8))
                .on_press(Message::FilterChanged(filter))
                .into()
        }))
        .spacing(4)
        .into()
    }

    /// The view switcher — Grid / Compact / Table.
    pub(super) fn view_switcher(&self) -> Element<'_, Message> {
        let entries = [
            (LibraryView::Grid, Icon::lucide().layout_grid()),
            (LibraryView::Compact, Icon::lucide().rows_3()),
            (LibraryView::Table, Icon::lucide().table()),
        ];
        row(entries.into_iter().map(|(view, icon_markup)| {
            let active = self.view == view;
            let (ink, weight) = if active {
                (color::Primary::DEFAULT, fonts::Inter::SEMI_BOLD)
            } else {
                (color::Text::MUTED, fonts::Inter::MEDIUM)
            };
            TextButton::content(
                row![
                    icon(icon_markup, 13.0, ink, 1.0),
                    Text::new(view.label()).size(12).font(weight).color(ink),
                ]
                .spacing(6)
                .align_y(alignment::Vertical::Center),
            )
            .padding(Padding::new(0.0).left(12).right(12).top(7).bottom(7))
            .on_press(Message::ViewChanged(view))
            .into()
        }))
        .spacing(2)
        .into()
    }
}
