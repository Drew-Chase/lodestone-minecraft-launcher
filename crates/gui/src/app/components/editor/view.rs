//! The editor's layout: the file's name with Save, Revert and Close, the text beside its line numbers, and a footer
//! that says where the caret is and how the file is stored.

use aurora_leaf::Icon;
use iced::widget::text::{LineHeight, Wrapping};
use iced::widget::{Space, Text, column, container, row, text_editor};
use iced::{Background, Border, Color, Element, Fill, Padding, Pixels, alignment};

use super::gutter::{self, Gutter};
use super::highlight::{self, Highlighter, Settings};
use super::{ADVANCE, Editor, LINE_HEIGHT, Message, PADDING, SCROLL_ID, TEXT_SIZE};
use crate::app::components::TextButton;
use crate::app::components::button::Variant;
use crate::app::components::hint::hint;
use crate::app::components::icon_button::{self, IconButton};
use crate::app::components::scrollbar;
use crate::app::components::size_track::SizeTrack;
use crate::app::lang;
use crate::app::theme::{color, fonts, utility};

/// The columns of room kept after the longest line, so the caret at its end is clear of the scrollbar.
const SLACK_COLUMNS: usize = 6;

impl Editor {
    /// The editor: its header, the text, and its footer. It fills the room it is given.
    pub fn view(&self) -> Element<'_, Message> {
        let mut layout = column![self.header()].spacing(10).width(Fill).height(Fill);
        if self.changed_on_disk {
            layout = layout.push(self.changed_notice());
        }
        layout.push(self.body()).push(self.footer()).into()
    }

    /// The file's name over the folder it sits in, a dot while it has unsaved changes, and the buttons.
    fn header(&self) -> Element<'_, Message> {
        let (folder, name) = self.path.rsplit_once('/').unwrap_or(("", &self.path));
        let mut title = row![
            Text::new(name)
                .size(14)
                .font(fonts::Inter::SEMI_BOLD)
                .color(color::text_headline())
                .wrapping(Wrapping::None),
        ]
        .spacing(8)
        .align_y(alignment::Vertical::Center);
        if self.is_dirty() {
            title = title.push(hint(dot(color::amber()), lang::t("editor.unsaved")));
        }
        let mut names = column![title].spacing(2);
        if !folder.is_empty() {
            names = names.push(
                Text::new(folder)
                    .size(11)
                    .font(fonts::JetBrainsMono::REGULAR)
                    .color(color::text_placeholder())
                    .wrapping(Wrapping::None),
            );
        }

        let save = TextButton::labeled_icon(Icon::lucide().save(), lang::t("editor.save"), Variant::Primary)
            .padding(Padding::new(0.0).left(14.0).right(14.0).top(8.0).bottom(8.0));
        let save = if self.is_dirty() && !self.read_only {
            save.on_press(Message::Save)
        } else {
            save.disabled(true)
        };
        let revert = TextButton::ghost(lang::t("editor.revert")).padding(Padding::new(0.0).left(14.0).right(14.0).top(8.0).bottom(8.0));
        let revert = if self.is_dirty() || self.changed_on_disk {
            revert.on_press(Message::Revert)
        } else {
            revert.disabled(true)
        };
        let close = IconButton::new(Icon::lucide().x())
            .size(32)
            .icon_size(14.0)
            .style(icon_button::ghost_button_style())
            .on_press(Message::Close);
        let mut bar = row![container(names).width(Fill).clip(true)]
            .spacing(8)
            .align_y(alignment::Vertical::Center);
        if !self.read_only {
            bar = bar.push(hint(save, lang::t("editor.save_hint"))).push(revert);
        }
        bar.push(hint(close, lang::t("common.close"))).into()
    }

    /// The line shown when the file changed on disk under unsaved changes: what happened, and a way to take the file's
    /// version.
    fn changed_notice(&self) -> Element<'_, Message> {
        container(
            row![
                utility::icon(Icon::lucide().triangle_exclamation_point(), 14.0, Some((color::amber(), color::amber()))),
                Text::new(lang::t("editor.changed_on_disk"))
                    .size(12)
                    .font(fonts::Inter::MEDIUM)
                    .color(color::text_default())
                    .width(Fill),
                TextButton::ghost(lang::t("editor.reload"))
                    .padding(Padding::new(0.0).left(12.0).right(12.0).top(6.0).bottom(6.0))
                    .on_press(Message::Revert),
            ]
            .spacing(10)
            .align_y(alignment::Vertical::Center),
        )
        .width(Fill)
        .padding(Padding::new(0.0).left(14).right(8).top(6).bottom(6))
        .style(|_| container::Style {
            background: Some(Background::Color(color::amber_tint())),
            border: Border {
                radius: 12.0.into(),
                ..Border::default()
            },
            ..container::Style::default()
        })
        .into()
    }

    /// The text beside its line numbers, in the dark well of a code block. The editor is laid out at the full size of
    /// its text, and the scroll around it moves both ways.
    fn body(&self) -> Element<'_, Message> {
        let view = self.view_size();
        let lines = self.content.line_count();
        let gutter_width = gutter::width_for(lines, ADVANCE);
        // The editor is at least as large as the scroll, so a click anywhere in the well reaches it.
        let text_width = PADDING * 2.0 + (self.longest + SLACK_COLUMNS) as f32 * ADVANCE;
        let editor = text_editor(&self.content)
            .on_action(Message::Action)
            .key_binding(|press| self.binding(press))
            .font(fonts::JetBrainsMono::REGULAR)
            .size(TEXT_SIZE)
            .line_height(LineHeight::Absolute(Pixels::from(LINE_HEIGHT)))
            .padding(PADDING)
            .wrapping(Wrapping::None)
            .width(text_width.max(view.width))
            // The editor adds its own padding to this height, so the padding is taken off first.
            .min_height((view.height - PADDING * 2.0).max(LINE_HEIGHT))
            .style(text_style)
            .highlight_with::<Highlighter>(
                Settings {
                    language: self.language,
                    revision: self.replaced,
                },
                highlight::format,
            );
        let scroll = scrollbar::scroll_both(editor)
            .id(SCROLL_ID)
            .on_scroll(|viewport| {
                let offset = viewport.absolute_offset();
                Message::Scrolled { x: offset.x, y: offset.y }
            })
            .width(Fill)
            .height(Fill);
        let numbers = Gutter::new(lines, self.content.cursor().position.line, self.scroll.1).metrics(PADDING, LINE_HEIGHT, TEXT_SIZE, ADVANCE);
        let well = row![
            container(numbers).width(gutter_width).height(Fill).style(|_| container::Style {
                border: Border {
                    width: 0.0,
                    ..Border::default()
                },
                background: Some(Background::Color(color::surface_bg1())),
                ..container::Style::default()
            }),
            SizeTrack::new(scroll, self.viewport, Message::Resized),
        ]
        .width(Fill)
        .height(Fill);
        container(well)
            .width(Fill)
            .height(Fill)
            .clip(true)
            .style(|_| container::Style {
                background: Some(Background::Color(color::surface_bg0())),
                border: Border {
                    radius: 12.0.into(),
                    width: 1.0,
                    color: color::surface_line(),
                },
                ..container::Style::default()
            })
            .into()
    }

    /// Where the caret is, then the language, the encoding and the line endings of the file.
    fn footer(&self) -> Element<'_, Message> {
        let caret = self.content.cursor().position;
        let column = self
            .content
            .line(caret.line)
            .map_or(0, |line| line.text[..caret.column.min(line.text.len())].chars().count());
        let position = lang::t_with(
            "editor.position",
            &[
                ("line", (caret.line + 1).to_string().as_str()),
                ("column", (column + 1).to_string().as_str()),
            ],
        );
        let mut footer = row![fact(position), Space::new().width(Fill)]
            .spacing(16)
            .align_y(alignment::Vertical::Center);
        if self.read_only {
            footer = footer.push(fact(lang::t("editor.read_only").to_string()));
        }
        if self.reencoded {
            footer = footer.push(fact(lang::t("editor.reencoded").to_string()));
        }
        footer
            .push(fact(self.language.label().to_string()))
            .push(fact(self.encoding.label().to_string()))
            .push(fact(if self.crlf { "CRLF" } else { "LF" }.to_string()))
            .padding(Padding::new(0.0).left(4).right(4))
            .into()
    }
}

/// One fact of the footer, in the small mono face.
fn fact<'a>(text: String) -> Element<'a, Message> {
    Text::new(text)
        .size(11)
        .font(fonts::JetBrainsMono::REGULAR)
        .color(color::text_muted())
        .wrapping(Wrapping::None)
        .into()
}

/// A small filled dot.
fn dot<'a>(ink: Color) -> Element<'a, Message> {
    container(Space::new().width(8.0).height(8.0))
        .style(move |_| container::Style {
            background: Some(Background::Color(ink)),
            border: Border {
                radius: 4.0.into(),
                ..Border::default()
            },
            ..container::Style::default()
        })
        .into()
}

/// The text's look. The well around it draws the fill and the border, so the editor itself draws neither.
fn text_style(_theme: &iced::Theme, _status: text_editor::Status) -> text_editor::Style {
    text_editor::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border::default(),
        placeholder: color::text_placeholder(),
        value: color::text_default(),
        selection: color::primary_tint(),
    }
}
