//! The Mods tab's bar: the text filter, the chips that narrow the list, Update all, and Add Mod. It stays at the top
//! while the list scrolls beneath it.

use aurora_leaf::Icon;
use iced::widget::{Text, container, row, text_input};
use iced::{Background, Border, Element, Fill, Padding, alignment};
use iced_color_extension::rgb;

use super::{Message, State};
use crate::app::components::TextButton;
use crate::app::components::button::Variant;
use crate::app::components::hint::hint;
use crate::app::components::icon_button::{self, IconButton};
use crate::app::components::input;
use crate::app::lang;
use crate::app::theme::{color, fonts};

impl State {
    /// The bar. `takes_mods` is whether the instance has a mod loader, which Add Mod needs.
    pub(super) fn toolbar(&self, takes_mods: bool) -> Element<'_, Message> {
        let add = TextButton::labeled_icon(Icon::lucide().plus(), lang::t("instance.mods.add"), Variant::Primary);
        // A locked instance takes no new mods until it is unlocked, so Add waits as it does for a vanilla instance.
        let can_add = takes_mods && !self.locked;
        let add = if can_add { add.on_press(Message::Add) } else { add.disabled(true) };
        let add_hint = if !takes_mods {
            lang::t("instance.mods.add_vanilla")
        } else if self.locked {
            lang::t("instance.mods.add_locked")
        } else {
            lang::t("instance.mods.add_hint")
        };
        let filter = text_input(lang::t("instance.mods.filter_placeholder"), &self.filter.query)
            .on_input(Message::Query)
            .style(input::style)
            .padding(Padding::new(0.0).left(14).right(14).top(10).bottom(10))
            .size(13)
            .width(Fill);
        let mut bar = row![filter].spacing(10).align_y(alignment::Vertical::Center);
        // The Disabled chip changes no mod, so a locked instance keeps it. It shows while a mod is disabled, and stays
        // while it is on, so the filter can always be turned off.
        let disabled = self.disabled_count();
        if disabled > 0 || self.filter.disabled_only {
            bar = bar.push(filter_chip(
                lang::t("instance.mods.disabled_filter"),
                self.filter.disabled_only,
                disabled,
                Message::Disabled(!self.filter.disabled_only),
            ));
        }
        // A locked instance cannot install the newer releases the chip counts, so the chip and Update all are left out
        // while it is locked.
        if !self.locked {
            let updates = self.update_count();
            // The Updates chip shows while a newer release exists, and stays while it is on, as the Disabled chip does.
            if updates > 0 || self.filter.updates_only {
                bar = bar.push(filter_chip(
                    lang::t("instance.mods.updates_filter"),
                    self.filter.updates_only,
                    updates,
                    Message::Updates(!self.filter.updates_only),
                ));
            }
            if updates > 0 {
                bar = bar.push(self.update_all());
            }
        }
        bar = bar.push(self.snapshot_button());
        bar.push(hint(add, add_hint)).into()
    }

    /// The button that saves every mod the instance holds. It waits while an Update all runs, and is off when the instance keeps
    /// no snapshots. Taking one only reads the mods, so a locked instance keeps it.
    fn snapshot_button(&self) -> Element<'_, Message> {
        let button = IconButton::new(Icon::lucide().camera())
            .size(36.0)
            .icon_size(18.0)
            .style(icon_button::ghost_button_style());
        let button = if self.snapshots_enabled && !self.updating_all {
            button.on_press(Message::Snapshot)
        } else {
            button.disabled(true)
        };
        hint(button, lang::t("instance.mods.snapshot_hint"))
    }

    /// The button that installs every newer release the Updates chip counts. It waits while the updates it started run.
    fn update_all(&self) -> Element<'_, Message> {
        let button = TextButton::labeled_icon(Icon::lucide().arrow_up(), lang::t("instance.mods.update_all"), Variant::Ghost);
        let button = if self.updating_all {
            button.disabled(true)
        } else {
            button.on_press(Message::UpdateAll)
        };
        hint(button, lang::t("instance.mods.update_all_hint"))
    }
}

/// A chip that narrows the list, with how many mods it would leave. It lights while it is on, as the Worlds page's
/// filter chips do. `press` is the message a press sends.
fn filter_chip(label: &'static str, active: bool, count: usize, press: Message) -> Element<'static, Message> {
    let (ink, weight) = if active {
        (color::text_headline(), fonts::Inter::SEMI_BOLD)
    } else {
        (color::text_muted(), fonts::Inter::MEDIUM)
    };
    let content = row![
        Text::new(label).size(12).font(weight).color(ink),
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
            .on_press(press),
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
