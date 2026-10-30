//! The language picker: a [`Select`] that lists every catalogue under `res/lang` by its
//! `langname` and reports the code of the row chosen.
//!
//! The picker is self-contained. Its owner picks the message to send and the look, so one
//! control serves the login page's top bar and a settings row alike:
//!
//! ```text
//! LanguagePicker::new(login::Message::SelectLanguage).style(PickerStyle::Chip)  // login top bar
//! LanguagePicker::new(settings::Message::SelectLanguage)                       // settings row
//! ```
//!
//! The rows come from [`lang::languages`] and the selected row from [`lang::active_index`], so a
//! language chosen anywhere shows here at once. A new catalogue appears after a rebuild.

use aurora_leaf::Icon;
use iced::{Element, Length};

use super::select::Select;
use crate::app::lang;

/// The look of the picker's trigger.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PickerStyle {
    /// The dark chip with a globe, for the login page's top bar.
    Chip,
    /// The `.select` field, for a settings row.
    #[default]
    Field,
}

/// The chip's default width, wide enough for the longest language name in the catalogues.
const CHIP_WIDTH: f32 = 160.0;
/// The field's default width, the width the instance settings' selects use.
const FIELD_WIDTH: f32 = 200.0;
/// The default trigger height.
const TRIGGER_HEIGHT: f32 = 36.0;

/// One row of the picker: a catalogue's code and the name the row shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Row {
    code: &'static str,
    name: &'static str,
}

/// The rows for `catalogues`, in order. Each catalogue is a `(code, langname)` pair.
fn rows_of(catalogues: impl IntoIterator<Item = (&'static str, &'static str)>) -> Vec<Row> {
    catalogues.into_iter().map(|(code, name)| Row { code, name }).collect()
}

/// The code a chosen row reports.
fn code_of(rows: &[Row], index: usize) -> &'static str {
    rows[index].code
}

/// The language picker. Build it with [`LanguagePicker::new`], set its look or size, and convert
/// it into an element.
pub struct LanguagePicker<'a, Message> {
    on_select: Box<dyn Fn(&'static str) -> Message + 'a>,
    style: PickerStyle,
    width: Option<Length>,
    height: Option<Length>,
}

impl<'a, Message> LanguagePicker<'a, Message> {
    /// A picker that sends `on_select` the code of the language chosen.
    pub fn new(on_select: impl Fn(&'static str) -> Message + 'a) -> Self {
        Self {
            on_select: Box::new(on_select),
            style: PickerStyle::default(),
            width: None,
            height: None,
        }
    }

    /// Sets the look. The default is [`PickerStyle::Field`].
    pub fn style(mut self, style: PickerStyle) -> Self {
        self.style = style;
        self
    }

    /// Sets the width. Without it, the style's default width applies.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = Some(width.into());
        self
    }

    /// Sets the trigger's height. Without it, the trigger is 36px tall.
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = Some(height.into());
        self
    }
}

impl<'a, Message: 'static> From<LanguagePicker<'a, Message>> for Element<'a, Message> {
    fn from(picker: LanguagePicker<'a, Message>) -> Self {
        let rows = rows_of(lang::languages().iter().map(|language| (language.code, language.name.as_str())));
        let names: Vec<&'static str> = rows.iter().map(|row| row.name).collect();
        let on_select = picker.on_select;
        let (default_width, chip) = match picker.style {
            PickerStyle::Chip => (CHIP_WIDTH, true),
            PickerStyle::Field => (FIELD_WIDTH, false),
        };
        let select = Select::new(names, Some(lang::active_index()))
            .width(picker.width.unwrap_or(Length::Fixed(default_width)))
            .height(picker.height.unwrap_or(Length::Fixed(TRIGGER_HEIGHT)))
            .on_select(move |index| on_select(code_of(&rows, index)));
        if chip {
            select.chip().leading_icon(Icon::lucide().globe()).into()
        } else {
            select.into()
        }
    }
}

#[cfg(test)]
#[path = "../../../tests/app/components/language_picker.rs"]
mod tests;
