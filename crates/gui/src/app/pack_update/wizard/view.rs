//! The wizard's view: a modal over the page, with the body of its step and the buttons that move between the steps. The changes step
//! shows each change with the decision it takes, and a text conflict's hunks with the side each one keeps.

use aurora_leaf::Icon;
use iced::widget::{Space, Text, column, container, row};
use iced::{Element, Fill, Padding, alignment};

use super::{Message, PackWizard, Step};
use crate::app::components::modal::Modal;
use crate::app::components::toggle::Toggle;
use crate::app::components::{TextButton, panel};
use crate::app::install::update::merge::Side;
use crate::app::install::update::plan::{Change, Decision, Kind};
use crate::app::install::update::review::Hunk;
use crate::app::lang;
use crate::app::theme::{color, fonts};

/// The width of the wizard's modal.
const WIDTH: f32 = 760.0;

impl PackWizard {
    /// The wizard as a modal: its title, the line that names its step, the body of the step, and its buttons.
    pub fn view(&self) -> Element<'_, Message> {
        let body = match self.step() {
            Step::Summary => self.summary(),
            Step::Changes => self.changes(),
            Step::Confirm => self.confirm(),
        };
        Modal::new(lang::t("pack_update.wizard.title"), body)
            .subtitle(step_line(self.step()))
            .icon(Icon::lucide().refresh_cw(), color::primary(), color::primary_tint())
            .footer(self.footer())
            .on_close(Message::Close)
            .close_on_scrim(false)
            .width(WIDTH)
            .into()
    }

    /// The first step: the release the instance moves to, the changes it brings, the Minecraft and loader changes it makes, and the
    /// backup of the worlds when Minecraft changes.
    fn summary(&self) -> Element<'_, Message> {
        let request = &self.review().request;
        let previous = request
            .instance
            .source_version
            .clone()
            .unwrap_or_else(|| lang::t("pack_update.wizard.unknown").to_string());
        let (apply, keep) = self.counts();
        let mut lines = column![
            text_line(lang::t_with(
                "pack_update.wizard.from_to",
                &[("from", previous.as_str()), ("to", request.release.number.as_str())],
            )),
            text_line(lang::t_with(
                "pack_update.wizard.counts",
                &[("apply", apply.to_string().as_str()), ("keep", keep.to_string().as_str())],
            )),
        ]
        .spacing(10);
        let game_changes = request.mc_version != request.instance.mc_version;
        for line in self.version_lines() {
            lines = lines.push(line);
        }
        let both_sides = self.review().changes.iter().filter(|change| change.kind == Kind::Conflict).count();
        if both_sides > 0 {
            lines = lines.push(text_line(lang::t_with(
                "pack_update.wizard.both_sides",
                &[("count", both_sides.to_string().as_str())],
            )));
        }
        if game_changes {
            let backup = row![
                Text::new(lang::t("pack_update.wizard.backup"))
                    .size(14)
                    .font(fonts::Inter::SEMI_BOLD)
                    .color(color::text_headline()),
                Space::new().width(Fill),
                Toggle::new(self.backup_worlds()).on_toggle_with(Message::SetBackup),
            ]
            .spacing(12)
            .align_y(alignment::Vertical::Center);
            lines = lines.push(backup);
        }
        lines.into()
    }

    /// The second step: each change, with the buttons that choose its decision. A text conflict shows its hunks instead, and each
    /// hunk has the buttons that choose its side. The jars the user added are counted at the end, since nothing is done to them.
    fn changes(&self) -> Element<'_, Message> {
        let mut rows = column![].spacing(12).width(Fill);
        for change in self.review().changes.iter().filter(|change| change.kind != Kind::Yours) {
            rows = rows.push(self.change_row(change));
        }
        let yours = self.review().changes.iter().filter(|change| change.kind == Kind::Yours).count();
        if yours > 0 {
            rows = rows.push(text_line(lang::t_with(
                "pack_update.wizard.yours",
                &[("count", yours.to_string().as_str())],
            )));
        }
        rows.into()
    }

    /// The last step: the Minecraft and loader changes, the worlds' backup when Minecraft changes, the decisions in short, and the
    /// warning that the update writes the instance once it starts.
    fn confirm(&self) -> Element<'_, Message> {
        let request = &self.review().request;
        let (apply, keep) = self.counts();
        let mut lines = column![text_line(lang::t_with(
            "pack_update.wizard.counts",
            &[("apply", apply.to_string().as_str()), ("keep", keep.to_string().as_str())],
        ))]
        .spacing(12);
        for line in self.version_lines() {
            lines = lines.push(line);
        }
        if request.mc_version != request.instance.mc_version {
            let backup = if self.backup_worlds() {
                "pack_update.wizard.confirm_backup"
            } else {
                "pack_update.wizard.confirm_no_backup"
            };
            lines = lines.push(text_line(lang::t(backup)));
        }
        lines.push(warn_line(lang::t("pack_update.wizard.confirm"))).into()
    }

    /// The Minecraft and loader changes the update makes, one line each. A Minecraft change is a warning, since worlds saved by a
    /// newer version cannot be opened by an older one.
    fn version_lines(&self) -> Vec<Element<'_, Message>> {
        let request = &self.review().request;
        let mut lines = Vec::new();
        if request.mc_version != request.instance.mc_version {
            lines.push(warn_line(lang::t_with(
                "pack_update.wizard.game",
                &[("from", request.instance.mc_version.as_str()), ("to", request.mc_version.as_str())],
            )));
        }
        if request.loader != request.instance.loader {
            lines.push(text_line(lang::t_with(
                "pack_update.wizard.loader",
                &[("from", request.instance.loader.label()), ("to", request.loader.label())],
            )));
        }
        lines
    }

    /// One change: its kind and path, with the decisions it offers. A text conflict offers its hunks in place of decisions.
    fn change_row(&self, change: &Change) -> Element<'_, Message> {
        let decision = self.decisions().decide(change);
        let heading = row![
            chip(kind_label(&change.kind)),
            Text::new(change.path.clone())
                .size(13)
                .font(fonts::JetBrainsMono::REGULAR)
                .color(color::text_headline())
                .width(Fill),
        ]
        .spacing(10)
        .align_y(alignment::Vertical::Center);
        let mut body = column![heading].spacing(8);
        match (&change.kind, self.conflict_of(&change.path)) {
            (Kind::Conflict, Some(conflict)) => {
                for (index, hunk) in conflict.hunks.iter().enumerate() {
                    body = body.push(self.hunk(&change.path, index, hunk));
                }
            }
            _ => {
                if let Some(choices) = choices_of(&change.kind) {
                    let buttons = choices.iter().map(|(label, to)| {
                        let selected = *to == decision;
                        choice_button(
                            label,
                            Message::Decide {
                                path: change.path.clone(),
                                decision: *to,
                            },
                            selected,
                        )
                    });
                    body = body.push(row(buttons.collect::<Vec<_>>()).spacing(8));
                }
            }
        }
        panel::panel_with_padding(body, Padding::new(14.0))
    }

    /// One hunk of a text conflict: the user's lines against the release's, and the buttons that choose which side it keeps.
    fn hunk<'a>(&'a self, path: &str, index: usize, hunk: &'a Hunk) -> Element<'a, Message> {
        let side = self.decisions().side(path, index);
        let mut lines = column![].spacing(2).width(Fill);
        for line in &hunk.yours {
            lines = lines.push(diff_line("-", line, color::buttons::danger_foreground()));
        }
        for line in &hunk.theirs {
            lines = lines.push(diff_line("+", line, color::primary()));
        }
        let keep_mine = choice_button(
            lang::t("pack_update.wizard.keep_mine"),
            Message::Choose {
                path: path.to_string(),
                index,
                side: Side::Yours,
            },
            side == Side::Yours,
        );
        let use_pack = choice_button(
            lang::t("pack_update.wizard.use_pack"),
            Message::Choose {
                path: path.to_string(),
                index,
                side: Side::Theirs,
            },
            side == Side::Theirs,
        );
        column![lines, row![keep_mine, use_pack].spacing(8)].spacing(8).into()
    }

    /// The footer: Cancel on the left, and Back and the step's forward button on the right.
    fn footer(&self) -> Element<'_, Message> {
        let mut right = row![].spacing(8).align_y(alignment::Vertical::Center);
        if self.step() != Step::Summary {
            right = right.push(TextButton::ghost(lang::t("pack_update.wizard.back")).on_press(Message::Back));
        }
        right = match self.step() {
            Step::Confirm => right.push(TextButton::primary(lang::t("pack_update.wizard.start")).on_press(Message::Apply)),
            Step::Summary | Step::Changes => right.push(TextButton::primary(lang::t("pack_update.wizard.next")).on_press(Message::Next)),
        };
        row![
            TextButton::ghost(lang::t("pack_update.wizard.cancel")).on_press(Message::Close),
            Space::new().width(Fill),
            right,
        ]
        .align_y(alignment::Vertical::Center)
        .width(Fill)
        .into()
    }
}

/// The decisions a change of this kind offers, with their labels: the first is what a change takes on the release's side. A text
/// conflict offers none, since its hunks choose.
fn choices_of(kind: &Kind) -> Option<[(&'static str, Decision); 2]> {
    match kind {
        Kind::Add | Kind::Update | Kind::Supersede { .. } => Some([
            (lang::t("pack_update.wizard.apply"), Decision::Take),
            (lang::t("pack_update.wizard.skip"), Decision::Keep),
        ]),
        Kind::Restore => Some([
            (lang::t("pack_update.wizard.restore"), Decision::Take),
            (lang::t("pack_update.wizard.keep_deleted"), Decision::Keep),
        ]),
        Kind::Drop => Some([
            (lang::t("pack_update.wizard.remove"), Decision::Take),
            (lang::t("pack_update.wizard.keep"), Decision::Keep),
        ]),
        Kind::Mine | Kind::Conflict => Some([
            (lang::t("pack_update.wizard.use_pack"), Decision::Take),
            (lang::t("pack_update.wizard.keep_mine"), Decision::Keep),
        ]),
        Kind::Yours => None,
    }
}

/// The label of a kind of change, as the wizard names it.
fn kind_label(kind: &Kind) -> &'static str {
    match kind {
        Kind::Add => lang::t("pack_update.wizard.kind.add"),
        Kind::Restore => lang::t("pack_update.wizard.kind.restore"),
        Kind::Update => lang::t("pack_update.wizard.kind.update"),
        Kind::Drop => lang::t("pack_update.wizard.kind.drop"),
        Kind::Supersede { .. } => lang::t("pack_update.wizard.kind.supersede"),
        Kind::Conflict => lang::t("pack_update.wizard.kind.conflict"),
        Kind::Mine => lang::t("pack_update.wizard.kind.mine"),
        Kind::Yours => lang::t("pack_update.wizard.kind.yours"),
    }
}

/// The name of a step, with its place among the steps.
fn step_line(step: Step) -> String {
    let (number, label) = match step {
        Step::Summary => (1, lang::t("pack_update.wizard.steps.summary")),
        Step::Changes => (2, lang::t("pack_update.wizard.steps.changes")),
        Step::Confirm => (3, lang::t("pack_update.wizard.steps.confirm")),
    };
    lang::t_with(
        "pack_update.wizard.step_line",
        &[("number", number.to_string().as_str()), ("label", label)],
    )
}

/// A button that chooses a decision or a side, filled while it is the one chosen.
fn choice_button<'a>(label: &'static str, message: Message, selected: bool) -> Element<'a, Message> {
    if selected {
        TextButton::primary(label).on_press(message).into()
    } else {
        TextButton::ghost(label).on_press(message).into()
    }
}

/// A line of ordinary text in a step's body.
fn text_line<'a>(text: impl Into<String>) -> Element<'a, Message> {
    Text::new(text.into())
        .size(14)
        .font(fonts::Inter::REGULAR)
        .color(color::text_headline())
        .into()
}

/// A line that warns the user, in the tone of an alert.
fn warn_line<'a>(text: impl Into<String>) -> Element<'a, Message> {
    Text::new(text.into())
        .size(14)
        .font(fonts::Inter::SEMI_BOLD)
        .color(color::buttons::danger_foreground())
        .into()
}

/// A line of a diff: its marker, then the line itself in mono type.
fn diff_line<'a>(marker: &'static str, line: &str, ink: iced::Color) -> Element<'a, Message> {
    row![
        Text::new(marker).size(12).font(fonts::JetBrainsMono::SEMI_BOLD).color(ink),
        Text::new(line.trim_end_matches(['\r', '\n']).to_string())
            .size(12)
            .font(fonts::JetBrainsMono::REGULAR)
            .color(color::text_headline())
            .width(Fill),
    ]
    .spacing(8)
    .into()
}

/// A small rounded label, such as a kind of change.
fn chip<'a>(label: &'static str) -> Element<'a, Message> {
    container(Text::new(label).size(11).font(fonts::JetBrainsMono::REGULAR).color(color::text_muted()))
        .padding(Padding::new(0.0).left(8).right(8).top(3).bottom(3))
        .into()
}
