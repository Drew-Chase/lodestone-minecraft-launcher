//! The Export step: the output the pack is written as, and the folder it is written into. The export waits while the
//! folder is missing, or the pack would overwrite a pack or land inside the instance.

use iced::alignment::Horizontal;
use iced::widget::{Space, column, container, row, text};
use iced::{Alignment, Background, Border, Element, Fill, Padding};

use super::{Blocker, Kind, Message, State, steps};
use crate::app::components::checkbox::{self, State as Tick};
use crate::app::components::segmented::segmented;
use crate::app::components::stepper::stepper;
use crate::app::components::{TextButton, typography};
use crate::app::lang;
use crate::app::theme::{color, fonts};

/// The widest the Export step's column grows.
const COLUMN_WIDTH: f32 = 720.0;

/// The Export step, with its output, its destination and its buttons.
pub fn view(state: &State) -> Element<'_, Message> {
    let caption = state.folder.display().to_string();
    let target = state.target().map_or_else(|| "—".to_string(), |target| target.display().to_string());
    let body = column![
        typography::h2(lang::t("server_pack.export.title")),
        typography::small(lang::t("server_pack.export.subtitle")).color(color::text_muted()),
        section(lang::t("server_pack.export.output_label"), output(state)),
        section(lang::t("server_pack.export.destination_label"), destination(state)),
        section(lang::t("server_pack.export.server_label"), server_option(state)),
        section(lang::t("server_pack.export.written_to"), code_line(target)),
        blocker_note(state),
        Space::new().height(Fill),
        buttons(state),
    ]
    .spacing(18)
    .width(Fill)
    .height(Fill);
    column![
        stepper(&steps(), 1, Some(caption)),
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

/// A labelled group on the step: its eyebrow, then its content.
fn section<'a>(label: &'static str, content: Element<'a, Message>) -> Element<'a, Message> {
    column![typography::eyebrow(label).color(color::text_muted()), content]
        .spacing(8)
        .width(Fill)
        .into()
}

/// The two outputs, as a switch: a folder of files, or one zip.
fn output(state: &State) -> Element<'_, Message> {
    let options = [
        (Kind::Folder, lang::t("server_pack.export.kind.folder")),
        (Kind::Zip, lang::t("server_pack.export.kind.zip")),
    ];
    segmented(&options, state.kind, Message::Kind)
}

/// The chosen folder, in a well, with a button that picks another.
fn destination(state: &State) -> Element<'_, Message> {
    let shown = match &state.destination {
        Some(folder) => folder.display().to_string(),
        None => lang::t("server_pack.export.destination.none").to_string(),
    };
    let well = container(text(shown).size(12.5).font(fonts::JetBrainsMono::REGULAR).color(color::text_default()))
        .padding(Padding::new(0.0).left(14.0).right(14.0).top(10.0).bottom(10.0))
        .width(Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(color::surface_bg2())),
            border: Border {
                color: color::surface_line_strong(),
                width: 1.0,
                radius: 8.0.into(),
            },
            ..Default::default()
        });
    let change: Element<'_, Message> = TextButton::ghost(lang::t("server_pack.export.destination.change"))
        .on_press(Message::PickDestination)
        .into();
    row![well, change].spacing(12).align_y(Alignment::Center).width(Fill).into()
}

/// The optional server: a checkbox that installs it beside the mods, and, once that is ticked, the EULA checkbox the
/// install asks for. A Quilt server is not installed by the launcher yet, so its option only says so.
fn server_option(state: &State) -> Element<'_, Message> {
    let Some(setup) = state.setup.as_ref() else {
        return Space::new().into();
    };
    if setup.kind == lodestone_loaders::server::ServerKind::Quilt {
        return typography::small(lang::t("server_pack.export.server.quilt"))
            .color(color::text_muted())
            .into();
    }
    let install_box = if state.install_server { Tick::Checked } else { Tick::Unchecked };
    let install = row![
        checkbox::checkbox(install_box, Message::InstallServer(!state.install_server)),
        typography::body(lang::t("server_pack.export.server.install")).color(color::text_default()),
    ]
    .spacing(12)
    .align_y(Alignment::Center);
    let mut options = column![
        install,
        typography::small(lang::t("server_pack.export.server.note")).color(color::text_muted())
    ]
    .spacing(10);
    if state.install_server {
        let eula_box = if state.accept_eula { Tick::Checked } else { Tick::Unchecked };
        options = options.push(
            row![
                checkbox::checkbox(eula_box, Message::AcceptEula(!state.accept_eula)),
                typography::body(lang::t("server_pack.export.server.eula")).color(color::text_default()),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
        );
        let java_box = if state.include_java { Tick::Checked } else { Tick::Unchecked };
        options = options.push(
            row![
                checkbox::checkbox(java_box, Message::IncludeJava(!state.include_java)),
                typography::body(lang::t("server_pack.export.server.java")).color(color::text_default()),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
        );
    }
    options.width(Fill).into()
}

/// A line of monospace text: the path the pack is written to.
fn code_line<'a>(line: String) -> Element<'a, Message> {
    text(line)
        .size(12.5)
        .font(fonts::JetBrainsMono::REGULAR)
        .color(color::text_default())
        .into()
}

/// The reason the export waits, when something blocks it. The destination's own prompt is not a warning.
fn blocker_note(state: &State) -> Element<'_, Message> {
    let Some(blocker) = state.export_blocker() else {
        return Space::new().into();
    };
    let note = match blocker {
        Blocker::NoDestination => typography::body(lang::t("server_pack.export.none")).color(color::text_muted()),
        Blocker::Exists => {
            let name = state.pack_name();
            let message = lang::t_with("server_pack.export.exists", &[("NAME", name.as_str())]);
            typography::body(message).color(color::amber())
        }
        Blocker::Inside => typography::body(lang::t("server_pack.export.inside")).color(color::amber()),
    };
    note.into()
}

/// The footer buttons: back to the Review step, and the export, which waits for a usable destination.
fn buttons(state: &State) -> Element<'_, Message> {
    let blocked = state.export_blocker().is_some();
    let back: Element<'_, Message> = TextButton::ghost(lang::t("server_pack.back")).on_press(Message::Review).into();
    let export: Element<'_, Message> = TextButton::primary(lang::t("server_pack.export.submit"))
        .on_press(Message::Export)
        .disabled(blocked)
        .into();
    row![Space::new().width(Fill), back, export]
        .spacing(12)
        .align_y(Alignment::Center)
        .width(Fill)
        .into()
}
