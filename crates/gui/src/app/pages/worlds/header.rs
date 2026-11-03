//! The Worlds page's header: its title and summary, the search, and the New world and Import world menus.

use aurora_leaf::Icon;
use iced::widget::{Space, Text, column, container, row, stack, text_input};
use iced::{Background, Border, Element, Fill, Length, Padding, alignment};
use iced_color_extension::rgb;

use super::{Menu, Message, State};
use crate::app::components::TextButton;
use crate::app::components::input;
use crate::app::components::panel::panel_with_padding;
use crate::app::components::popover::Popover;
use crate::app::lang;
use crate::app::theme::{color, fonts, utility};

/// The search field's width, per the reference.
const SEARCH_WIDTH: f32 = 320.0;

/// The header: the title and the summary on the left, and on the right the search and the two menus.
pub fn view(state: &State) -> Element<'_, Message> {
    let title = column![
        Text::new(lang::t("worlds.title"))
            .size(24)
            .font(fonts::Inter::EXTRA_BOLD)
            .color(color::text_headline()),
        Text::new(state.summary()).size(12).font(fonts::Inter::REGULAR).color(color::text_muted()),
    ]
    .spacing(2);
    row![
        title,
        Space::new().width(Fill),
        search_box(&state.query),
        menu_button(state, Menu::Import, lang::t("worlds.import"), Icon::lucide().upload(), false),
        menu_button(state, Menu::New, lang::t("worlds.new"), Icon::lucide().plus(), true),
    ]
    .spacing(12)
    .align_y(alignment::Vertical::Center)
    .width(Fill)
    .into()
}

/// The search field, with its glass on the left.
fn search_box(query: &str) -> Element<'_, Message> {
    stack![
        text_input(lang::t("worlds.search_placeholder"), query)
            .on_input(Message::Search)
            .style(input::style)
            .size(13)
            .padding(Padding::new(0.0).left(36.0).right(14.0).top(10.0).bottom(10.0))
            .width(Length::Fixed(SEARCH_WIDTH)),
        container(utility::icon(
            Icon::lucide().search(),
            14.0,
            Some((color::text_placeholder(), color::text_placeholder()))
        ))
        .width(Length::Fixed(SEARCH_WIDTH))
        .height(Fill)
        .align_y(alignment::Vertical::Center)
        .padding(Padding::new(0.0).left(14.0)),
    ]
    .width(SEARCH_WIDTH)
    .into()
}

/// A header button that opens a menu of the instances beside it. The menu's rows are built only while it is open.
fn menu_button<'a>(state: &'a State, menu: Menu, label: &'static str, glyph: &'static str, primary: bool) -> Element<'a, Message> {
    let open = state.menu == Some(menu);
    let content: Element<'_, Message> = if !open {
        Space::new().into()
    } else if state.instances.is_empty() {
        panel_with_padding(
            Text::new(lang::t("worlds.picker.none"))
                .size(12)
                .font(fonts::Inter::REGULAR)
                .color(color::text_muted()),
            Padding::new(12.0),
        )
    } else {
        let rows: Vec<Element<'_, Message>> = state
            .instances
            .iter()
            .map(|(id, name)| {
                TextButton::ghost(name.as_str())
                    .on_press(Message::Pick { menu, instance: id.clone() })
                    .into()
            })
            .collect();
        panel_with_padding(column(rows).spacing(2.0).width(220.0), Padding::new(6.0))
    };
    Popover::new(face(label, glyph, primary), content)
        .open(open)
        .on_toggle(Message::ToggleMenu(menu))
        .on_dismiss(Message::DismissMenu)
        .into()
}

/// The look of a header button, drawn as a plain box so the popover receives its press. The primary one is green.
fn face(label: &'static str, glyph: &'static str, primary: bool) -> Element<'static, Message> {
    let (ink, fill, line) = if primary {
        (color::primary_on(), color::primary(), color::primary())
    } else {
        (color::text_default(), rgb!(255, 255, 255, 4%), color::surface_line())
    };
    container(
        row![
            utility::icon(glyph, 14.0, Some((ink, ink))),
            Text::new(label).size(13).font(fonts::Inter::BOLD).color(ink),
        ]
        .spacing(8)
        .align_y(alignment::Vertical::Center),
    )
    .padding(Padding::new(0.0).left(18.0).right(18.0).top(12.0).bottom(12.0))
    .style(move |_| container::Style {
        background: Some(Background::Color(fill)),
        border: Border {
            radius: 12.0.into(),
            width: 1.0,
            color: line,
        },
        ..container::Style::default()
    })
    .into()
}
