//! The Done step: the pack's receipt, with what it holds, and the buttons that open it or leave the wizard.

use aurora_leaf::Icon;
use iced::alignment::Horizontal;
use iced::widget::{Space, column, container, row, text};
use iced::{Alignment, Color, Element, Fill, Padding};

use super::{Message, Receipt, State, steps};
use crate::app::components::stepper::stepper;
use crate::app::components::{TextButton, typography};
use crate::app::lang;
use crate::app::theme::{color, fonts, utility};

/// The widest the Done step's column grows.
const COLUMN_WIDTH: f32 = 720.0;

/// The Done step: the pack's path, the count of each kind of mod, and the buttons.
pub fn view(state: &State) -> Element<'_, Message> {
    let caption = state.folder.display().to_string();
    let Some(receipt) = state.receipt.as_ref() else {
        return container(typography::body(lang::t("server_pack.done.missing")).color(color::text_muted()))
            .padding(Padding::new(32.0))
            .into();
    };
    let body = column![
        typography::h2(lang::t("server_pack.done.title")),
        text(receipt.target.display().to_string())
            .size(12.5)
            .font(fonts::JetBrainsMono::REGULAR)
            .color(color::text_default()),
        stats(receipt),
        Space::new().height(Fill),
        buttons(),
    ]
    .spacing(18)
    .width(Fill)
    .height(Fill);
    column![
        stepper(&steps(), 2, Some(caption)),
        container(container(body).max_width(COLUMN_WIDTH).width(Fill).height(Fill))
            .width(Fill)
            .height(Fill)
            .align_x(Horizontal::Center)
            .padding(Padding::new(0.0).left(32.0).right(32.0).top(28.0).bottom(28.0)),
    ]
    .width(Fill)
    .height(Fill)
    .into()
}

/// The receipt's counts, one line each. A count of zero for a note that only some packs have is left out.
fn stats(receipt: &Receipt) -> Element<'_, Message> {
    let taken = receipt.taken.to_string();
    let left_out = receipt.left_out.to_string();
    let unsure = receipt.unsure.to_string();
    let chosen = receipt.chosen.to_string();
    let files = receipt.files.to_string();
    let mut lines = column![line(
        lang::t_with("server_pack.done.taken", &[("COUNT", taken.as_str())]),
        color::primary()
    )]
    .spacing(10);
    lines = lines.push(line(
        lang::t_with("server_pack.done.left_out", &[("COUNT", left_out.as_str())]),
        color::text_muted(),
    ));
    if receipt.unsure > 0 {
        lines = lines.push(line(
            lang::t_with("server_pack.done.unsure", &[("COUNT", unsure.as_str())]),
            color::amber(),
        ));
    }
    if receipt.chosen > 0 {
        lines = lines.push(line(
            lang::t_with("server_pack.done.chosen", &[("COUNT", chosen.as_str())]),
            color::text_muted(),
        ));
    }
    lines = lines.push(line(
        lang::t_with("server_pack.done.files", &[("COUNT", files.as_str())]),
        color::text_muted(),
    ));
    lines.into()
}

/// One line of the receipt, with a tick in the colour its count speaks for.
fn line<'a>(label: String, ink: Color) -> Element<'a, Message> {
    row![
        utility::icon(Icon::lucide().check(), 12.0, Some((ink, ink))),
        text(label).size(14.0).font(fonts::Inter::MEDIUM).color(color::text_default()),
    ]
    .spacing(10)
    .align_y(Alignment::Center)
    .into()
}

/// The buttons: open the pack in the file manager, or leave the wizard.
fn buttons<'a>() -> Element<'a, Message> {
    let open: Element<'a, Message> = TextButton::ghost(lang::t("server_pack.done.open")).on_press(Message::Reveal).into();
    let leave: Element<'a, Message> = TextButton::primary(lang::t("server_pack.done.close")).on_press(Message::Back).into();
    row![Space::new().width(Fill), open, leave]
        .spacing(12)
        .align_y(Alignment::Center)
        .width(Fill)
        .into()
}
