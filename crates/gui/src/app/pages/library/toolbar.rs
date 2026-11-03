//! The Library header row and the instance-list controls: search, filter
//! tabs and the view switcher.

use crate::app::components::hint::hint;
use crate::app::components::icon_button::ghost_button_style;
use crate::app::components::{IconButton, TextButton, input};
use crate::app::lang;
use crate::app::notifications::{Notifications, bell};
use crate::app::theme::{color, fonts};
use crate::app::updater::Button;
use aurora_leaf::Icon;
use iced::widget::{Row, Space, Text, column, container, row, stack, text_input};
use iced::{Element, Fill, Length, Padding, alignment};

use super::{Library, LibraryFilter, LibraryView, Message, icon};

/// The search field's width — the reference's 240px search.
const SEARCH_WIDTH: f32 = 240.0;

/// The header's summary: the instance count and the total playtime.
fn summary(instances: usize, total_hours: u32) -> String {
    let count = lang::t_count("library.instances", instances);
    let hours = total_hours.to_string();
    lang::t_with("library.summary", &[("instances", count.as_str()), ("hours", hours.as_str())])
}

/// The update button: a download icon, hinted with what the update is doing. It is disabled while the update is
/// downloading or installing.
fn update_button(update: Button) -> Element<'static, Message> {
    hint(
        IconButton::new(Icon::lucide().download())
            .size(36)
            .radius(12.0)
            .icon_size(16.0)
            .style(ghost_button_style())
            .disabled(!update.enabled)
            .on_press(Message::UpdatePressed),
        update.tooltip,
    )
}

impl Library {
    /// The page header — title, stats, search and actions. The update button, when an update is showing, sits between
    /// the refresh button and the notifications bell.
    pub(super) fn header<'a>(&'a self, update: Option<Button>, notifications: &'a Notifications) -> Element<'a, Message> {
        let total_hours: u32 = self.instances.iter().map(|instance| instance.playtime_seconds / 3600).sum();
        let title = column![
            Text::new(lang::t("library.title"))
                .size(24)
                .font(fonts::Inter::EXTRA_BOLD)
                .color(color::text_headline()),
            Text::new(summary(self.instances.len(), total_hours))
                .size(12)
                .font(fonts::Inter::REGULAR)
                .color(color::text_muted()),
        ]
        .spacing(2);
        let mut children: Vec<Element<'_, Message>> = vec![title.into(), Space::new().width(Fill).into(), self.search_box()];
        children.push(
            IconButton::new(Icon::lucide().refresh_cw())
                .size(36)
                .radius(12.0)
                .icon_size(16.0)
                .style(ghost_button_style())
                .on_press(Message::Reload)
                .into(),
        );
        if let Some(update) = update {
            children.push(update_button(update));
        }
        children.push(bell(notifications).map(Message::Notifications));
        children.push(
            TextButton::content(
                row![
                    icon(Icon::lucide().plus(), 14.0, color::primary_on(), 1.0),
                    Text::new(lang::t("library.new_instance"))
                        .size(13)
                        .font(fonts::Inter::BOLD)
                        .color(color::primary_on()),
                ]
                .spacing(8)
                .align_y(alignment::Vertical::Center),
            )
            .variant(crate::app::components::button::Variant::Primary)
            .padding(Padding::new(0.0).left(18).right(18).top(12).bottom(12))
            .on_press(Message::OpenNewInstance)
            .into(),
        );
        Row::with_children(children)
            .spacing(16)
            .align_y(alignment::Vertical::Center)
            .width(Fill)
            .into()
    }

    /// The search input with its leading icon, in the `.input` field style.
    pub(super) fn search_box(&self) -> Element<'_, Message> {
        stack![
            text_input(lang::t("library.search_placeholder"), &self.search)
                .on_input(Message::SearchChanged)
                .style(input::style)
                .size(13)
                .padding(Padding::new(0.0).left(34).right(14).top(10).bottom(10))
                .width(Length::Fixed(SEARCH_WIDTH)),
            container(icon(Icon::lucide().search(), 14.0, color::text_placeholder(), 1.0))
                .width(Length::Fixed(SEARCH_WIDTH))
                .height(Fill)
                .align_x(alignment::Horizontal::Left)
                .align_y(alignment::Vertical::Center)
                .padding(Padding::new(0.0).left(14)),
        ]
        .width(SEARCH_WIDTH)
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
                Text::new(label).size(12).font(fonts::Inter::SEMI_BOLD).color(color::primary())
            } else {
                Text::new(label).size(12).font(fonts::Inter::MEDIUM).color(color::text_muted())
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
                (color::primary(), fonts::Inter::SEMI_BOLD)
            } else {
                (color::text_muted(), fonts::Inter::MEDIUM)
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
