//! The Servers page's header: its title and the players online, the search field that also takes an address, and the
//! refresh and Add server buttons.

use aurora_leaf::Icon;
use iced::widget::{Row, Text, column, container, row, stack, text_input};
use iced::{Element, Fill, Length, Padding, alignment};

use super::{Message, State};
use crate::app::components::button::Variant;
use crate::app::components::hint::hint;
use crate::app::components::icon_button::{IconButton, ghost_button_style};
use crate::app::components::{TextButton, input};
use crate::app::lang;
use crate::app::theme::{color, fonts, utility};

/// The search field's width, as the Library's and the Worlds page's are.
const SEARCH_WIDTH: f32 = 320.0;

/// The header: the title and the summary of the servers that answered, and on the right the search, refresh and Add.
pub fn view(state: &State) -> Element<'_, Message> {
    let (online, players) = state.online_summary();
    let online = online.to_string();
    let players = players.to_string();
    let summary = lang::t_with("servers.summary", &[("online", online.as_str()), ("players", players.as_str())]);
    let title = column![
        Text::new(lang::t("servers.title"))
            .size(24)
            .font(fonts::Inter::EXTRA_BOLD)
            .color(color::text_headline()),
        Text::new(summary).size(12).font(fonts::Inter::REGULAR).color(color::text_muted()),
    ]
    .spacing(2);
    // The search and the buttons wrap beside the title, and keep the right edge, so a narrow window moves them to a second line.
    let actions: Vec<Element<'_, Message>> = vec![
        search_box(state.query()),
        hint(
            IconButton::new(Icon::lucide().refresh_cw())
                .size(36)
                .radius(12.0)
                .icon_size(16.0)
                .style(ghost_button_style())
                .on_press(Message::Round),
            lang::t("servers.refresh"),
        ),
        TextButton::content(
            row![
                utility::icon(Icon::lucide().plus(), 14.0, Some((color::primary_on(), color::primary_on()))),
                Text::new(lang::t("servers.add"))
                    .size(13)
                    .font(fonts::Inter::BOLD)
                    .color(color::primary_on())
                    .wrapping(iced::widget::text::Wrapping::None),
            ]
            .spacing(8)
            .align_y(alignment::Vertical::Center),
        )
        .variant(Variant::Primary)
        .padding(Padding::new(0.0).left(18).right(18).top(12).bottom(12))
        .on_press(Message::Add)
        .into(),
    ];
    let actions = container(
        Row::with_children(actions)
            .spacing(12)
            .align_y(alignment::Vertical::Center)
            .wrap()
            .vertical_spacing(12)
            .align_x(alignment::Horizontal::Right),
    )
    .width(Fill)
    .align_x(alignment::Horizontal::Right);
    row![title, actions].spacing(16).align_y(alignment::Vertical::Center).width(Fill).into()
}

/// The search field, with its icon on the left. Enter sends a submit, which opens the Add dialog on a valid address.
fn search_box(query: &str) -> Element<'_, Message> {
    stack![
        text_input(lang::t("servers.search_placeholder"), query)
            .on_input(Message::Query)
            .on_submit(Message::Submit)
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
